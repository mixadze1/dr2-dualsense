use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct Logger {
    file: Option<File>,
    start: Instant,
    enabled: bool,
}

static LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

pub fn init(path: Option<PathBuf>, enabled: bool) {
    let file = if enabled {
        path.and_then(|p| OpenOptions::new().create(true).write(true).truncate(true).open(p).ok())
    } else {
        None
    };
    if let Ok(mut g) = LOGGER.lock() {
        *g = Some(Logger { file, start: Instant::now(), enabled });
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    write(&format!("dr2-dualsense {} started, unix time {secs}", env!("CARGO_PKG_VERSION")));
}

pub fn write(msg: &str) {
    if let Ok(mut g) = LOGGER.try_lock() {
        if let Some(l) = g.as_mut() {
            if !l.enabled {
                return;
            }
            let t = l.start.elapsed().as_secs_f32();
            if let Some(f) = l.file.as_mut() {
                let _ = writeln!(f, "[{t:9.3}] {msg}");
                let _ = f.flush();
            }
        }
    }
}
