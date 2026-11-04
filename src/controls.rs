use crate::dualsense::report::PadInput;

pub const LEVELS: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Intensity(i32),
    Toggled(bool),
}

pub struct Controls {
    pub level: i32,
    pub enabled: bool,
    prev: PadInput,
    swallow_ps: bool,
}

impl Controls {
    pub fn new(enabled: bool) -> Self {
        Controls { level: LEVELS, enabled, prev: PadInput::centered(), swallow_ps: false }
    }

    pub fn intensity(&self) -> f32 {
        self.level as f32 / LEVELS as f32
    }

    pub fn process(&mut self, pad: &PadInput) -> (PadInput, Option<Event>) {
        let mut out = *pad;
        let mut event = None;
        if pad.ps {
            let up = pad.dpad_up() && !self.prev.dpad_up();
            let down = pad.dpad_down() && !self.prev.dpad_down();
            let toggle = pad.options && !self.prev.options;
            if up && self.level < LEVELS {
                self.level += 1;
                event = Some(Event::Intensity(self.level));
            } else if down && self.level > 0 {
                self.level -= 1;
                event = Some(Event::Intensity(self.level));
            } else if up || down {
                event = Some(Event::Intensity(self.level));
            }
            if toggle {
                self.enabled = !self.enabled;
                event = Some(Event::Toggled(self.enabled));
            }
            out.hat = 8;
            out.options = false;
            self.swallow_ps = true;
        } else if self.swallow_ps {
            if pad.hat != 8 || pad.options {
                out.hat = 8;
                out.options = false;
            } else {
                self.swallow_ps = false;
            }
        }
        self.prev = *pad;
        (out, event)
    }
}

pub fn level_leds(level: i32) -> u8 {
    let bars = ((level.clamp(0, LEVELS) as f32 / LEVELS as f32) * 5.0).ceil() as u32;
    const MAP: [u8; 6] = [0b00000, 0b00100, 0b01010, 0b10101, 0b11011, 0b11111];
    MAP[bars.min(5) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad() -> PadInput {
        PadInput::centered()
    }

    #[test]
    fn dpad_passes_through_without_ps() {
        let mut c = Controls::new(true);
        let mut p = pad();
        p.hat = 0;
        let (out, ev) = c.process(&p);
        assert_eq!(out.hat, 0);
        assert!(ev.is_none());
    }

    #[test]
    fn ps_down_lowers_intensity_once_per_press() {
        let mut c = Controls::new(true);
        let mut p = pad();
        p.ps = true;
        c.process(&p);
        p.hat = 4;
        let (out, ev) = c.process(&p);
        assert_eq!(ev, Some(Event::Intensity(9)));
        assert_eq!(out.hat, 8);
        let (_, ev) = c.process(&p);
        assert!(ev.is_none());
        p.hat = 8;
        c.process(&p);
        p.hat = 4;
        let (_, ev) = c.process(&p);
        assert_eq!(ev, Some(Event::Intensity(8)));
        assert!((c.intensity() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn intensity_is_bounded() {
        let mut c = Controls::new(true);
        let mut p = pad();
        p.ps = true;
        for _ in 0..30 {
            p.hat = 0;
            c.process(&p);
            p.hat = 8;
            c.process(&p);
        }
        assert_eq!(c.level, LEVELS);
        for _ in 0..30 {
            p.hat = 4;
            c.process(&p);
            p.hat = 8;
            c.process(&p);
        }
        assert_eq!(c.level, 0);
    }

    #[test]
    fn ps_options_toggles_and_is_hidden_from_game() {
        let mut c = Controls::new(true);
        let mut p = pad();
        p.ps = true;
        c.process(&p);
        p.options = true;
        let (out, ev) = c.process(&p);
        assert_eq!(ev, Some(Event::Toggled(false)));
        assert!(!out.options);
        assert!(!c.enabled);
    }

    #[test]
    fn releasing_ps_first_keeps_combo_hidden_until_released() {
        let mut c = Controls::new(true);
        let mut p = pad();
        p.ps = true;
        p.options = true;
        c.process(&p);
        p.ps = false;
        let (out, _) = c.process(&p);
        assert!(!out.options);
        p.options = false;
        c.process(&p);
        p.options = true;
        let (out, _) = c.process(&p);
        assert!(out.options);
    }

    #[test]
    fn leds_show_level() {
        assert_eq!(level_leds(0), 0);
        assert_eq!(level_leds(10), 0b11111);
        assert_eq!(level_leds(5), 0b10101);
        assert_eq!(level_leds(1), 0b00100);
    }
}
