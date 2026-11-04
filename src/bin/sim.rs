use std::net::UdpSocket;
use std::time::{Duration, Instant};

use xinput9_1_0::telemetry::Telemetry;

struct Phase {
    name: &'static str,
    seconds: f32,
    make: fn(f32, f32) -> Telemetry,
}

fn base(speed: f32, rpm: f32, gear: f32) -> Telemetry {
    Telemetry {
        speed,
        wheel_speed: [speed; 4],
        gear,
        rpm,
        max_rpm: 8000.0,
        idle_rpm: 900.0,
        max_gears: 6.0,
        ..Default::default()
    }
}

fn phases() -> Vec<Phase> {
    vec![
        Phase { name: "idle in neutral", seconds: 2.0, make: |_, _| base(0.0, 900.0, 0.0) },
        Phase {
            name: "smooth cruise (both triggers resist)",
            seconds: 3.0,
            make: |_, _| {
                let mut t = base(20.0, 4200.0, 3.0);
                t.throttle = 0.4;
                t
            },
        },
        Phase {
            name: "gravel road texture (right motor)",
            seconds: 4.0,
            make: |p, _| {
                let mut t = base(22.0, 4500.0, 3.0);
                t.throttle = 0.5;
                let s = (p * 37.0).sin() * 0.35 + (p * 91.0).sin() * 0.15;
                t.susp_vel = [s, -s * 0.8, s * 0.9, -s];
                t
            },
        },
        Phase {
            name: "launch with wheelspin (R2 vibrates)",
            seconds: 3.0,
            make: |p, _| {
                let speed = 2.0 + p * 6.0;
                let mut t = base(speed, 6000.0, 1.0);
                t.throttle = 1.0;
                t.wheel_speed = [speed * 2.2, speed * 2.2, speed, speed];
                t
            },
        },
        Phase {
            name: "rev limiter (R2 pulses, lightbar flashes)",
            seconds: 2.5,
            make: |p, _| {
                let mut t = base(15.0, 7900.0 + (p * 60.0).sin() * 60.0, 2.0);
                t.throttle = 1.0;
                t
            },
        },
        Phase {
            name: "upshifts 2-3-4-5 (left motor kicks)",
            seconds: 4.0,
            make: |p, _| {
                let gear = 2.0 + (p * 4.0).floor().min(3.0);
                let mut t = base(20.0 + p * 20.0, 4000.0 + (p * 4.0).fract() * 3000.0, gear);
                t.throttle = 1.0;
                t
            },
        },
        Phase {
            name: "hard braking with front lockup (L2 vibrates)",
            seconds: 3.0,
            make: |p, _| {
                let speed = 35.0 - p * 25.0;
                let mut t = base(speed, 3000.0, 4.0);
                t.brake = 1.0;
                t.wheel_speed = [speed * 0.9, speed * 0.9, speed * 0.2, speed * 0.2];
                t
            },
        },
        Phase {
            name: "jump and heavy landing (big impact)",
            seconds: 3.0,
            make: |p, _| {
                let mut t = base(25.0, 5000.0, 4.0);
                t.throttle = 0.6;
                if (0.45..0.5).contains(&p) {
                    t.susp_vel = [2.8; 4];
                    t.g_lon = -4.0;
                }
                t
            },
        },
        Phase { name: "stopped", seconds: 2.0, make: |_, _| base(0.0, 900.0, 0.0) },
    ]
}

fn main() {
    let mut port = 20777u16;
    let mut loops = 1u32;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => port = args.next().and_then(|v| v.parse().ok()).unwrap_or(port),
            "--loops" => loops = args.next().and_then(|v| v.parse().ok()).unwrap_or(loops),
            "--list" => {
                for p in phases() {
                    println!("{:>5.1}s  {}", p.seconds, p.name);
                }
                return;
            }
            _ => {
                eprintln!("usage: dr2ds-sim [--port 20777] [--loops N] [--list]");
                return;
            }
        }
    }
    let socket = UdpSocket::bind("127.0.0.1:0").expect("bind");
    let start = Instant::now();
    let mut sent = 0u64;
    for _ in 0..loops {
        for phase in phases() {
            println!("> {}", phase.name);
            let t0 = Instant::now();
            loop {
                let elapsed = t0.elapsed().as_secs_f32();
                if elapsed >= phase.seconds {
                    break;
                }
                let mut t = (phase.make)(elapsed / phase.seconds, elapsed);
                t.time = start.elapsed().as_secs_f32();
                let _ = socket.send_to(&t.to_bytes(), ("127.0.0.1", port));
                sent += 1;
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    println!("sent {sent} packets to 127.0.0.1:{port}");
}
