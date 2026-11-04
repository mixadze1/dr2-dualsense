#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerEffect(pub [u8; 11]);

pub const MODE_OFF: u8 = 0x05;
pub const MODE_FEEDBACK: u8 = 0x21;
pub const MODE_WEAPON: u8 = 0x25;
pub const MODE_VIBRATION: u8 = 0x26;

impl Default for TriggerEffect {
    fn default() -> Self {
        Self::off()
    }
}

fn pack_zones(levels: &[u8; 10]) -> Option<(u16, u32)> {
    let mut active: u16 = 0;
    let mut packed: u32 = 0;
    for (i, &level) in levels.iter().enumerate() {
        if level > 0 {
            let value = (level.min(8) - 1) as u32 & 0x07;
            packed |= value << (3 * i);
            active |= 1 << i;
        }
    }
    if active == 0 {
        None
    } else {
        Some((active, packed))
    }
}

impl TriggerEffect {
    pub fn off() -> Self {
        let mut b = [0u8; 11];
        b[0] = MODE_OFF;
        TriggerEffect(b)
    }

    pub fn mode(&self) -> u8 {
        self.0[0]
    }

    pub fn is_off(&self) -> bool {
        self.0[0] == MODE_OFF
    }

    pub fn feedback(position: u8, strength: u8) -> Self {
        if position > 9 || strength == 0 {
            return Self::off();
        }
        let mut levels = [0u8; 10];
        for level in levels.iter_mut().skip(position as usize) {
            *level = strength.min(8);
        }
        Self::zones(levels)
    }

    pub fn zones(levels: [u8; 10]) -> Self {
        match pack_zones(&levels) {
            None => Self::off(),
            Some((active, packed)) => {
                let mut b = [0u8; 11];
                b[0] = MODE_FEEDBACK;
                b[1] = (active & 0xFF) as u8;
                b[2] = (active >> 8) as u8;
                b[3..7].copy_from_slice(&packed.to_le_bytes());
                TriggerEffect(b)
            }
        }
    }

    pub fn slope(start: u8, end: u8, start_strength: u8, end_strength: u8) -> Self {
        let start = start.min(9);
        let end = end.clamp(start, 9);
        let s0 = start_strength.min(8) as f32;
        let s1 = end_strength.min(8) as f32;
        let mut levels = [0u8; 10];
        for (i, level) in levels.iter_mut().enumerate() {
            let i = i as u8;
            if i < start {
                continue;
            }
            let t = if end == start {
                1.0
            } else {
                ((i.min(end) - start) as f32) / ((end - start) as f32)
            };
            let v = (s0 + (s1 - s0) * t).round();
            *level = v.clamp(1.0, 8.0) as u8;
        }
        if start_strength == 0 && end_strength == 0 {
            return Self::off();
        }
        Self::zones(levels)
    }

    pub fn weapon(start: u8, end: u8, strength: u8) -> Self {
        if !(2..=7).contains(&start) || end <= start || end > 8 || strength == 0 {
            return Self::off();
        }
        let zones: u16 = (1 << start) | (1 << end);
        let mut b = [0u8; 11];
        b[0] = MODE_WEAPON;
        b[1] = (zones & 0xFF) as u8;
        b[2] = (zones >> 8) as u8;
        b[3] = strength.min(8) - 1;
        TriggerEffect(b)
    }

    pub fn vibration(position: u8, amplitude: u8, frequency: u8) -> Self {
        if position > 9 || amplitude == 0 || frequency == 0 {
            return Self::off();
        }
        let mut levels = [0u8; 10];
        for level in levels.iter_mut().skip(position as usize) {
            *level = amplitude.min(8);
        }
        Self::vibration_zones(levels, frequency)
    }

    pub fn vibration_zones(levels: [u8; 10], frequency: u8) -> Self {
        if frequency == 0 {
            return Self::off();
        }
        match pack_zones(&levels) {
            None => Self::off(),
            Some((active, packed)) => {
                let mut b = [0u8; 11];
                b[0] = MODE_VIBRATION;
                b[1] = (active & 0xFF) as u8;
                b[2] = (active >> 8) as u8;
                b[3..7].copy_from_slice(&packed.to_le_bytes());
                b[9] = frequency;
                TriggerEffect(b)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_effect_layout() {
        assert_eq!(TriggerEffect::off().0, [0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn feedback_matches_reference_encoding() {
        let e = TriggerEffect::feedback(0, 8);
        assert_eq!(e.0[0], 0x21);
        assert_eq!(e.0[1], 0xFF);
        assert_eq!(e.0[2], 0x03);
        let packed = u32::from_le_bytes([e.0[3], e.0[4], e.0[5], e.0[6]]);
        assert_eq!(packed, 0x3FFF_FFFF);
        assert_eq!(&e.0[7..], &[0, 0, 0, 0]);
    }

    #[test]
    fn feedback_from_middle_position() {
        let e = TriggerEffect::feedback(5, 1);
        let active = u16::from_le_bytes([e.0[1], e.0[2]]);
        assert_eq!(active, 0b11_1110_0000);
        let packed = u32::from_le_bytes([e.0[3], e.0[4], e.0[5], e.0[6]]);
        assert_eq!(packed, 0);
    }

    #[test]
    fn feedback_invalid_is_off() {
        assert!(TriggerEffect::feedback(10, 4).is_off());
        assert!(TriggerEffect::feedback(0, 0).is_off());
    }

    #[test]
    fn strength_is_clamped_to_eight() {
        assert_eq!(TriggerEffect::feedback(0, 200), TriggerEffect::feedback(0, 8));
    }

    #[test]
    fn slope_rises_monotonically() {
        let e = TriggerEffect::slope(1, 9, 1, 8);
        let active = u16::from_le_bytes([e.0[1], e.0[2]]);
        assert_eq!(active, 0b11_1111_1110);
        let packed = u32::from_le_bytes([e.0[3], e.0[4], e.0[5], e.0[6]]);
        let mut last = 0;
        for i in 1..10 {
            let v = (packed >> (3 * i)) & 7;
            assert!(v >= last, "zone {i} decreased");
            last = v;
        }
        assert_eq!((packed >> 3) & 7, 0);
        assert_eq!((packed >> 27) & 7, 7);
    }

    #[test]
    fn slope_all_zero_is_off() {
        assert!(TriggerEffect::slope(0, 9, 0, 0).is_off());
    }

    #[test]
    fn vibration_layout() {
        let e = TriggerEffect::vibration(2, 6, 40);
        assert_eq!(e.0[0], 0x26);
        let active = u16::from_le_bytes([e.0[1], e.0[2]]);
        assert_eq!(active, 0b11_1111_1100);
        let packed = u32::from_le_bytes([e.0[3], e.0[4], e.0[5], e.0[6]]);
        for i in 2..10 {
            assert_eq!((packed >> (3 * i)) & 7, 5);
        }
        assert_eq!(e.0[9], 40);
    }

    #[test]
    fn vibration_invalid_is_off() {
        assert!(TriggerEffect::vibration(0, 0, 30).is_off());
        assert!(TriggerEffect::vibration(0, 3, 0).is_off());
    }

    #[test]
    fn weapon_layout() {
        let e = TriggerEffect::weapon(2, 6, 8);
        assert_eq!(e.0[0], 0x25);
        assert_eq!(u16::from_le_bytes([e.0[1], e.0[2]]), (1 << 2) | (1 << 6));
        assert_eq!(e.0[3], 7);
        assert!(TriggerEffect::weapon(1, 6, 8).is_off());
    }
}
