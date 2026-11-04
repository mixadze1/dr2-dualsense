use crate::config::Config;
use crate::dualsense::report::OutputState;
use crate::dualsense::trigger::TriggerEffect;
use crate::telemetry::{Telemetry, FL, FR};

pub const TELEMETRY_TIMEOUT: f32 = 0.5;
pub const PAUSE_AFTER: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrakeState {
    Idle,
    Resist,
    Lockup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThrottleState {
    Idle,
    Resist,
    Wheelspin,
    Limiter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Diagnostics {
    pub active: bool,
    pub paused: bool,
    pub brake: BrakeState,
    pub throttle: ThrottleState,
    pub road: f32,
    pub impact: f32,
    pub shift: bool,
    pub spin_ratio: f32,
    pub lock_ratio: f32,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Diagnostics {
            active: false,
            paused: false,
            brake: BrakeState::Idle,
            throttle: ThrottleState::Idle,
            road: 0.0,
            impact: 0.0,
            shift: false,
            spin_ratio: 0.0,
            lock_ratio: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Inputs<'a> {
    pub telemetry: Option<&'a Telemetry>,
    pub telemetry_age: f32,
    pub game_rumble: (u16, u16),
    pub dt: f32,
    pub intensity: f32,
    pub enabled: bool,
    pub clock: f32,
}

pub struct Engine {
    pub cfg: Config,
    prev: Option<Telemetry>,
    susp_smooth: [f32; 4],
    road: f32,
    impact: f32,
    shift_timer: f32,
    last_gear: Option<i32>,
    last_g: (f32, f32),
    frozen_for: f32,
    motors: (f32, f32),
    pub diag: Diagnostics,
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn scale_level(level: u8, power: f32) -> u8 {
    if level == 0 || power <= 0.0 {
        0
    } else {
        ((level as f32 * power).round() as u8).clamp(1, 8)
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
}

pub fn rpm_color(frac: f32, clock: f32) -> [u8; 3] {
    const GREEN: [u8; 3] = [0, 255, 0];
    const YELLOW: [u8; 3] = [255, 200, 0];
    const RED: [u8; 3] = [255, 0, 0];
    if frac >= 0.95 {
        if (clock * 8.0).fract() < 0.5 {
            RED
        } else {
            [0, 0, 0]
        }
    } else if frac < 0.6 {
        mix([0, 60, 0], GREEN, frac / 0.6)
    } else if frac < 0.8 {
        mix(GREEN, YELLOW, (frac - 0.6) / 0.2)
    } else {
        mix(YELLOW, RED, (frac - 0.8) / 0.15)
    }
}

impl Engine {
    pub fn new(cfg: Config) -> Self {
        Engine {
            cfg,
            prev: None,
            susp_smooth: [0.0; 4],
            road: 0.0,
            impact: 0.0,
            shift_timer: 0.0,
            last_gear: None,
            last_g: (0.0, 0.0),
            frozen_for: 0.0,
            motors: (0.0, 0.0),
            diag: Diagnostics::default(),
        }
    }

    pub fn reset(&mut self) {
        self.prev = None;
        self.susp_smooth = [0.0; 4];
        self.road = 0.0;
        self.impact = 0.0;
        self.shift_timer = 0.0;
        self.last_gear = None;
        self.frozen_for = 0.0;
        self.motors = (0.0, 0.0);
    }

    fn brake_resist(&self) -> TriggerEffect {
        let p = self.cfg.trigger_power;
        TriggerEffect::slope(self.cfg.brake_start_zone, 9, scale_level(self.cfg.brake_min, p), scale_level(self.cfg.brake_max, p))
    }

    fn throttle_resist(&self) -> TriggerEffect {
        let p = self.cfg.trigger_power;
        TriggerEffect::slope(self.cfg.throttle_start_zone, 9, scale_level(self.cfg.throttle_min, p), scale_level(self.cfg.throttle_max, p))
    }

    fn idle_output(&mut self, inp: &Inputs, paused: bool) -> OutputState {
        let keep_prev = if paused { self.prev } else { None };
        self.reset();
        self.prev = keep_prev;
        self.frozen_for = if paused { PAUSE_AFTER } else { 0.0 };
        self.diag = Diagnostics { paused, ..Diagnostics::default() };
        let (l, r) = if !inp.enabled {
            (inp.game_rumble.0 as f32 / 65535.0, inp.game_rumble.1 as f32 / 65535.0)
        } else if self.cfg.menu_rumble && !paused {
            let g = self.cfg.game_rumble * self.cfg.vibration_power * inp.intensity;
            (inp.game_rumble.0 as f32 / 65535.0 * g, inp.game_rumble.1 as f32 / 65535.0 * g)
        } else {
            (0.0, 0.0)
        };
        let menu = inp.enabled && self.cfg.menu_triggers;
        OutputState {
            left_trigger: if menu { self.brake_resist() } else { TriggerEffect::off() },
            right_trigger: if menu { self.throttle_resist() } else { TriggerEffect::off() },
            motor_left: to_u8(l),
            motor_right: to_u8(r),
            lightbar: Some(self.cfg.color),
            player_leds: None,
        }
    }

    pub fn update(&mut self, inp: &Inputs) -> OutputState {
        let telemetry = match inp.telemetry {
            Some(t) if inp.enabled && inp.telemetry_age < TELEMETRY_TIMEOUT => *t,
            _ => return self.idle_output(inp, false),
        };
        let dt = inp.dt.clamp(0.0, 0.1);

        if self.cfg.pause_detection {
            let frozen = self.prev.map_or(false, |p| p.time == telemetry.time && p.speed == telemetry.speed);
            if frozen {
                self.frozen_for += dt;
            } else {
                self.frozen_for = 0.0;
            }
            if self.frozen_for >= PAUSE_AFTER {
                return self.idle_output(inp, true);
            }
        }

        let cfg = self.cfg.clone();
        let t = &telemetry;
        let speed = t.speed.abs();
        let tp = cfg.trigger_power;

        let front_min = t.wheel_speed[FL].abs().min(t.wheel_speed[FR].abs());
        let lock_ratio = if speed > 3.0 { (1.0 - front_min / speed).clamp(0.0, 1.0) } else { 0.0 };
        let max_wheel = t.wheel_speed.iter().fold(0.0f32, |a, w| a.max(w.abs()));
        let spin_ratio = ((max_wheel - speed) / speed.max(2.0)).max(0.0);

        let lockup = cfg.lockup_vibration && t.brake > 0.05 && speed > 3.0 && lock_ratio > cfg.lockup_threshold;
        let lock_amp = scale_level(cfg.lockup_amplitude, tp);
        let (left_trigger, brake_state) = if lockup && lock_amp > 0 {
            (TriggerEffect::vibration(cfg.brake_start_zone, lock_amp, cfg.lockup_frequency.max(1)), BrakeState::Lockup)
        } else {
            let e = self.brake_resist();
            (e, if e.is_off() { BrakeState::Idle } else { BrakeState::Resist })
        };

        let rpm_frac = t.rpm_fraction();
        let limiter = cfg.rev_limiter
            && t.throttle > 0.5
            && t.max_rpm > 100.0
            && t.rpm >= t.max_rpm * cfg.rev_limiter_fraction;
        let spinning = cfg.wheelspin_vibration && t.throttle > 0.2 && spin_ratio > cfg.wheelspin_threshold && max_wheel > 2.0;
        let limiter_amp = scale_level(cfg.rev_limiter_amplitude, tp);
        let (right_trigger, throttle_state) = if limiter && limiter_amp > 0 {
            (TriggerEffect::vibration(cfg.throttle_start_zone, limiter_amp, cfg.rev_limiter_frequency.max(1)), ThrottleState::Limiter)
        } else if spinning && cfg.wheelspin_amplitude > 0 && tp > 0.0 {
            let over = ((spin_ratio - cfg.wheelspin_threshold) / 0.6).clamp(0.0, 1.0);
            let max_amp = cfg.wheelspin_amplitude as f32;
            let amp = scale_level((1.0 + over * (max_amp - 1.0)).round().max(1.0) as u8, tp);
            (TriggerEffect::vibration(cfg.throttle_start_zone, amp, cfg.wheelspin_frequency.max(1)), ThrottleState::Wheelspin)
        } else {
            let e = self.throttle_resist();
            (e, if e.is_off() { ThrottleState::Idle } else { ThrottleState::Resist })
        };

        let alpha = (dt / 0.05).clamp(0.0, 1.0);
        let mut hp_sum = 0.0;
        let mut peak = 0.0f32;
        for i in 0..4 {
            let v = t.susp_vel[i];
            self.susp_smooth[i] += (v - self.susp_smooth[i]) * alpha;
            hp_sum += (v - self.susp_smooth[i]).abs();
            peak = peak.max(v.abs());
        }
        let speed_factor = (speed / 8.0).clamp(0.0, 1.0);
        let road_target = (hp_sum / 4.0 * 2.5).clamp(0.0, 1.0).sqrt() * 0.8 * speed_factor;
        let road_alpha = (dt / 0.04).clamp(0.0, 1.0);
        self.road += (road_target - self.road) * road_alpha;

        let dg = ((t.g_lat - self.last_g.0).powi(2) + (t.g_lon - self.last_g.1).powi(2)).sqrt();
        self.last_g = (t.g_lat, t.g_lon);
        let mut hit = 0.0f32;
        if self.prev.is_some() {
            if peak > cfg.impact_threshold {
                hit = hit.max(((peak - cfg.impact_threshold) / cfg.impact_threshold).clamp(0.0, 1.0) * 0.6 + 0.4);
            }
            if dg > 1.5 {
                hit = hit.max(((dg - 1.5) / 3.0).clamp(0.0, 1.0) * 0.6 + 0.4);
            }
        }
        self.impact = (self.impact * (-dt / 0.12).exp()).max(hit);

        let gear = t.forward_gear();
        let shifted = matches!((self.last_gear, gear), (Some(a), Some(b)) if a != b);
        if gear.is_some() {
            self.last_gear = gear;
        }
        if shifted {
            self.shift_timer = cfg.shift_ms / 1000.0;
        } else {
            self.shift_timer = (self.shift_timer - dt).max(0.0);
        }
        let shift_active = self.shift_timer > 0.0;

        let engine = rpm_frac.map(|f| f * f * (0.3 + 0.7 * t.throttle)).unwrap_or(0.0) * cfg.engine;
        let slip_level = if spinning || lockup { spin_ratio.max(lock_ratio).clamp(0.0, 1.0) * cfg.slip } else { 0.0 };
        let game = cfg.game_rumble;
        let (game_l, game_r) = (inp.game_rumble.0 as f32 / 65535.0 * game, inp.game_rumble.1 as f32 / 65535.0 * game);
        let shift_level = if shift_active { cfg.shift } else { 0.0 };
        let left_target = [self.impact * cfg.impact, shift_level, game_l, slip_level * 0.6].into_iter().fold(0.0f32, f32::max);
        let right_target = [self.road * cfg.road, engine, slip_level, game_r, shift_level * 0.5].into_iter().fold(0.0f32, f32::max);

        let resting = cfg.quiet_when_stopped && speed < 0.5 && t.throttle < 0.05 && t.brake < 0.05;
        let (left_target, right_target) = if resting { (0.0, 0.0) } else { (left_target, right_target) };
        if resting {
            self.motors = (0.0, 0.0);
            self.impact = 0.0;
        }
        let release = if cfg.smoothing_ms > 0.0 { (-dt / (cfg.smoothing_ms / 1000.0)).exp() } else { 0.0 };
        self.motors.0 = left_target.max(self.motors.0 * release);
        self.motors.1 = right_target.max(self.motors.1 * release);

        let scale = cfg.vibration_power * inp.intensity;
        let lightbar = match (cfg.rpm_lightbar, rpm_frac) {
            (true, Some(f)) => rpm_color(f, inp.clock),
            _ => cfg.color,
        };

        self.prev = Some(telemetry);
        self.diag = Diagnostics {
            active: true,
            paused: false,
            brake: brake_state,
            throttle: throttle_state,
            road: self.road,
            impact: self.impact,
            shift: shift_active,
            spin_ratio,
            lock_ratio,
        };

        OutputState {
            left_trigger,
            right_trigger,
            motor_left: to_u8(self.motors.0 * scale),
            motor_right: to_u8(self.motors.1 * scale),
            lightbar: Some(lightbar),
            player_leds: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Preset;
    use crate::dualsense::trigger::{MODE_FEEDBACK, MODE_VIBRATION};

    fn full() -> Config {
        Config { vibration_power: 1.0, smoothing_ms: 0.0, ..Config::preset(Preset::Strong) }
    }

    fn cruising() -> Telemetry {
        Telemetry {
            speed: 20.0,
            wheel_speed: [20.0; 4],
            throttle: 0.5,
            gear: 3.0,
            rpm: 4000.0,
            max_rpm: 8000.0,
            idle_rpm: 1000.0,
            max_gears: 6.0,
            ..Default::default()
        }
    }

    struct Clock(f32);

    impl Clock {
        fn tick(&mut self, t: &Telemetry) -> Telemetry {
            self.0 += 0.01;
            Telemetry { time: self.0, ..*t }
        }
    }

    fn inputs(t: &Telemetry) -> Inputs<'_> {
        Inputs { telemetry: Some(t), telemetry_age: 0.0, game_rumble: (0, 0), dt: 0.01, intensity: 1.0, enabled: true, clock: 0.0 }
    }

    fn run(engine: &mut Engine, clock: &mut Clock, t: &Telemetry, steps: usize) -> OutputState {
        let mut out = OutputState::neutral();
        for _ in 0..steps {
            let p = clock.tick(t);
            out = engine.update(&inputs(&p));
        }
        out
    }

    #[test]
    fn cruising_gives_resistance_on_both_triggers() {
        let mut e = Engine::new(Config::default());
        let out = run(&mut e, &mut Clock(0.0), &cruising(), 5);
        assert_eq!(out.left_trigger.mode(), MODE_FEEDBACK);
        assert_eq!(out.right_trigger.mode(), MODE_FEEDBACK);
        assert_eq!(e.diag.brake, BrakeState::Resist);
        assert_eq!(e.diag.throttle, ThrottleState::Resist);
        assert!(e.diag.active);
        assert_eq!(out.motor_left, 0);
    }

    #[test]
    fn brake_resistance_matches_config_slope() {
        let cfg = Config::default();
        let mut e = Engine::new(cfg.clone());
        let out = run(&mut e, &mut Clock(0.0), &cruising(), 1);
        assert_eq!(out.left_trigger, TriggerEffect::slope(cfg.brake_start_zone, 9, cfg.brake_min, cfg.brake_max));
    }

    #[test]
    fn trigger_power_scales_resistance() {
        let half = Config { trigger_power: 0.5, ..Config::preset(Preset::Strong) };
        let mut e = Engine::new(half.clone());
        let out = run(&mut e, &mut Clock(0.0), &cruising(), 1);
        assert_eq!(out.left_trigger, TriggerEffect::slope(half.brake_start_zone, 9, 1, 4));
        let off = Config { trigger_power: 0.0, ..Config::default() };
        let mut e = Engine::new(off);
        let out = run(&mut e, &mut Clock(0.0), &cruising(), 1);
        assert!(out.left_trigger.is_off() && out.right_trigger.is_off());
    }

    #[test]
    fn scale_level_keeps_effects_alive() {
        assert_eq!(scale_level(8, 1.0), 8);
        assert_eq!(scale_level(8, 0.5), 4);
        assert_eq!(scale_level(1, 0.1), 1);
        assert_eq!(scale_level(0, 1.0), 0);
        assert_eq!(scale_level(5, 0.0), 0);
    }

    #[test]
    fn front_lockup_vibrates_brake_trigger() {
        let mut e = Engine::new(full());
        let mut t = cruising();
        t.brake = 1.0;
        t.throttle = 0.0;
        t.wheel_speed = [15.0, 15.0, 2.0, 2.0];
        let out = run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(out.left_trigger.mode(), MODE_VIBRATION);
        assert_eq!(e.diag.brake, BrakeState::Lockup);
        assert!(out.motor_left > 0 || out.motor_right > 0);
    }

    #[test]
    fn no_lockup_without_brake() {
        let mut e = Engine::new(Config::default());
        let mut t = cruising();
        t.wheel_speed = [15.0, 15.0, 2.0, 2.0];
        run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(e.diag.brake, BrakeState::Resist);
    }

    #[test]
    fn no_lockup_when_stopped() {
        let mut e = Engine::new(Config::default());
        let mut t = cruising();
        t.speed = 1.0;
        t.brake = 1.0;
        t.wheel_speed = [0.0; 4];
        run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(e.diag.brake, BrakeState::Resist);
    }

    #[test]
    fn wheelspin_vibrates_throttle_trigger() {
        let cfg = Config::default();
        let mut e = Engine::new(cfg.clone());
        let mut t = cruising();
        t.speed = 5.0;
        t.throttle = 1.0;
        t.wheel_speed = [12.0, 12.0, 5.0, 5.0];
        t.rpm = 5000.0;
        let out = run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(out.right_trigger.mode(), MODE_VIBRATION);
        assert_eq!(e.diag.throttle, ThrottleState::Wheelspin);
        assert_eq!(out.right_trigger.0[9], cfg.wheelspin_frequency);
        let amp = (u32::from_le_bytes([out.right_trigger.0[3], out.right_trigger.0[4], out.right_trigger.0[5], out.right_trigger.0[6]]) >> (3 * 9)) & 7;
        assert!(amp + 1 <= cfg.wheelspin_amplitude as u32);
    }

    #[test]
    fn rev_limiter_overrides_wheelspin() {
        let mut e = Engine::new(Config::default());
        let mut t = cruising();
        t.throttle = 1.0;
        t.rpm = 7900.0;
        t.wheel_speed = [40.0, 40.0, 20.0, 20.0];
        let out = run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(e.diag.throttle, ThrottleState::Limiter);
        assert_eq!(out.right_trigger.0[9], Config::default().rev_limiter_frequency);
    }

    #[test]
    fn rev_limiter_needs_throttle() {
        let mut e = Engine::new(Config::default());
        let mut t = cruising();
        t.throttle = 0.0;
        t.rpm = 7900.0;
        run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(e.diag.throttle, ThrottleState::Resist);
    }

    #[test]
    fn gear_change_kicks_left_motor_then_fades() {
        let mut e = Engine::new(full());
        let mut c = Clock(0.0);
        let t = cruising();
        run(&mut e, &mut c, &t, 3);
        let mut t2 = t;
        t2.gear = 4.0;
        let out = run(&mut e, &mut c, &t2, 1);
        assert!(e.diag.shift);
        assert!(out.motor_left > 100, "{}", out.motor_left);
        let out = run(&mut e, &mut c, &t2, 20);
        assert!(!e.diag.shift);
        assert_eq!(out.motor_left, 0);
    }

    #[test]
    fn shifting_through_neutral_counts_once() {
        let mut e = Engine::new(Config::default());
        let mut c = Clock(0.0);
        let mut t = cruising();
        run(&mut e, &mut c, &t, 2);
        t.gear = 0.0;
        run(&mut e, &mut c, &t, 1);
        assert!(!e.diag.shift);
        t.gear = 3.0;
        run(&mut e, &mut c, &t, 1);
        assert!(!e.diag.shift);
        t.gear = 2.0;
        run(&mut e, &mut c, &t, 1);
        assert!(e.diag.shift);
    }

    fn rough(i: usize, speed: f32) -> Telemetry {
        let mut r = cruising();
        r.speed = speed;
        r.wheel_speed = [speed; 4];
        let s = if i % 2 == 0 { 0.4 } else { -0.4 };
        r.susp_vel = [s, -s, s, -s];
        r
    }

    #[test]
    fn rough_road_drives_right_motor() {
        let mut e = Engine::new(full());
        let mut c = Clock(0.0);
        let mut smooth_max = 0;
        for _ in 0..50 {
            smooth_max = smooth_max.max(run(&mut e, &mut c, &cruising(), 1).motor_right);
        }
        let mut rough_max = 0;
        for i in 0..100 {
            rough_max = rough_max.max(run(&mut e, &mut c, &rough(i, 20.0), 1).motor_right);
        }
        assert!(rough_max > smooth_max + 40, "smooth {smooth_max} rough {rough_max}");
    }

    #[test]
    fn rough_road_is_silent_when_stationary() {
        let mut e = Engine::new(Config { engine: 0.0, ..full() });
        let mut c = Clock(0.0);
        let mut max = 0;
        for i in 0..100 {
            max = max.max(run(&mut e, &mut c, &rough(i, 0.0), 1).motor_right);
        }
        assert_eq!(max, 0);
    }

    #[test]
    fn default_preset_is_much_gentler_than_strong() {
        let measure = |cfg: Config| {
            let mut e = Engine::new(cfg);
            let mut c = Clock(0.0);
            let mut max = 0u8;
            for i in 0..200 {
                max = max.max(run(&mut e, &mut c, &rough(i, 25.0), 1).motor_right);
            }
            max
        };
        let medium = measure(Config::default());
        let strong = measure(Config::preset(Preset::Strong));
        let soft = measure(Config::preset(Preset::Soft));
        assert!(soft < medium && medium < strong, "soft {soft} medium {medium} strong {strong}");
        assert!(medium < 120, "medium road too strong: {medium}");
    }

    #[test]
    fn standing_still_without_pedals_is_silent() {
        let mut e = Engine::new(Config { game_rumble: 1.0, engine: 1.0, ..full() });
        let mut c = Clock(0.0);
        let mut t = cruising();
        t.speed = 0.0;
        t.wheel_speed = [0.0; 4];
        t.throttle = 0.0;
        t.rpm = 900.0;
        let mut max = (0u8, 0u8);
        for i in 0..50 {
            let mut p = c.tick(&t);
            let s = if i % 2 == 0 { 2.0 } else { -2.0 };
            p.susp_vel = [s; 4];
            p.gear = if i % 10 < 5 { 1.0 } else { 2.0 };
            let mut inp = inputs(&p);
            inp.game_rumble = (65535, 65535);
            let out = e.update(&inp);
            max = (max.0.max(out.motor_left), max.1.max(out.motor_right));
        }
        assert_eq!(max, (0, 0));
        assert_eq!(e.diag.brake, BrakeState::Resist);
    }

    #[test]
    fn revving_while_stopped_still_vibrates() {
        let mut e = Engine::new(full());
        let mut t = cruising();
        t.speed = 0.0;
        t.wheel_speed = [0.0; 4];
        t.throttle = 1.0;
        t.rpm = 7900.0;
        let out = run(&mut e, &mut Clock(0.0), &t, 5);
        assert!(out.motor_right > 0);
        assert_eq!(e.diag.throttle, ThrottleState::Limiter);
    }

    #[test]
    fn quiet_when_stopped_can_be_disabled() {
        let mut e = Engine::new(Config { quiet_when_stopped: false, game_rumble: 1.0, ..full() });
        let mut t = cruising();
        t.speed = 0.0;
        t.wheel_speed = [0.0; 4];
        t.throttle = 0.0;
        let p = Telemetry { time: 1.0, ..t };
        let mut inp = inputs(&p);
        inp.game_rumble = (65535, 0);
        assert!(e.update(&inp).motor_left > 0);
    }

    #[test]
    fn big_landing_triggers_impact() {
        let mut e = Engine::new(full());
        let mut c = Clock(0.0);
        let t = cruising();
        run(&mut e, &mut c, &t, 3);
        let mut hit = t;
        hit.susp_vel = [2.5; 4];
        let out = run(&mut e, &mut c, &hit, 1);
        assert!(out.motor_left > 150, "{}", out.motor_left);
        let out = run(&mut e, &mut c, &t, 100);
        assert!(out.motor_left < 5);
    }

    #[test]
    fn g_spike_triggers_impact() {
        let mut e = Engine::new(full());
        let mut c = Clock(0.0);
        let t = cruising();
        run(&mut e, &mut c, &t, 3);
        let mut crash = t;
        crash.g_lon = -6.0;
        let out = run(&mut e, &mut c, &crash, 1);
        assert!(out.motor_left > 100);
    }

    #[test]
    fn smoothing_releases_gradually() {
        let mut e = Engine::new(Config { vibration_power: 1.0, smoothing_ms: 60.0, ..Config::preset(Preset::Strong) });
        let mut c = Clock(0.0);
        let t = cruising();
        run(&mut e, &mut c, &t, 3);
        let mut hit = t;
        hit.susp_vel = [2.5; 4];
        let peak = run(&mut e, &mut c, &hit, 1).motor_left;
        let next = run(&mut e, &mut c, &t, 1).motor_left;
        assert!(next > 0 && next < peak, "peak {peak} next {next}");
    }

    #[test]
    fn frozen_telemetry_is_treated_as_pause() {
        let mut e = Engine::new(Config { game_rumble: 1.0, ..full() });
        let mut c = Clock(0.0);
        let mut t = cruising();
        t.susp_vel = [0.4, -0.4, 0.4, -0.4];
        run(&mut e, &mut c, &t, 10);
        let frozen = c.tick(&t);
        let mut out = OutputState::neutral();
        for _ in 0..40 {
            let mut i = inputs(&frozen);
            i.game_rumble = (65535, 65535);
            out = e.update(&i);
        }
        assert!(e.diag.paused);
        assert!(!e.diag.active);
        assert!(out.left_trigger.is_off() && out.right_trigger.is_off());
        assert_eq!((out.motor_left, out.motor_right), (0, 0));
        let out = run(&mut e, &mut c, &cruising(), 1);
        assert!(!e.diag.paused);
        assert_eq!(out.left_trigger.mode(), MODE_FEEDBACK);
    }

    #[test]
    fn pause_detection_can_be_disabled() {
        let mut e = Engine::new(Config { pause_detection: false, ..Config::default() });
        let t = cruising();
        for _ in 0..60 {
            e.update(&inputs(&t));
        }
        assert!(e.diag.active);
    }

    #[test]
    fn short_repeats_are_not_a_pause() {
        let mut e = Engine::new(Config::default());
        let mut c = Clock(0.0);
        for _ in 0..20 {
            let p = c.tick(&cruising());
            e.update(&inputs(&p));
            e.update(&inputs(&p));
        }
        assert!(e.diag.active && !e.diag.paused);
    }

    #[test]
    fn menus_are_silent_by_default_even_if_game_rumbles() {
        let mut e = Engine::new(Config::default());
        let t = cruising();
        let mut i = inputs(&t);
        i.telemetry_age = 2.0;
        i.game_rumble = (65535, 32768);
        let out = e.update(&i);
        assert!(out.left_trigger.is_off() && out.right_trigger.is_off());
        assert!(!e.diag.active);
        assert_eq!((out.motor_left, out.motor_right), (0, 0));
    }

    #[test]
    fn menu_rumble_option_passes_game_rumble() {
        let cfg = Config { menu_rumble: true, ..Config::default() };
        let mut e = Engine::new(cfg.clone());
        let i = Inputs { telemetry: None, telemetry_age: 99.0, game_rumble: (65535, 0), dt: 0.01, intensity: 1.0, enabled: true, clock: 0.0 };
        let out = e.update(&i);
        assert_eq!(out.motor_left, to_u8(cfg.game_rumble * cfg.vibration_power));
    }

    #[test]
    fn menu_triggers_option_keeps_resistance_in_menus() {
        let mut e = Engine::new(Config { menu_triggers: true, ..Config::default() });
        let i = Inputs { telemetry: None, telemetry_age: 99.0, game_rumble: (0, 0), dt: 0.01, intensity: 1.0, enabled: true, clock: 0.0 };
        let out = e.update(&i);
        assert_eq!(out.left_trigger.mode(), MODE_FEEDBACK);
    }

    #[test]
    fn disabled_mod_is_plain_controller() {
        let mut e = Engine::new(Config::default());
        let t = cruising();
        let mut i = inputs(&t);
        i.enabled = false;
        i.game_rumble = (65535, 0);
        let out = e.update(&i);
        assert!(out.left_trigger.is_off() && out.right_trigger.is_off());
        assert_eq!(out.motor_left, 255);
        assert_eq!(out.motor_right, 0);
    }

    #[test]
    fn intensity_and_vibration_power_scale_haptics() {
        let t = cruising();
        let mut hit = t;
        hit.susp_vel = [2.5; 4];
        let level = |cfg: Config, intensity: f32| {
            let mut e = Engine::new(cfg);
            let mut c = Clock(0.0);
            for _ in 0..3 {
                let p = c.tick(&t);
                e.update(&Inputs { intensity, ..inputs(&p) });
            }
            let p = c.tick(&hit);
            e.update(&Inputs { intensity, ..inputs(&p) }).motor_left
        };
        assert!(level(full(), 1.0) > level(full(), 0.5));
        assert_eq!(level(full(), 0.0), 0);
        assert!(level(Config { vibration_power: 0.3, ..full() }, 1.0) < level(full(), 1.0));
    }

    #[test]
    fn rpm_lightbar_goes_green_to_red_and_flashes() {
        assert_eq!(rpm_color(0.6, 0.0), [0, 255, 0]);
        assert!(rpm_color(0.949, 0.0)[1] < 30);
        assert_eq!(rpm_color(0.99, 0.0), [255, 0, 0]);
        assert_eq!(rpm_color(0.99, 0.07), [0, 0, 0]);
    }

    #[test]
    fn lightbar_uses_static_color_without_rpm() {
        let mut e = Engine::new(Config::default());
        let mut t = cruising();
        t.max_rpm = 0.0;
        let out = e.update(&inputs(&t));
        assert_eq!(out.lightbar, Some(Config::default().color));
    }

    #[test]
    fn disabled_features_fall_back_to_resistance() {
        let cfg = Config { lockup_vibration: false, wheelspin_vibration: false, rev_limiter: false, ..Config::default() };
        let mut e = Engine::new(cfg);
        let mut t = cruising();
        t.brake = 1.0;
        t.throttle = 1.0;
        t.rpm = 7999.0;
        t.wheel_speed = [40.0, 40.0, 1.0, 1.0];
        run(&mut e, &mut Clock(0.0), &t, 3);
        assert_eq!(e.diag.brake, BrakeState::Resist);
        assert_eq!(e.diag.throttle, ThrottleState::Resist);
    }
}
