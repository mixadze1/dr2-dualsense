use xinput9_1_0::config::Config;
use xinput9_1_0::dualsense::report::{build_bt, build_usb, verify_bt_output, OFFSET_LEFT_TRIGGER, OFFSET_RIGHT_TRIGGER};
use xinput9_1_0::dualsense::trigger::{MODE_FEEDBACK, MODE_OFF, MODE_VIBRATION};
use xinput9_1_0::effects::{Engine, Inputs};
use xinput9_1_0::telemetry::Telemetry;

fn packet(speed: f32, wheels: [f32; 4], throttle: f32, brake: f32, gear: f32, rpm: f32) -> Vec<u8> {
    Telemetry {
        speed,
        wheel_speed: wheels,
        throttle,
        brake,
        gear,
        rpm,
        max_rpm: 7500.0,
        idle_rpm: 800.0,
        max_gears: 6.0,
        ..Default::default()
    }
    .to_bytes()
}

fn step(engine: &mut Engine, bytes: &[u8]) -> xinput9_1_0::dualsense::report::OutputState {
    let t = Telemetry::parse(bytes).expect("packet parses");
    engine.update(&Inputs { telemetry: Some(&t), telemetry_age: 0.0, game_rumble: (0, 0), dt: 0.01, intensity: 1.0, enabled: true, clock: 0.0 })
}

#[test]
fn udp_bytes_to_bluetooth_report() {
    let mut engine = Engine::new(Config::default());
    let out = step(&mut engine, &packet(20.0, [20.0; 4], 0.3, 0.0, 3.0, 4000.0));
    let r = build_bt(&out, 5);
    assert!(verify_bt_output(&r));
    assert_eq!(r[3 + OFFSET_LEFT_TRIGGER], MODE_FEEDBACK);
    assert_eq!(r[3 + OFFSET_RIGHT_TRIGGER], MODE_FEEDBACK);

    let out = step(&mut engine, &packet(25.0, [24.0, 24.0, 3.0, 3.0], 0.0, 1.0, 3.0, 3000.0));
    let r = build_bt(&out, 6);
    assert!(verify_bt_output(&r));
    assert_eq!(r[3 + OFFSET_LEFT_TRIGGER], MODE_VIBRATION);

    let out = step(&mut engine, &packet(15.0, [15.0; 4], 1.0, 0.0, 2.0, 7450.0));
    let r = build_usb(&out);
    assert_eq!(r[1 + OFFSET_RIGHT_TRIGGER], MODE_VIBRATION);
}

#[test]
fn leaving_the_stage_releases_triggers() {
    let mut engine = Engine::new(Config::default());
    step(&mut engine, &packet(20.0, [20.0; 4], 0.3, 0.0, 3.0, 4000.0));
    let out = engine.update(&Inputs { telemetry: None, telemetry_age: f32::MAX, game_rumble: (0, 0), dt: 0.01, intensity: 1.0, enabled: true, clock: 0.0 });
    let r = build_usb(&out);
    assert_eq!(r[1 + OFFSET_LEFT_TRIGGER], MODE_OFF);
    assert_eq!(r[1 + OFFSET_RIGHT_TRIGGER], MODE_OFF);
    assert_eq!(r[3], 0);
    assert_eq!(r[4], 0);
}

#[test]
fn full_stage_never_panics_and_stays_in_range() {
    let mut engine = Engine::new(Config::default());
    let mut seed = 0x1234_5678u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    for i in 0..20_000 {
        let speed = (i as f32 * 0.01).sin().abs() * 45.0;
        let t = Telemetry {
            speed,
            wheel_speed: [speed + rnd() * 8.0, speed + rnd() * 8.0, speed + rnd() * 8.0, speed + rnd() * 8.0],
            susp_vel: [rnd() * 3.0, rnd() * 3.0, rnd() * 3.0, rnd() * 3.0],
            throttle: rnd().abs(),
            brake: rnd().max(0.0),
            gear: (rnd().abs() * 7.0).floor() - 1.0,
            rpm: rnd().abs() * 9000.0,
            max_rpm: 8000.0,
            idle_rpm: 900.0,
            g_lat: rnd() * 4.0,
            g_lon: rnd() * 4.0,
            ..Default::default()
        };
        let out = engine.update(&Inputs { telemetry: Some(&t), telemetry_age: 0.0, game_rumble: (0, 0), dt: 0.008, intensity: 1.0, enabled: true, clock: i as f32 * 0.008 });
        assert!(matches!(out.left_trigger.mode(), MODE_FEEDBACK | MODE_VIBRATION | MODE_OFF));
        assert!(matches!(out.right_trigger.mode(), MODE_FEEDBACK | MODE_VIBRATION | MODE_OFF));
        assert!(verify_bt_output(&build_bt(&out, (i & 0xF) as u8)));
    }
}
