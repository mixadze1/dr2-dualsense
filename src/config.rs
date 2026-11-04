#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Soft,
    Medium,
    Strong,
}

impl Preset {
    pub fn parse(v: &str) -> Option<Preset> {
        match v.trim().to_ascii_lowercase().as_str() {
            "soft" | "weak" | "light" | "slabo" => Some(Preset::Soft),
            "medium" | "normal" | "default" => Some(Preset::Medium),
            "strong" | "hard" | "max" => Some(Preset::Strong),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub preset: Preset,
    pub enabled: bool,
    pub udp_port: u16,
    pub udp_forward_port: u16,
    pub hide_other_pads: bool,
    pub log: bool,

    pub trigger_power: f32,
    pub vibration_power: f32,

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
    pub wheelspin_amplitude: u8,
    pub wheelspin_frequency: u8,
    pub rev_limiter: bool,
    pub rev_limiter_fraction: f32,
    pub rev_limiter_amplitude: u8,
    pub rev_limiter_frequency: u8,
    pub menu_triggers: bool,

    pub road: f32,
    pub impact: f32,
    pub impact_threshold: f32,
    pub engine: f32,
    pub shift: f32,
    pub shift_ms: f32,
    pub slip: f32,
    pub game_rumble: f32,
    pub menu_rumble: bool,
    pub pause_detection: bool,
    pub quiet_when_stopped: bool,
    pub throttle_gate: bool,
    pub coast_vibration: f32,
    pub smoothing_ms: f32,

    pub rpm_lightbar: bool,
    pub color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Config::preset(Preset::Medium)
    }
}

impl Config {
    pub fn preset(preset: Preset) -> Config {
        let medium = Config {
            preset: Preset::Medium,
            enabled: true,
            udp_port: 20777,
            udp_forward_port: 0,
            hide_other_pads: true,
            log: true,
            trigger_power: 1.0,
            vibration_power: 0.55,
            brake_start_zone: 1,
            brake_min: 1,
            brake_max: 6,
            throttle_start_zone: 2,
            throttle_min: 2,
            throttle_max: 4,
            lockup_vibration: true,
            lockup_threshold: 0.25,
            lockup_amplitude: 5,
            lockup_frequency: 30,
            wheelspin_vibration: true,
            wheelspin_threshold: 0.2,
            wheelspin_amplitude: 5,
            wheelspin_frequency: 50,
            rev_limiter: true,
            rev_limiter_fraction: 0.96,
            rev_limiter_amplitude: 6,
            rev_limiter_frequency: 22,
            menu_triggers: false,
            road: 0.5,
            impact: 0.7,
            impact_threshold: 1.0,
            engine: 0.0,
            shift: 0.5,
            shift_ms: 70.0,
            slip: 0.35,
            game_rumble: 0.25,
            menu_rumble: false,
            pause_detection: true,
            quiet_when_stopped: true,
            throttle_gate: true,
            coast_vibration: 0.0,
            smoothing_ms: 60.0,
            rpm_lightbar: true,
            color: [0, 40, 255],
        };
        match preset {
            Preset::Medium => medium,
            Preset::Soft => Config {
                preset,
                vibration_power: 0.35,
                brake_max: 4,
                throttle_min: 1,
                throttle_max: 3,
                lockup_amplitude: 3,
                wheelspin_amplitude: 3,
                rev_limiter_amplitude: 4,
                road: 0.4,
                impact: 0.5,
                impact_threshold: 1.2,
                shift: 0.35,
                slip: 0.25,
                game_rumble: 0.2,
                smoothing_ms: 90.0,
                ..medium
            },
            Preset::Strong => Config {
                preset,
                vibration_power: 0.85,
                brake_min: 2,
                brake_max: 8,
                throttle_max: 5,
                lockup_amplitude: 7,
                lockup_frequency: 35,
                wheelspin_amplitude: 8,
                wheelspin_frequency: 55,
                rev_limiter_amplitude: 8,
                road: 0.65,
                impact: 1.0,
                impact_threshold: 0.8,
                engine: 0.1,
                shift: 0.8,
                shift_ms: 90.0,
                slip: 0.5,
                game_rumble: 0.5,
                smoothing_ms: 40.0,
                ..medium
            },
        }
    }
}

pub const DEFAULT_INI: &str = "\
; DiRT Rally 2.0 DualSense mod
; The file is reloaded automatically while the game runs.
; In game: hold PS + D-pad Up/Down to change intensity, PS + Options to toggle the mod.

[general]
; soft | medium | strong
preset = medium
; overall strength in percent, applied on top of the preset
trigger_power = 100
vibration_power = 55
enabled = true
udp_port = 20777
udp_forward_port = 0
hide_other_pads = true
log = true

; Fine tuning. Remove the ';' in front of a line to override the preset.
; Trigger strengths: 0..8. Zones: 0..9 (0 = released, 9 = fully pressed).
[triggers]
;brake_start_zone = 1
;brake_min = 1
;brake_max = 6
;throttle_start_zone = 2
;throttle_min = 2
;throttle_max = 4
;lockup_vibration = true
;lockup_threshold = 0.25
;lockup_amplitude = 5
;lockup_frequency = 30
;wheelspin_vibration = true
;wheelspin_threshold = 0.2
;wheelspin_amplitude = 5
;wheelspin_frequency = 50
;rev_limiter = true
;rev_limiter_fraction = 0.96
;rev_limiter_amplitude = 6
;rev_limiter_frequency = 22
;menu_triggers = false

; Vibration parts: 0.0..1.0
[haptics]
;road = 0.5
;impact = 0.7
;impact_threshold = 1.0
;engine = 0.0
;shift = 0.5
;shift_ms = 70
;slip = 0.35
;game_rumble = 0.25
;menu_rumble = false
;pause_detection = true
;quiet_when_stopped = true
;throttle_gate = true
;coast_vibration = 0.0
;smoothing_ms = 60

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

fn parse_percent(v: &str) -> Option<f32> {
    let x: f32 = v.trim_end_matches('%').trim().parse().ok()?;
    if !x.is_finite() {
        return None;
    }
    Some((x / 100.0).clamp(0.0, 1.0))
}

fn lines(text: &str) -> impl Iterator<Item = (usize, String, Option<(String, String)>)> + '_ {
    let mut section = String::new();
    text.lines().enumerate().filter_map(move |(n, raw)| {
        let line = raw.split([';', '#']).next().unwrap_or("").trim();
        if line.is_empty() {
            return None;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            return None;
        }
        let kv = line
            .split_once('=')
            .map(|(k, v)| (format!("{}.{}", section, k.trim().to_ascii_lowercase()), v.trim().to_string()));
        Some((n + 1, line.to_string(), kv))
    })
}

impl Config {
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut warnings = Vec::new();
        let mut preset = Preset::Medium;
        for (n, _, kv) in lines(text) {
            if let Some((k, v)) = kv {
                if k == "general.preset" {
                    match Preset::parse(&v) {
                        Some(p) => preset = p,
                        None => warnings.push(format!("line {n}: unknown preset {v}, using medium")),
                    }
                }
            }
        }
        let mut c = Config::preset(preset);
        for (n, line, kv) in lines(text) {
            let Some((key, v)) = kv else {
                warnings.push(format!("line {n}: expected key = value, got {line}"));
                continue;
            };
            if key == "general.preset" {
                continue;
            }
            if !c.apply(&key, &v) {
                warnings.push(format!("line {n}: bad or unknown setting {key} = {v}"));
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
            "general.trigger_power" => set!(self.trigger_power, parse_percent(v)),
            "general.vibration_power" => set!(self.vibration_power, parse_percent(v)),
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
            "triggers.wheelspin_amplitude" => set!(self.wheelspin_amplitude, u8v()),
            "triggers.wheelspin_frequency" => set!(self.wheelspin_frequency, u8v()),
            "triggers.rev_limiter" => set!(self.rev_limiter, bv()),
            "triggers.rev_limiter_fraction" => set!(self.rev_limiter_fraction, f32v()),
            "triggers.rev_limiter_amplitude" => set!(self.rev_limiter_amplitude, u8v()),
            "triggers.rev_limiter_frequency" => set!(self.rev_limiter_frequency, u8v()),
            "triggers.menu_triggers" => set!(self.menu_triggers, bv()),
            "haptics.master" => set!(self.vibration_power, f32v()),
            "haptics.road" => set!(self.road, f32v()),
            "haptics.impact" => set!(self.impact, f32v()),
            "haptics.impact_threshold" => set!(self.impact_threshold, f32v()),
            "haptics.engine" => set!(self.engine, f32v()),
            "haptics.shift" => set!(self.shift, f32v()),
            "haptics.shift_ms" => set!(self.shift_ms, f32v()),
            "haptics.slip" => set!(self.slip, f32v()),
            "haptics.game_rumble" => set!(self.game_rumble, f32v()),
            "haptics.menu_rumble" => set!(self.menu_rumble, bv()),
            "haptics.pause_detection" => set!(self.pause_detection, bv()),
            "haptics.quiet_when_stopped" => set!(self.quiet_when_stopped, bv()),
            "haptics.throttle_gate" => set!(self.throttle_gate, bv()),
            "haptics.coast_vibration" => set!(self.coast_vibration, f32v()),
            "haptics.smoothing_ms" => set!(self.smoothing_ms, f32v()),
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
            &mut self.wheelspin_amplitude,
            &mut self.rev_limiter_amplitude,
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
            &mut self.trigger_power,
            &mut self.vibration_power,
            &mut self.road,
            &mut self.impact,
            &mut self.engine,
            &mut self.shift,
            &mut self.slip,
            &mut self.game_rumble,
            &mut self.coast_vibration,
        ] {
            *v = v.clamp(0.0, 1.0);
        }
        self.lockup_threshold = self.lockup_threshold.clamp(0.02, 0.95);
        self.wheelspin_threshold = self.wheelspin_threshold.clamp(0.02, 5.0);
        self.rev_limiter_fraction = self.rev_limiter_fraction.clamp(0.5, 1.0);
        self.impact_threshold = self.impact_threshold.max(0.01);
        self.shift_ms = self.shift_ms.clamp(0.0, 1000.0);
        self.smoothing_ms = self.smoothing_ms.clamp(0.0, 500.0);
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
    fn default_ini_lists_every_tunable_of_the_preset() {
        let medium = Config::preset(Preset::Medium);
        let uncommented: String = DEFAULT_INI
            .lines()
            .map(|l| match l.strip_prefix(';') {
                Some(rest) if rest.starts_with(|c: char| c.is_ascii_alphabetic()) => rest.to_string(),
                _ => l.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(uncommented.contains("\nbrake_max = 6"));
        let (c, w) = Config::parse(&uncommented);
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c, medium);
    }

    #[test]
    fn presets_are_ordered_by_strength() {
        let s = Config::preset(Preset::Soft);
        let m = Config::preset(Preset::Medium);
        let h = Config::preset(Preset::Strong);
        assert!(s.vibration_power < m.vibration_power && m.vibration_power < h.vibration_power);
        assert!(s.brake_max < m.brake_max && m.brake_max < h.brake_max);
        assert!(s.impact < m.impact && m.impact < h.impact);
        assert!(s.lockup_amplitude < m.lockup_amplitude && m.lockup_amplitude < h.lockup_amplitude);
    }

    #[test]
    fn preset_can_be_anywhere_and_keys_override_it() {
        let (c, w) = Config::parse("[triggers]\nbrake_max = 3\n[general]\npreset = strong\n");
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(c.preset, Preset::Strong);
        assert_eq!(c.brake_max, 3);
        assert_eq!(c.impact, Config::preset(Preset::Strong).impact);
    }

    #[test]
    fn unknown_preset_warns() {
        let (c, w) = Config::parse("[general]\npreset = turbo\n");
        assert_eq!(c.preset, Preset::Medium);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn power_is_percent() {
        let (c, w) = Config::parse("[general]\ntrigger_power = 40%\nvibration_power = 150\n");
        assert!(w.is_empty(), "{w:?}");
        assert!((c.trigger_power - 0.4).abs() < 1e-6);
        assert_eq!(c.vibration_power, 1.0);
    }

    #[test]
    fn legacy_master_key_maps_to_vibration_power() {
        let (c, w) = Config::parse("[haptics]\nmaster = 0.3\n");
        assert!(w.is_empty());
        assert!((c.vibration_power - 0.3).abs() < 1e-6);
    }

    #[test]
    fn values_are_applied() {
        let (c, w) = Config::parse(
            "[general]\nenabled = off\nudp_port=20888\n[triggers]\nbrake_max = 5 ; comment\n[lightbar]\ncolor = 255,0,10\n",
        );
        assert!(w.is_empty(), "{w:?}");
        assert!(!c.enabled);
        assert_eq!(c.udp_port, 20888);
        assert_eq!(c.brake_max, 5);
        assert_eq!(c.color, [255, 0, 10]);
    }

    #[test]
    fn bad_values_warn_and_keep_defaults() {
        let (c, w) = Config::parse("[triggers]\nbrake_max = lots\nnope = 1\n[haptics]\nroad = NaN\n");
        assert_eq!(w.len(), 3);
        assert_eq!(c.brake_max, Config::default().brake_max);
        assert_eq!(c.road, Config::default().road);
    }

    #[test]
    fn values_are_clamped() {
        let (c, _) = Config::parse("[triggers]\nbrake_min = 6\nbrake_max = 200\nbrake_start_zone = 40\n[haptics]\nroad = -1\nimpact = 9\n");
        assert_eq!(c.brake_max, 8);
        assert_eq!(c.brake_min, 6);
        assert_eq!(c.brake_start_zone, 9);
        assert_eq!(c.road, 0.0);
        assert_eq!(c.impact, 1.0);
        let (c, _) = Config::parse("[triggers]\nthrottle_min = 5\nthrottle_max = 2\n");
        assert_eq!(c.throttle_max, 5);
    }
}
