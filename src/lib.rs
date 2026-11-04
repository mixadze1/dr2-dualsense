pub mod config;
pub mod controls;
pub mod dualsense;
pub mod effects;
pub mod log;
pub mod runtime;
pub mod telemetry;
pub mod udp;
pub mod xinput;

use std::ffi::c_void;
use std::sync::atomic::Ordering;

use xinput::{Route, XInputCapabilities, XInputState, XInputVibration, ERROR_BAD_ARGUMENTS, ERROR_DEVICE_NOT_CONNECTED, ERROR_SUCCESS};

fn guard<F: FnOnce() -> u32 + std::panic::UnwindSafe>(f: F) -> u32 {
    std::panic::catch_unwind(f).unwrap_or(ERROR_DEVICE_NOT_CONNECTED)
}

fn current_route(user: u32) -> Route {
    let s = runtime::shared();
    let connected = s.pad.lock().map(|g| g.connected).unwrap_or(false);
    xinput::route(user, connected, s.hide_other_pads.load(Ordering::Relaxed))
}

#[no_mangle]
pub unsafe extern "system" fn XInputGetState(user: u32, state: *mut XInputState) -> u32 {
    if state.is_null() {
        return ERROR_BAD_ARGUMENTS;
    }
    let state = state as usize;
    guard(move || match current_route(user) {
        Route::DualSense => {
            let snap = runtime::shared().pad.lock().map(|g| *g).unwrap_or_default();
            if !snap.connected {
                return ERROR_DEVICE_NOT_CONNECTED;
            }
            unsafe { *(state as *mut XInputState) = snap.state };
            ERROR_SUCCESS
        }
        Route::Real(i) => match xinput::real().get_state {
            Some(f) => unsafe { f(i, state as *mut XInputState) },
            None => ERROR_DEVICE_NOT_CONNECTED,
        },
        Route::None => ERROR_DEVICE_NOT_CONNECTED,
    })
}

#[no_mangle]
pub unsafe extern "system" fn XInputSetState(user: u32, vibration: *mut XInputVibration) -> u32 {
    if vibration.is_null() {
        return ERROR_BAD_ARGUMENTS;
    }
    let vibration = vibration as usize;
    guard(move || match current_route(user) {
        Route::DualSense => {
            let v = unsafe { *(vibration as *mut XInputVibration) };
            runtime::shared().rumble.store(((v.left_motor as u32) << 16) | v.right_motor as u32, Ordering::Relaxed);
            ERROR_SUCCESS
        }
        Route::Real(i) => match xinput::real().set_state {
            Some(f) => unsafe { f(i, vibration as *mut XInputVibration) },
            None => ERROR_DEVICE_NOT_CONNECTED,
        },
        Route::None => ERROR_DEVICE_NOT_CONNECTED,
    })
}

#[no_mangle]
pub unsafe extern "system" fn XInputGetCapabilities(user: u32, flags: u32, caps: *mut XInputCapabilities) -> u32 {
    if caps.is_null() {
        return ERROR_BAD_ARGUMENTS;
    }
    let caps = caps as usize;
    guard(move || match current_route(user) {
        Route::DualSense => {
            unsafe { *(caps as *mut XInputCapabilities) = xinput::capabilities() };
            ERROR_SUCCESS
        }
        Route::Real(i) => match xinput::real().get_caps {
            Some(f) => unsafe { f(i, flags, caps as *mut XInputCapabilities) },
            None => ERROR_DEVICE_NOT_CONNECTED,
        },
        Route::None => ERROR_DEVICE_NOT_CONNECTED,
    })
}

#[no_mangle]
pub unsafe extern "system" fn XInputGetDSoundAudioDeviceGuids(user: u32, render: *mut u8, capture: *mut u8) -> u32 {
    let (render, capture) = (render as usize, capture as usize);
    guard(move || match current_route(user) {
        Route::Real(i) => match xinput::real().get_guids {
            Some(f) => unsafe { f(i, render as *mut u8, capture as *mut u8) },
            None => ERROR_DEVICE_NOT_CONNECTED,
        },
        Route::DualSense => {
            for p in [render, capture] {
                if p != 0 {
                    unsafe { std::ptr::write_bytes(p as *mut u8, 0, 16) };
                }
            }
            ERROR_SUCCESS
        }
        Route::None => ERROR_DEVICE_NOT_CONNECTED,
    })
}

#[no_mangle]
pub extern "system" fn DllMain(module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
    match reason {
        DLL_PROCESS_ATTACH => {
            runtime::MODULE.store(module as usize, Ordering::Relaxed);
        }
        DLL_PROCESS_DETACH => {
            let _ = std::panic::catch_unwind(runtime::reset_controller_on_exit);
        }
        _ => {}
    }
    1
}
