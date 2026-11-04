use std::path::PathBuf;

use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use xinput9_1_0::xinput::{XInputState, ERROR_BAD_ARGUMENTS, ERROR_DEVICE_NOT_CONNECTED};

fn dll_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let candidates = [profile_dir.join("xinput9_1_0.dll"), profile_dir.parent().unwrap().join("release").join("xinput9_1_0.dll")];
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| panic!("xinput9_1_0.dll not built; run `cargo build` before `cargo test`"))
}

#[test]
fn dll_exports_xinput_api() {
    let path = dll_path();
    assert!(path.exists(), "{} missing", path.display());
    let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain(Some(0)).collect();
    let module = unsafe { LoadLibraryW(wide.as_ptr()) };
    assert!(!module.is_null());
    for name in [
        &b"XInputGetState\0"[..],
        b"XInputSetState\0",
        b"XInputGetCapabilities\0",
        b"XInputGetDSoundAudioDeviceGuids\0",
    ] {
        assert!(unsafe { GetProcAddress(module, name.as_ptr()) }.is_some(), "{}", String::from_utf8_lossy(name));
    }
    type GetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;
    let get: GetState = unsafe { std::mem::transmute(GetProcAddress(module, b"XInputGetState\0".as_ptr()).unwrap()) };
    assert_eq!(unsafe { get(0, std::ptr::null_mut()) }, ERROR_BAD_ARGUMENTS);
    let mut st = XInputState::default();
    assert_eq!(unsafe { get(7, &mut st) }, ERROR_DEVICE_NOT_CONNECTED);
}
