use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use crate::config::{Config, DEFAULT_INI};
use crate::controls::{level_leds, Controls, Event};
use crate::dualsense::device::{self, Device};
use crate::dualsense::report::{self, parse_input, InputKind, OutputState};
use crate::effects::{Engine, Inputs};
use crate::log;
use crate::telemetry::Telemetry;
use crate::udp::Receiver;
use crate::xinput::{map_pad, XInputState};

pub static MODULE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Default)]
pub struct PadSnapshot {
    pub connected: bool,
    pub state: XInputState,
}

pub struct Shared {
    pub pad: Mutex<PadSnapshot>,
    pub rumble: AtomicU32,
    pub hide_other_pads: AtomicBool,
    pub telemetry: Mutex<Option<(Telemetry, Instant)>>,
    pub telemetry_packets: AtomicU64,
    pub input_reports: AtomicU64,
    pub output_reports: AtomicU64,
}

struct ResetTarget {
    path: Vec<u16>,
    bluetooth: bool,
    output_len: usize,
}

static RESET: Mutex<Option<ResetTarget>> = Mutex::new(None);
static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();

pub fn shared() -> &'static Arc<Shared> {
    SHARED.get_or_init(start)
}

pub fn module_dir() -> PathBuf {
    use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
    let module = MODULE.load(Ordering::Relaxed);
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(module as _, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    let p = PathBuf::from(String::from_utf16_lossy(&buf[..n]));
    p.parent().map(|d| d.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

fn config_path() -> PathBuf {
    module_dir().join("dr2_dualsense.ini")
}

fn load_config() -> (Config, Option<SystemTime>) {
    let path = config_path();
    if !path.exists() {
        let _ = std::fs::write(&path, DEFAULT_INI);
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (cfg, warnings) = Config::parse(&text);
    for w in warnings {
        log::write(&format!("config: {w}"));
    }
    let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    (cfg, mtime)
}

fn start() -> Arc<Shared> {
    let (cfg, mtime) = load_config();
    log::init(Some(module_dir().join("dr2_dualsense.log")), cfg.log);
    log::write(&format!("config loaded from {}: preset {:?}, trigger power {:.0}%, vibration power {:.0}%", config_path().display(), cfg.preset, cfg.trigger_power * 100.0, cfg.vibration_power * 100.0));
    let shared = Arc::new(Shared {
        pad: Mutex::new(PadSnapshot::default()),
        rumble: AtomicU32::new(0),
        hide_other_pads: AtomicBool::new(cfg.hide_other_pads),
        telemetry: Mutex::new(None),
        telemetry_packets: AtomicU64::new(0),
        input_reports: AtomicU64::new(0),
        output_reports: AtomicU64::new(0),
    });
    {
        let s = shared.clone();
        let (port, fwd) = (cfg.udp_port, cfg.udp_forward_port);
        let _ = std::thread::Builder::new()
            .name("dr2ds-telemetry".into())
            .spawn(move || supervise("telemetry", || telemetry_loop(&s, port, fwd)));
    }
    {
        let s = shared.clone();
        let _ = std::thread::Builder::new()
            .name("dr2ds-device".into())
            .spawn(move || supervise("device", || device_loop(&s, cfg.clone(), mtime)));
    }
    shared
}

fn supervise<F: Fn() + std::panic::RefUnwindSafe>(name: &str, f: F) {
    loop {
        if let Err(e) = std::panic::catch_unwind(&f) {
            let msg = e
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| e.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            log::write(&format!("{name} thread panicked: {msg}; restarting"));
            std::thread::sleep(Duration::from_secs(1));
        } else {
            return;
        }
    }
}

fn telemetry_loop(shared: &Shared, port: u16, forward: u16) {
    loop {
        let mut rx = match Receiver::bind(port, forward) {
            Ok(r) => r,
            Err(e) => {
                log::write(&format!("cannot listen for telemetry on 127.0.0.1:{port}: {e}; retrying"));
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        log::write(&format!("listening for DiRT Rally 2.0 telemetry on 127.0.0.1:{port}"));
        loop {
            match rx.recv() {
                Ok(Some(t)) => {
                    let n = shared.telemetry_packets.fetch_add(1, Ordering::Relaxed);
                    if n == 0 {
                        log::write(&format!("first telemetry packet: speed {:.1} m/s, rpm {:.0}, max rpm {:.0}", t.speed, t.rpm, t.max_rpm));
                    }
                    if let Ok(mut g) = shared.telemetry.lock() {
                        *g = Some((t, Instant::now()));
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    log::write(&format!("telemetry socket error: {e}"));
                    break;
                }
            }
        }
    }
}

fn set_connected(shared: &Shared, connected: bool) {
    if let Ok(mut g) = shared.pad.lock() {
        g.connected = connected;
        if !connected {
            g.state.gamepad = Default::default();
        }
    }
}

struct Feedback {
    until: Instant,
    leds: Option<u8>,
    lightbar: Option<[u8; 3]>,
}

fn open_first() -> Option<Device> {
    let list = device::enumerate();
    let info = list.into_iter().next()?;
    let name = if info.product_id == device::DUALSENSE_EDGE_PID { "DualSense Edge" } else { "DualSense" };
    match Device::open(info.clone()) {
        Ok(dev) => {
            log::write(&format!(
                "{name} opened over {} (input {} / output {} / feature {} bytes): {}",
                if info.bluetooth { "Bluetooth" } else { "USB" },
                info.input_len,
                info.output_len,
                info.feature_len,
                info.path_string()
            ));
            if info.bluetooth {
                match dev.get_feature(0x05) {
                    Ok(_) => log::write("bluetooth full report mode requested"),
                    Err(e) => log::write(&format!("calibration feature report failed: {e}")),
                }
            }
            if let Ok(mut g) = RESET.lock() {
                *g = Some(ResetTarget { path: info.path.clone(), bluetooth: info.bluetooth, output_len: info.output_len });
            }
            Some(dev)
        }
        Err(e) => {
            log::write(&format!("{name} found but could not be opened: {e}"));
            None
        }
    }
}

fn device_loop(shared: &Shared, mut cfg: Config, mut cfg_mtime: Option<SystemTime>) {
    let mut engine = Engine::new(cfg.clone());
    let mut controls = Controls::new(cfg.enabled);
    let mut dev: Option<Device> = None;
    let mut last_scan: Option<Instant> = None;
    let mut last_cfg_check = Instant::now();
    let mut last_tick = Instant::now();
    let mut last_send = Instant::now() - Duration::from_secs(10);
    let mut last_sent: Option<OutputState> = None;
    let mut seq: u8 = 0;
    let mut packet: u32 = 0;
    let mut last_kind: Option<InputKind> = None;
    let mut feedback: Option<Feedback> = None;
    let mut last_stats = Instant::now();
    let clock0 = Instant::now();
    let mut last_mode = (false, false);

    loop {
        if last_cfg_check.elapsed() > Duration::from_secs(1) {
            last_cfg_check = Instant::now();
            let mtime = std::fs::metadata(config_path()).and_then(|m| m.modified()).ok();
            if mtime != cfg_mtime {
                let (c, m) = load_config();
                cfg_mtime = m;
                if c.enabled != cfg.enabled {
                    controls.enabled = c.enabled;
                }
                shared.hide_other_pads.store(c.hide_other_pads, Ordering::Relaxed);
                engine.cfg = c.clone();
                cfg = c;
                log::write("config reloaded");
            }
        }

        if dev.is_none() {
            set_connected(shared, false);
            if last_scan.map_or(true, |t| t.elapsed() > Duration::from_secs(1)) {
                last_scan = Some(Instant::now());
                dev = open_first();
                if dev.is_some() {
                    last_sent = None;
                    last_kind = None;
                    set_connected(shared, true);
                }
            }
            if dev.is_none() {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        }

        let d = dev.as_mut().unwrap();
        match d.read(4) {
            Ok(Some(raw)) => {
                if let Some((pad, kind)) = parse_input(&raw, d.info.bluetooth) {
                    shared.input_reports.fetch_add(1, Ordering::Relaxed);
                    if last_kind != Some(kind) {
                        log::write(&format!("input report format: {kind:?}"));
                        last_kind = Some(kind);
                    }
                    let (filtered, event) = controls.process(&pad);
                    let gamepad = map_pad(&filtered);
                    if let Ok(mut g) = shared.pad.lock() {
                        if g.state.gamepad != gamepad || !g.connected {
                            packet = packet.wrapping_add(1);
                            g.state = XInputState { packet_number: packet, gamepad };
                        }
                        g.connected = true;
                    }
                    if let Some(ev) = event {
                        log::write(&format!("control event: {ev:?}"));
                        feedback = Some(match ev {
                            Event::Intensity(l) => Feedback { until: Instant::now() + Duration::from_millis(1500), leds: Some(level_leds(l)), lightbar: None },
                            Event::Toggled(on) => Feedback {
                                until: Instant::now() + Duration::from_millis(800),
                                leds: Some(if on { 0b11111 } else { 0 }),
                                lightbar: Some(if on { [0, 255, 0] } else { [255, 0, 0] }),
                            },
                        });
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                log::write(&format!("controller read failed: {e}; reconnecting"));
                dev = None;
                continue;
            }
        }

        let now = Instant::now();
        if now.duration_since(last_tick) >= Duration::from_millis(8) {
            let dt = now.duration_since(last_tick).as_secs_f32();
            last_tick = now;
            let snap = shared.telemetry.lock().ok().and_then(|g| *g);
            let (telemetry, age) = match snap {
                Some((t, at)) => (Some(t), now.duration_since(at).as_secs_f32()),
                None => (None, f32::MAX),
            };
            let rumble = shared.rumble.load(Ordering::Relaxed);
            let inputs = Inputs {
                telemetry: telemetry.as_ref(),
                telemetry_age: age,
                game_rumble: ((rumble >> 16) as u16, (rumble & 0xFFFF) as u16),
                dt,
                intensity: controls.intensity(),
                enabled: controls.enabled,
                clock: now.duration_since(clock0).as_secs_f32(),
            };
            let mut out = engine.update(&inputs);
            let mode = (engine.diag.active, engine.diag.paused);
            if mode != last_mode {
                log::write(match mode {
                    (true, _) => "driving: effects on",
                    (false, true) => "game paused: triggers released, vibration off",
                    (false, false) => "no live telemetry (menu or replay): triggers released",
                });
                last_mode = mode;
            }
            if let Some(f) = &feedback {
                if now < f.until {
                    if f.leds.is_some() {
                        out.player_leds = f.leds;
                    }
                    if f.lightbar.is_some() {
                        out.lightbar = f.lightbar;
                    }
                } else {
                    out.player_leds = Some(0);
                    feedback = None;
                }
            }
            let changed = last_sent.map_or(true, |s| s != out);
            if changed || now.duration_since(last_send) > Duration::from_secs(1) {
                let bytes = report::build(&out, d.info.bluetooth, seq);
                seq = seq.wrapping_add(1) & 0x0F;
                match d.write(&bytes) {
                    Ok(()) => {
                        shared.output_reports.fetch_add(1, Ordering::Relaxed);
                        last_sent = Some(out);
                        last_send = now;
                    }
                    Err(e) => {
                        log::write(&format!("controller write failed: {e}; reconnecting"));
                        dev = None;
                        continue;
                    }
                }
            }
        }

        if last_stats.elapsed() > Duration::from_secs(10) {
            last_stats = Instant::now();
            log::write(&format!(
                "stats: input {} output {} telemetry {} | brake {:?} throttle {:?} active {} paused {}",
                shared.input_reports.load(Ordering::Relaxed),
                shared.output_reports.load(Ordering::Relaxed),
                shared.telemetry_packets.load(Ordering::Relaxed),
                engine.diag.brake,
                engine.diag.throttle,
                engine.diag.active,
                engine.diag.paused
            ));
        }
    }
}

pub fn reset_controller_on_exit() {
    let Ok(guard) = RESET.try_lock() else {
        return;
    };
    if let Some(t) = guard.as_ref() {
        let bytes = report::build(&OutputState::neutral(), t.bluetooth, 0);
        device::write_blocking(&t.path, t.output_len, &bytes);
    }
}
