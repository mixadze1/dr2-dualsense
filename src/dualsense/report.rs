use super::trigger::TriggerEffect;

pub const USB_OUTPUT_LEN: usize = 48;
pub const BT_OUTPUT_LEN: usize = 78;
pub const COMMON_LEN: usize = 47;

pub const FLAG0_COMPATIBLE_VIBRATION: u8 = 0x01;
pub const FLAG0_HAPTICS_SELECT: u8 = 0x02;
pub const FLAG0_RIGHT_TRIGGER: u8 = 0x04;
pub const FLAG0_LEFT_TRIGGER: u8 = 0x08;
pub const FLAG1_LIGHTBAR: u8 = 0x04;
pub const FLAG1_PLAYER_LEDS: u8 = 0x10;
pub const FLAG2_LIGHTBAR_SETUP: u8 = 0x02;
pub const FLAG2_COMPATIBLE_VIBRATION2: u8 = 0x04;

pub const OFFSET_MOTOR_RIGHT: usize = 2;
pub const OFFSET_MOTOR_LEFT: usize = 3;
pub const OFFSET_RIGHT_TRIGGER: usize = 10;
pub const OFFSET_LEFT_TRIGGER: usize = 21;
pub const OFFSET_FLAG2: usize = 38;
pub const OFFSET_LIGHTBAR_SETUP: usize = 41;
pub const OFFSET_PLAYER_LEDS: usize = 43;
pub const OFFSET_LIGHTBAR: usize = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct OutputState {
    pub left_trigger: TriggerEffect,
    pub right_trigger: TriggerEffect,
    pub motor_left: u8,
    pub motor_right: u8,
    pub lightbar: Option<[u8; 3]>,
    pub player_leds: Option<u8>,
}

impl OutputState {
    pub fn neutral() -> Self {
        OutputState {
            left_trigger: TriggerEffect::off(),
            right_trigger: TriggerEffect::off(),
            motor_left: 0,
            motor_right: 0,
            lightbar: None,
            player_leds: None,
        }
    }
}

pub fn build_common(state: &OutputState) -> [u8; COMMON_LEN] {
    let mut c = [0u8; COMMON_LEN];
    c[0] = FLAG0_COMPATIBLE_VIBRATION | FLAG0_HAPTICS_SELECT | FLAG0_RIGHT_TRIGGER | FLAG0_LEFT_TRIGGER;
    c[OFFSET_MOTOR_RIGHT] = state.motor_right;
    c[OFFSET_MOTOR_LEFT] = state.motor_left;
    c[OFFSET_RIGHT_TRIGGER..OFFSET_RIGHT_TRIGGER + 11].copy_from_slice(&state.right_trigger.0);
    c[OFFSET_LEFT_TRIGGER..OFFSET_LEFT_TRIGGER + 11].copy_from_slice(&state.left_trigger.0);
    c[OFFSET_FLAG2] = FLAG2_COMPATIBLE_VIBRATION2;
    if let Some(rgb) = state.lightbar {
        c[1] |= FLAG1_LIGHTBAR;
        c[OFFSET_FLAG2] |= FLAG2_LIGHTBAR_SETUP;
        c[OFFSET_LIGHTBAR_SETUP] = 0x02;
        c[OFFSET_LIGHTBAR..OFFSET_LIGHTBAR + 3].copy_from_slice(&rgb);
    }
    if let Some(leds) = state.player_leds {
        c[1] |= FLAG1_PLAYER_LEDS;
        c[OFFSET_PLAYER_LEDS] = leds & 0x1F;
    }
    c
}

pub fn build_usb(state: &OutputState) -> [u8; USB_OUTPUT_LEN] {
    let mut r = [0u8; USB_OUTPUT_LEN];
    r[0] = 0x02;
    r[1..1 + COMMON_LEN].copy_from_slice(&build_common(state));
    r
}

pub fn build_bt(state: &OutputState, seq: u8) -> [u8; BT_OUTPUT_LEN] {
    let mut r = [0u8; BT_OUTPUT_LEN];
    r[0] = 0x31;
    r[1] = (seq & 0x0F) << 4;
    r[2] = 0x10;
    r[3..3 + COMMON_LEN].copy_from_slice(&build_common(state));
    let crc = bt_crc(0xA2, &r[..BT_OUTPUT_LEN - 4]);
    r[BT_OUTPUT_LEN - 4..].copy_from_slice(&crc.to_le_bytes());
    r
}

pub fn build(state: &OutputState, bluetooth: bool, seq: u8) -> Vec<u8> {
    if bluetooth {
        build_bt(state, seq).to_vec()
    } else {
        build_usb(state).to_vec()
    }
}

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

pub fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    for &b in data {
        crc = CRC_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc
}

pub fn crc32(data: &[u8]) -> u32 {
    !crc32_update(0xFFFF_FFFF, data)
}

pub fn bt_crc(seed: u8, data: &[u8]) -> u32 {
    let crc = crc32_update(0xFFFF_FFFF, &[seed]);
    !crc32_update(crc, data)
}

pub fn verify_bt_output(report: &[u8]) -> bool {
    if report.len() < BT_OUTPUT_LEN {
        return false;
    }
    let n = BT_OUTPUT_LEN - 4;
    let expected = u32::from_le_bytes([report[n], report[n + 1], report[n + 2], report[n + 3]]);
    bt_crc(0xA2, &report[..n]) == expected
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PadInput {
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub hat: u8,
    pub square: bool,
    pub cross: bool,
    pub circle: bool,
    pub triangle: bool,
    pub l1: bool,
    pub r1: bool,
    pub l2_button: bool,
    pub r2_button: bool,
    pub create: bool,
    pub options: bool,
    pub l3: bool,
    pub r3: bool,
    pub ps: bool,
    pub touchpad: bool,
    pub mute: bool,
}

impl PadInput {
    pub fn centered() -> Self {
        PadInput { lx: 128, ly: 128, rx: 128, ry: 128, hat: 8, ..Default::default() }
    }

    pub fn dpad_up(&self) -> bool {
        matches!(self.hat, 7 | 0 | 1)
    }
    pub fn dpad_right(&self) -> bool {
        matches!(self.hat, 1..=3)
    }
    pub fn dpad_down(&self) -> bool {
        matches!(self.hat, 3..=5)
    }
    pub fn dpad_left(&self) -> bool {
        matches!(self.hat, 5..=7)
    }
}

fn decode(lx: u8, ly: u8, rx: u8, ry: u8, l2: u8, r2: u8, b0: u8, b1: u8, b2: u8) -> PadInput {
    PadInput {
        lx,
        ly,
        rx,
        ry,
        l2,
        r2,
        hat: b0 & 0x0F,
        square: b0 & 0x10 != 0,
        cross: b0 & 0x20 != 0,
        circle: b0 & 0x40 != 0,
        triangle: b0 & 0x80 != 0,
        l1: b1 & 0x01 != 0,
        r1: b1 & 0x02 != 0,
        l2_button: b1 & 0x04 != 0,
        r2_button: b1 & 0x08 != 0,
        create: b1 & 0x10 != 0,
        options: b1 & 0x20 != 0,
        l3: b1 & 0x40 != 0,
        r3: b1 & 0x80 != 0,
        ps: b2 & 0x01 != 0,
        touchpad: b2 & 0x02 != 0,
        mute: b2 & 0x04 != 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    UsbFull,
    BluetoothFull,
    BluetoothSimple,
}

pub fn parse_input(report: &[u8], bluetooth: bool) -> Option<(PadInput, InputKind)> {
    let id = *report.first()?;
    match (bluetooth, id) {
        (false, 0x01) if report.len() >= 11 => {
            let d = &report[1..];
            Some((decode(d[0], d[1], d[2], d[3], d[4], d[5], d[7], d[8], d[9]), InputKind::UsbFull))
        }
        (true, 0x31) if report.len() >= 12 => {
            let d = &report[2..];
            Some((decode(d[0], d[1], d[2], d[3], d[4], d[5], d[7], d[8], d[9]), InputKind::BluetoothFull))
        }
        (true, 0x01) if report.len() >= 10 => {
            let d = &report[1..];
            Some((decode(d[0], d[1], d[2], d[3], d[7], d[8], d[4], d[5], d[6]), InputKind::BluetoothSimple))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_reference_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn bt_crc_equals_crc_of_prefixed_buffer() {
        let data = [1u8, 2, 3, 4, 5, 250];
        let mut prefixed = vec![0xA2];
        prefixed.extend_from_slice(&data);
        assert_eq!(bt_crc(0xA2, &data), crc32(&prefixed));
    }

    #[test]
    fn usb_report_layout() {
        let state = OutputState {
            left_trigger: TriggerEffect::feedback(2, 5),
            right_trigger: TriggerEffect::vibration(1, 4, 30),
            motor_left: 200,
            motor_right: 50,
            lightbar: Some([1, 2, 3]),
            player_leds: Some(0b00100),
        };
        let r = build_usb(&state);
        assert_eq!(r.len(), 48);
        assert_eq!(r[0], 0x02);
        assert_eq!(r[1], 0x0F);
        assert_eq!(r[2], FLAG1_LIGHTBAR | FLAG1_PLAYER_LEDS);
        assert_eq!(r[3], 50);
        assert_eq!(r[4], 200);
        assert_eq!(&r[11..22], &state.right_trigger.0);
        assert_eq!(&r[22..33], &state.left_trigger.0);
        assert_eq!(r[39], FLAG2_COMPATIBLE_VIBRATION2 | FLAG2_LIGHTBAR_SETUP);
        assert_eq!(r[42], 0x02);
        assert_eq!(r[44], 0b00100);
        assert_eq!(&r[45..48], &[1, 2, 3]);
    }

    #[test]
    fn bt_report_layout_and_crc() {
        let state = OutputState {
            left_trigger: TriggerEffect::feedback(0, 8),
            motor_left: 10,
            ..OutputState::neutral()
        };
        let r = build_bt(&state, 0x13);
        assert_eq!(r.len(), 78);
        assert_eq!(r[0], 0x31);
        assert_eq!(r[1], 0x30);
        assert_eq!(r[2], 0x10);
        assert_eq!(r[3], 0x0F);
        assert_eq!(r[3 + OFFSET_MOTOR_LEFT], 10);
        assert_eq!(&r[3 + OFFSET_LEFT_TRIGGER..3 + OFFSET_LEFT_TRIGGER + 11], &state.left_trigger.0);
        assert!(verify_bt_output(&r));
        let mut broken = r;
        broken[10] ^= 0xFF;
        assert!(!verify_bt_output(&broken));
    }

    #[test]
    fn no_lightbar_flags_when_unset() {
        let r = build_usb(&OutputState::neutral());
        assert_eq!(r[2], 0);
        assert_eq!(r[39], FLAG2_COMPATIBLE_VIBRATION2);
        assert_eq!(r[11], 0x05);
        assert_eq!(r[22], 0x05);
    }

    fn usb_input(lx: u8, l2: u8, r2: u8, b0: u8, b1: u8, b2: u8) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x01;
        r[1] = lx;
        r[2] = 128;
        r[3] = 128;
        r[4] = 128;
        r[5] = l2;
        r[6] = r2;
        r[8] = b0;
        r[9] = b1;
        r[10] = b2;
        r
    }

    #[test]
    fn parse_usb_input() {
        let r = usb_input(10, 99, 200, 0x20 | 0x02, 0x01 | 0x20, 0x01);
        let (p, kind) = parse_input(&r, false).unwrap();
        assert_eq!(kind, InputKind::UsbFull);
        assert_eq!(p.lx, 10);
        assert_eq!(p.l2, 99);
        assert_eq!(p.r2, 200);
        assert_eq!(p.hat, 2);
        assert!(p.cross && !p.square && !p.circle && !p.triangle);
        assert!(p.l1 && p.options && !p.r1);
        assert!(p.ps);
        assert!(p.dpad_right() && !p.dpad_up());
    }

    #[test]
    fn parse_bt_full_input() {
        let mut r = vec![0u8; 78];
        r[0] = 0x31;
        r[2] = 1;
        r[3] = 2;
        r[4] = 3;
        r[5] = 4;
        r[6] = 5;
        r[7] = 6;
        r[9] = 0x80 | 0x08;
        r[10] = 0x02 | 0x80;
        r[11] = 0x02;
        let (p, kind) = parse_input(&r, true).unwrap();
        assert_eq!(kind, InputKind::BluetoothFull);
        assert_eq!((p.lx, p.ly, p.rx, p.ry, p.l2, p.r2), (1, 2, 3, 4, 5, 6));
        assert!(p.triangle && p.r1 && p.r3 && p.touchpad);
        assert_eq!(p.hat, 8);
    }

    #[test]
    fn parse_bt_simple_input() {
        let mut r = vec![0u8; 78];
        r[0] = 0x01;
        r[1] = 11;
        r[2] = 22;
        r[3] = 33;
        r[4] = 44;
        r[5] = 0x40 | 0x04;
        r[6] = 0x10;
        r[7] = 0x01;
        r[8] = 77;
        r[9] = 88;
        let (p, kind) = parse_input(&r, true).unwrap();
        assert_eq!(kind, InputKind::BluetoothSimple);
        assert_eq!((p.lx, p.ly, p.rx, p.ry, p.l2, p.r2), (11, 22, 33, 44, 77, 88));
        assert!(p.circle && p.create && p.ps);
        assert!(p.dpad_down());
    }

    #[test]
    fn parse_rejects_unknown() {
        assert!(parse_input(&[0x05, 0, 0], false).is_none());
        assert!(parse_input(&[], true).is_none());
        assert!(parse_input(&[0x31, 0, 0], false).is_none());
    }

    #[test]
    fn dpad_diagonals() {
        let mut p = PadInput::centered();
        p.hat = 7;
        assert!(p.dpad_up() && p.dpad_left());
        p.hat = 3;
        assert!(p.dpad_down() && p.dpad_right());
        p.hat = 8;
        assert!(!p.dpad_up() && !p.dpad_down() && !p.dpad_left() && !p.dpad_right());
    }
}
