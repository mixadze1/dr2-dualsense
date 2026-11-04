use std::sync::OnceLock;

use crate::dualsense::report::PadInput;

pub const ERROR_SUCCESS: u32 = 0;
pub const ERROR_BAD_ARGUMENTS: u32 = 160;
pub const ERROR_DEVICE_NOT_CONNECTED: u32 = 1167;

pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const BACK: u16 = 0x0020;
pub const LEFT_THUMB: u16 = 0x0040;
pub const RIGHT_THUMB: u16 = 0x0080;
pub const LEFT_SHOULDER: u16 = 0x0100;
pub const RIGHT_SHOULDER: u16 = 0x0200;
pub const BUTTON_A: u16 = 0x1000;
pub const BUTTON_B: u16 = 0x2000;
pub const BUTTON_X: u16 = 0x4000;
pub const BUTTON_Y: u16 = 0x8000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XInputGamepad {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XInputState {
    pub packet_number: u32,
    pub gamepad: XInputGamepad,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XInputVibration {
    pub left_motor: u16,
    pub right_motor: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XInputCapabilities {
    pub kind: u8,
    pub sub_type: u8,
    pub flags: u16,
    pub gamepad: XInputGamepad,
    pub vibration: XInputVibration,
}

pub fn axis(v: u8) -> i16 {
    ((v as i32) * 65535 / 255 - 32768) as i16
}

pub fn axis_inverted(v: u8) -> i16 {
    (32767 - (v as i32) * 65535 / 255) as i16
}

pub fn map_pad(p: &PadInput) -> XInputGamepad {
    let mut b = 0u16;
    let mut set = |cond: bool, bit: u16| {
        if cond {
            b |= bit;
        }
    };
    set(p.dpad_up(), DPAD_UP);
    set(p.dpad_down(), DPAD_DOWN);
    set(p.dpad_left(), DPAD_LEFT);
    set(p.dpad_right(), DPAD_RIGHT);
    set(p.options, START);
    set(p.create || p.touchpad, BACK);
    set(p.l3, LEFT_THUMB);
    set(p.r3, RIGHT_THUMB);
    set(p.l1, LEFT_SHOULDER);
    set(p.r1, RIGHT_SHOULDER);
    set(p.cross, BUTTON_A);
    set(p.circle, BUTTON_B);
    set(p.square, BUTTON_X);
    set(p.triangle, BUTTON_Y);
    XInputGamepad {
        buttons: b,
        left_trigger: p.l2,
        right_trigger: p.r2,
        thumb_lx: axis(p.lx),
        thumb_ly: axis_inverted(p.ly),
        thumb_rx: axis(p.rx),
        thumb_ry: axis_inverted(p.ry),
    }
}

pub fn capabilities() -> XInputCapabilities {
    XInputCapabilities {
        kind: 1,
        sub_type: 1,
        flags: 0,
        gamepad: XInputGamepad {
            buttons: 0xF3FF,
            left_trigger: 0xFF,
            right_trigger: 0xFF,
            thumb_lx: -64,
            thumb_ly: -64,
            thumb_rx: -64,
            thumb_ry: -64,
        },
        vibration: XInputVibration { left_motor: 0xFFFF, right_motor: 0xFFFF },
    }
}

type GetStateFn = unsafe extern "system" fn(u32, *mut XInputState) -> u32;
type SetStateFn = unsafe extern "system" fn(u32, *mut XInputVibration) -> u32;
type GetCapsFn = unsafe extern "system" fn(u32, u32, *mut XInputCapabilities) -> u32;
type GetGuidsFn = unsafe extern "system" fn(u32, *mut u8, *mut u8) -> u32;

pub struct RealXInput {
    pub get_state: Option<GetStateFn>,
    pub set_state: Option<SetStateFn>,
    pub get_caps: Option<GetCapsFn>,
    pub get_guids: Option<GetGuidsFn>,
}

static REAL: OnceLock<RealXInput> = OnceLock::new();

pub fn real() -> &'static RealXInput {
    REAL.get_or_init(load_real)
}

fn load_real() -> RealXInput {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
    let mut dir = [0u16; 260];
    let n = unsafe { GetSystemDirectoryW(dir.as_mut_ptr(), dir.len() as u32) } as usize;
    let mut path: Vec<u16> = dir[..n.min(dir.len())].to_vec();
    path.extend("\\xinput9_1_0.dll".encode_utf16());
    path.push(0);
    let module = unsafe { LoadLibraryW(path.as_ptr()) };
    if module.is_null() {
        crate::log::write("real xinput9_1_0.dll could not be loaded");
        return RealXInput { get_state: None, set_state: None, get_caps: None, get_guids: None };
    }
    unsafe {
        let p = |name: &[u8]| GetProcAddress(module, name.as_ptr());
        RealXInput {
            get_state: p(b"XInputGetState\0").map(|f| std::mem::transmute::<_, GetStateFn>(f)),
            set_state: p(b"XInputSetState\0").map(|f| std::mem::transmute::<_, SetStateFn>(f)),
            get_caps: p(b"XInputGetCapabilities\0").map(|f| std::mem::transmute::<_, GetCapsFn>(f)),
            get_guids: p(b"XInputGetDSoundAudioDeviceGuids\0").map(|f| std::mem::transmute::<_, GetGuidsFn>(f)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    DualSense,
    Real(u32),
    None,
}

pub fn route(user: u32, dualsense_connected: bool, hide_other_pads: bool) -> Route {
    if user > 3 {
        return Route::None;
    }
    if !dualsense_connected {
        return Route::Real(user);
    }
    if user == 0 {
        Route::DualSense
    } else if hide_other_pads {
        Route::None
    } else {
        Route::Real(user - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_sizes_match_xinput_abi() {
        assert_eq!(std::mem::size_of::<XInputGamepad>(), 12);
        assert_eq!(std::mem::size_of::<XInputState>(), 16);
        assert_eq!(std::mem::size_of::<XInputVibration>(), 4);
        assert_eq!(std::mem::size_of::<XInputCapabilities>(), 20);
    }

    #[test]
    fn axes_cover_full_range() {
        assert_eq!(axis(0), -32768);
        assert_eq!(axis(255), 32767);
        assert!(axis(128).abs() < 200);
        assert_eq!(axis_inverted(0), 32767);
        assert_eq!(axis_inverted(255), -32768);
    }

    #[test]
    fn buttons_map_to_xbox_layout() {
        let mut p = PadInput::centered();
        p.cross = true;
        p.circle = true;
        p.square = true;
        p.triangle = true;
        p.l1 = true;
        p.r1 = true;
        p.options = true;
        p.create = true;
        p.l3 = true;
        p.r3 = true;
        p.hat = 0;
        let g = map_pad(&p);
        assert_eq!(
            g.buttons,
            BUTTON_A | BUTTON_B | BUTTON_X | BUTTON_Y | LEFT_SHOULDER | RIGHT_SHOULDER | START | BACK | LEFT_THUMB | RIGHT_THUMB | DPAD_UP
        );
    }

    #[test]
    fn triggers_and_sticks_map() {
        let mut p = PadInput::centered();
        p.l2 = 10;
        p.r2 = 250;
        p.ly = 0;
        p.lx = 255;
        let g = map_pad(&p);
        assert_eq!(g.left_trigger, 10);
        assert_eq!(g.right_trigger, 250);
        assert_eq!(g.thumb_ly, 32767);
        assert_eq!(g.thumb_lx, 32767);
    }

    #[test]
    fn ps_button_is_not_exposed() {
        let mut p = PadInput::centered();
        p.ps = true;
        p.mute = true;
        assert_eq!(map_pad(&p).buttons, 0);
    }

    #[test]
    fn routing() {
        assert_eq!(route(0, true, true), Route::DualSense);
        assert_eq!(route(1, true, true), Route::None);
        assert_eq!(route(1, true, false), Route::Real(0));
        assert_eq!(route(2, false, true), Route::Real(2));
        assert_eq!(route(4, false, true), Route::None);
    }
}
