#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub udp_port: u16,
    pub udp_forward_port: u16,
    pub hide_other_pads: bool,
    pub log: bool,

    pub brake_start_zone: u8,
    pub brake_min: u8,
    pub brake_max: u8,
    pub throttle_start_zone: u8,
    pub throttle_min: u8,
    pub throttle_max: u8,
    pub lockup_vibration: bool,
    pub lockup_threshold: f32,
    pub lockup_amplitude: u8,
    pub lockup_frequency: u8,
    pub wheelspin_vibration: bool,
    pub wheelspin_threshold: f32,
    pub wheelspin_frequency: u8,
    pub rev_limiter: bool,
    pub rev_limiter_fraction: f32,
    pub rev_limiter_frequency: u8,
    pub menu_triggers: bool,

    pub master: f32,
    pub road: f32,
    pub impact: f32,
    pub impact_threshold: f32,
    pub engine: f32,
    pub shift: f32,
    pub shift_ms: f32,
    pub slip: f32,
    pub game_rumble: f32,

    pub rpm_lightbar: bool,
    pub color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            udp_port: 20777,
            udp_forward_port: 0,
            hide_other_pads: true,
            log: true,
            brake_start_zone: 1,
            brake_min: 2,
            brake_max: 8,
            throttle_start_zone: 2,
            throttle_min: 1,
            throttle_max: 4,
            lockup_vibration: true,
            lockup_threshold: 0.25,
            lockup_amplitude: 6,
            lockup_frequency: 35,
            wheelspin_vibration: true,
            wheelspin_threshold: 0.2,
            wheelspin_frequency: 55,
            rev_limiter: true,
            rev_limiter_fraction: 0.96,
            rev_limiter_frequency: 22,
            menu_triggers: false,
            master: 0.8,
            road: 0.6,
            impact: 1.0,
            impact_threshold: 0.8,
            engine: 0.15,
            shift: 0.8,
            shift_ms: 90.0,
            slip: 0.5,
            game_rumble: 0.5,
            rpm_lightbar: true,
            color: [0, 40, 255],
        }
    }
}

pub const DEFAULT_INI: &str = "\
; DiRT Rally 2.0 DualSense mod
; Trigger strengths: 0..8. Zones: 0..9 (0 = trigger fully released, 9 = fully pressed).
; Haptic strengths: 0.0..1.0. The file is reloaded automatically while the game runs.
; In game: hold PS + D-pad Up/Down to change intensity, PS + Options to toggle the mod.

[general]
enabled = true
udp_port = 20777
udp_forward_port = 0
hide_other_pads = true
log = true

[triggers]
brake_start_zone = 1
brake_min = 2
brake_max = 8
throttle_start_zone = 2
throttle_min = 1
throttle_max = 4
lockup_vibration = true
lockup_threshold = 0.25
lockup_amplitude = 6
lockup_frequency = 35
wheelspin_vibration = true
wheelspin_threshold = 0.2
wheelspin_frequency = 55
rev_limiter = true
rev_limiter_fraction = 0.96
rev_limiter_frequency = 22
menu_triggers = false

[haptics]
master = 0.8
road = 0.6
impact = 1.0
impact_threshold = 0.8
engine = 0.15
shift = 0.8
shift_ms = 90
slip = 0.5
game_rumble = 0.5

[lightbar]
rpm_lightbar = true
color = 0, 40, 255
";

fn parse_bool(v: &str) -> Option<bool> {
    match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_color(v: &str) -> Option<[u8; 3]> {
    let parts: Vec<&str> = v.split(|c| c == ',' || c == ' ').filter(|s| !s.is_empty()).collect();
    if parts.len() != 3 {
        return None;
    }
    let mut out = [0u8; 3];
    for (o, p) in out.iter_mut().zip(parts) {
        *o = p.parse().ok()?;
    }
    Some(out)
}

impl Config {
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut c = Config::default();
        let mut warnings = Vec::new();
        let mut section = String::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split([';', '#']).next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_ascii_lowercase();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                warnings.push(format!("line {}: expected key = value", n + 1));
                continue;
            };
            let key = format!("{}.{}", section, k.trim().to_ascii_lowercase());
            let v = v.trim();
            if !c.apply(&key, v) {
                warnings.push(format!("line {}: bad or unknown setting {key} = {v}", n + 1));
            }
        }
        c.clamp();
        (c, warnings)
    }

    fn apply(&mut self, key: &str, v: &str) -> bool {
        macro_rules! set {
            ($field:expr, $parser:expr) => {
                match $parser {
                    Some(x) => {
                        $field = x;
                        true
                    }
                    None => false,
                }
            };
        }
        let f32v = || v.parse::<f32>().ok().filter(|x| x.is_finite());
        let u8v = || v.parse::<u8>().ok();
        let u16v = || v.parse::<u16>().ok();
        let bv = || parse_bool(v);
        match key {
            "general.enabled" => set!(self.enabled, bv()),
            "general.udp_port" => set!(self.udp_port, u16v()),
            "general.udp_forward_port" => set!(self.udp_forward_port, u16v()),
            "general.hide_other_pads" => set!(self.hide_other_pads, bv()),
            "general.log" => set!(self.log, bv()),
            "triggers.brake_start_zone" => set!(self.brake_start_zone, u8v()),
            "triggers.brake_min" => set!(self.brake_min, u8v()),
            "triggers.brake_max" => set!(self.brake_max, u8v()),
            "triggers.throttle_start_zone" => set!(self.throttle_start_zone, u8v()),
            "triggers.throttle_min" => set!(self.throttle_min, u8v()),
            "triggers.throttle_max" => set!(self.throttle_max, u8v()),
            "triggers.lockup_vibration" => set!(self.lockup_vibration, bv()),
            "triggers.lockup_threshold" => set!(self.lockup_threshold, f32v()),
            "triggers.lockup_amplitude" => set!(self.lockup_amplitude, u8v()),
            "triggers.lockup_frequency" => set!(self.lockup_frequency, u8v()),
            "triggers.wheelspin_vibration" => set!(self.wheelspin_vibration, bv()),
            "triggers.wheelspin_threshold" => set!(self.wheelspin_threshold, f32v()),
            "triggers.wheelspin_frequency" => set!(self.wheelspin_frequency, u8v()),
            "triggers.rev_limiter" => set!(self.rev_limiter, bv()),
            "triggers.rev_limiter_fraction" => set!(self.rev_limiter_fraction, f32v()),
            "triggers.rev_limiter_frequency" => set!(self.rev_limiter_frequency, u8v()),
            "triggers.menu_triggers" => set!(self.menu_triggers, bv()),
            "haptics.master" => set!(self.master, f32v()),
            "haptics.road" => set!(self.road, f32v()),
            "haptics.impact" => set!(self.impact, f32v()),
            "haptics.impact_threshold" => set!(self.impact_threshold, f32v()),
            "haptics.engine" => set!(self.engine, f32v()),
            "haptics.shift" => set!(self.shift, f32v()),
            "haptics.shift_ms" => set!(self.shift_ms, f32v()),
            "haptics.slip" => set!(self.slip, f32v()),
            "haptics.game_rumble" => set!(self.game_rumble, f32v()),
            "lightbar.rpm_lightbar" => set!(self.rpm_lightbar, bv()),
            "lightbar.color" => set!(self.color, parse_color(v)),
            _ => false,
        }
    }

    fn clamp(&mut self) {
        self.brake_start_zone = self.brake_start_zone.min(9);
        self.throttle_start_zone = self.throttle_start_zone.min(9);
        for v in [
            &mut self.brake_min,
            &mut self.brake_max,
            &mut self.throttle_min,
            &mut self.throttle_max,
            &mut self.lockup_amplitude,
        ] {
            *v = (*v).min(8);
        }
        if self.brake_max < self.brake_min {
            self.brake_max = self.brake_min;
        }
        if self.throttle_max < self.throttle_min {
            self.throttle_max = self.throttle_min;
        }
        for v in [
            &mut self.master,
            &mut self.road,
            &mut self.impact,
            &mut self.engine,
            &mut self.shift,
            &mut self.slip,
            &mut self.game_rumble,
        ] {
            *v = v.clamp(0.0, 1.0);
        }
        self.lockup_threshold = self.lockup_threshold.clamp(0.02, 0.95);
        self.wheelspin_threshold = self.wheelspin_threshold.clamp(0.02, 5.0);
        self.rev_limiter_fraction = self.rev_limiter_fraction.clamp(0.5, 1.0);
        self.impact_threshold = self.impact_threshold.max(0.01);
        self.shift_ms = self.shift_ms.clamp(0.0, 1000.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ini_parses_to_defaults_without_warnings() {
        let (c, w) = Config::parse(DEFAULT_INI);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c, Config::default());
    }

    #[test]
    fn values_are_applied() {
        let (c, w) = Config::parse(
            "[general]\nenabled = off\nudp_port=20888\n[triggers]\nbrake_max = 5 ; comment\n[haptics]\nmaster = 0.3\n[lightbar]\ncolor = 255,0,10\n",
        );
        assert!(w.is_empty(), "{w:?}");
        assert!(!c.enabled);
        assert_eq!(c.udp_port, 20888);
        assert_eq!(c.brake_max, 5);
        assert_eq!(c.master, 0.3);
        assert_eq!(c.color, [255, 0, 10]);
    }

    #[test]
    fn bad_values_warn_and_keep_defaults() {
        let (c, w) = Config::parse("[triggers]\nbrake_max = lots\nnope = 1\n[haptics]\nmaster = NaN\n");
        assert_eq!(w.len(), 3);
        assert_eq!(c.brake_max, Config::default().brake_max);
        assert_eq!(c.master, Config::default().master);
    }

    #[test]
    fn values_are_clamped() {
        let (c, _) = Config::parse("[triggers]\nbrake_min = 6\nbrake_max = 200\nbrake_start_zone = 40\n[haptics]\nmaster = 7\nroad = -1\n");
        assert_eq!(c.brake_max, 8);
        assert_eq!(c.brake_min, 6);
        assert_eq!(c.brake_start_zone, 9);
        assert_eq!(c.master, 1.0);
        assert_eq!(c.road, 0.0);
        let (c, _) = Config::parse("[triggers]\nthrottle_min = 5\nthrottle_max = 2\n");
        assert_eq!(c.throttle_max, 5);
    }
}
