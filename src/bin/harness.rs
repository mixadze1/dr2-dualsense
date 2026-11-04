use std::time::{Duration, Instant};

use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use xinput9_1_0::xinput::{XInputCapabilities, XInputState, XInputVibration};

type GetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;
type SetState = unsafe extern "system" fn(u32, *mut XInputVibration) -> u32;
type GetCaps = unsafe extern "system" fn(u32, u32, *mut XInputCapabilities) -> u32;

fn main() {
    let mut dll = String::from("target\\release\\xinput9_1_0.dll");
    let mut seconds = 10.0f32;
    let mut rumble = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dll" => dll = args.next().unwrap_or(dll),
            "--seconds" => seconds = args.next().and_then(|v| v.parse().ok()).unwrap_or(seconds),
            "--rumble" => rumble = true,
            _ => {
                eprintln!("usage: dr2ds-harness [--dll path] [--seconds N] [--rumble]");
                return;
            }
        }
    }
    let wide: Vec<u16> = dll.encode_utf16().chain(Some(0)).collect();
    let module = unsafe { LoadLibraryW(wide.as_ptr()) };
    if module.is_null() {
        eprintln!("cannot load {dll}");
        std::process::exit(2);
    }
    let (get_state, set_state, get_caps) = unsafe {
        let g = GetProcAddress(module, b"XInputGetState\0".as_ptr()).expect("XInputGetState export");
        let s = GetProcAddress(module, b"XInputSetState\0".as_ptr()).expect("XInputSetState export");
        let c = GetProcAddress(module, b"XInputGetCapabilities\0".as_ptr()).expect("XInputGetCapabilities export");
        (
            std::mem::transmute::<_, GetState>(g),
            std::mem::transmute::<_, SetState>(s),
            std::mem::transmute::<_, GetCaps>(c),
        )
    };
    println!("loaded {dll}");
    let start = Instant::now();
    let mut last_packet = u32::MAX;
    let mut last_rc = u32::MAX;
    let mut connected_once = false;
    while start.elapsed().as_secs_f32() < seconds {
        let mut st = XInputState::default();
        let rc = unsafe { get_state(0, &mut st) };
        if rc != last_rc {
            println!("[{:6.2}] XInputGetState(0) -> {rc}", start.elapsed().as_secs_f32());
            last_rc = rc;
            if rc == 0 && !connected_once {
                connected_once = true;
                let mut caps = XInputCapabilities::default();
                let crc = unsafe { get_caps(0, 1, &mut caps) };
                println!("         XInputGetCapabilities -> {crc} type {} subtype {}", caps.kind, caps.sub_type);
            }
        }
        if rc == 0 && st.packet_number != last_packet {
            last_packet = st.packet_number;
            let g = st.gamepad;
            println!(
                "[{:6.2}] packet {:5} buttons {:04X} L2 {:3} R2 {:3} LX {:6} LY {:6}",
                start.elapsed().as_secs_f32(),
                st.packet_number,
                g.buttons,
                g.left_trigger,
                g.right_trigger,
                g.thumb_lx,
                g.thumb_ly
            );
        }
        if rumble {
            let phase = start.elapsed().as_secs_f32() % 2.0;
            let mut v = if phase < 0.5 {
                XInputVibration { left_motor: 40000, right_motor: 0 }
            } else if phase < 1.0 {
                XInputVibration { left_motor: 0, right_motor: 40000 }
            } else {
                XInputVibration::default()
            };
            unsafe { set_state(0, &mut v) };
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    let mut v = XInputVibration::default();
    unsafe { set_state(0, &mut v) };
    println!("done; connected = {connected_once}");
}
