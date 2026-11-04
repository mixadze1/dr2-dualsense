pub const FIELD_COUNT_EXTRADATA3: usize = 66;
pub const FIELD_COUNT_BASIC: usize = 38;

pub const IDX_TIME: usize = 0;
pub const IDX_SPEED: usize = 7;
pub const IDX_SUSP_POS: usize = 17;
pub const IDX_SUSP_VEL: usize = 21;
pub const IDX_WHEEL_SPEED: usize = 25;
pub const IDX_THROTTLE: usize = 29;
pub const IDX_STEER: usize = 30;
pub const IDX_BRAKE: usize = 31;
pub const IDX_CLUTCH: usize = 32;
pub const IDX_GEAR: usize = 33;
pub const IDX_G_LAT: usize = 34;
pub const IDX_G_LON: usize = 35;
pub const IDX_RPM: usize = 37;
pub const IDX_MAX_RPM: usize = 63;
pub const IDX_IDLE_RPM: usize = 64;
pub const IDX_MAX_GEARS: usize = 65;

pub const RL: usize = 0;
pub const RR: usize = 1;
pub const FL: usize = 2;
pub const FR: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Telemetry {
    pub time: f32,
    pub speed: f32,
    pub susp_pos: [f32; 4],
    pub susp_vel: [f32; 4],
    pub wheel_speed: [f32; 4],
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
    pub clutch: f32,
    pub gear: f32,
    pub g_lat: f32,
    pub g_lon: f32,
    pub rpm: f32,
    pub max_rpm: f32,
    pub idle_rpm: f32,
    pub max_gears: f32,
}

fn sanitize(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

impl Telemetry {
    pub fn parse(buf: &[u8]) -> Option<Telemetry> {
        if buf.len() < FIELD_COUNT_BASIC * 4 {
            return None;
        }
        let count = buf.len() / 4;
        let f = |i: usize| -> f32 {
            if i < count {
                sanitize(f32::from_le_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]))
            } else {
                0.0
            }
        };
        let arr = |base: usize| [f(base), f(base + 1), f(base + 2), f(base + 3)];
        Some(Telemetry {
            time: f(IDX_TIME),
            speed: f(IDX_SPEED),
            susp_pos: arr(IDX_SUSP_POS),
            susp_vel: arr(IDX_SUSP_VEL),
            wheel_speed: arr(IDX_WHEEL_SPEED),
            throttle: f(IDX_THROTTLE).clamp(0.0, 1.0),
            steer: f(IDX_STEER).clamp(-1.0, 1.0),
            brake: f(IDX_BRAKE).clamp(0.0, 1.0),
            clutch: f(IDX_CLUTCH).clamp(0.0, 1.0),
            gear: f(IDX_GEAR),
            g_lat: f(IDX_G_LAT),
            g_lon: f(IDX_G_LON),
            rpm: f(IDX_RPM) * 10.0,
            max_rpm: f(IDX_MAX_RPM) * 10.0,
            idle_rpm: f(IDX_IDLE_RPM) * 10.0,
            max_gears: f(IDX_MAX_GEARS),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut fields = [0f32; FIELD_COUNT_EXTRADATA3];
        fields[IDX_TIME] = self.time;
        fields[IDX_SPEED] = self.speed;
        fields[IDX_SUSP_POS..IDX_SUSP_POS + 4].copy_from_slice(&self.susp_pos);
        fields[IDX_SUSP_VEL..IDX_SUSP_VEL + 4].copy_from_slice(&self.susp_vel);
        fields[IDX_WHEEL_SPEED..IDX_WHEEL_SPEED + 4].copy_from_slice(&self.wheel_speed);
        fields[IDX_THROTTLE] = self.throttle;
        fields[IDX_STEER] = self.steer;
        fields[IDX_BRAKE] = self.brake;
        fields[IDX_CLUTCH] = self.clutch;
        fields[IDX_GEAR] = self.gear;
        fields[IDX_G_LAT] = self.g_lat;
        fields[IDX_G_LON] = self.g_lon;
        fields[IDX_RPM] = self.rpm / 10.0;
        fields[IDX_MAX_RPM] = self.max_rpm / 10.0;
        fields[IDX_IDLE_RPM] = self.idle_rpm / 10.0;
        fields[IDX_MAX_GEARS] = self.max_gears;
        fields.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    pub fn forward_gear(&self) -> Option<i32> {
        let g = self.gear.round() as i32;
        if (1..=9).contains(&g) {
            Some(g)
        } else {
            None
        }
    }

    pub fn rpm_fraction(&self) -> Option<f32> {
        if self.max_rpm <= 100.0 {
            return None;
        }
        let idle = if self.idle_rpm > 0.0 && self.idle_rpm < self.max_rpm { self.idle_rpm } else { 0.0 };
        Some(((self.rpm - idle) / (self.max_rpm - idle)).clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Telemetry {
        Telemetry {
            time: 12.5,
            speed: 27.0,
            susp_pos: [1.0, 2.0, 3.0, 4.0],
            susp_vel: [0.1, -0.2, 0.3, -0.4],
            wheel_speed: [30.0, 31.0, 27.0, 27.5],
            throttle: 0.75,
            steer: -0.25,
            brake: 0.0,
            clutch: 0.0,
            gear: 3.0,
            g_lat: 0.5,
            g_lon: -0.2,
            rpm: 6500.0,
            max_rpm: 8000.0,
            idle_rpm: 1000.0,
            max_gears: 6.0,
        }
    }

    #[test]
    fn roundtrip_extradata3() {
        let t = sample();
        let bytes = t.to_bytes();
        assert_eq!(bytes.len(), 264);
        let p = Telemetry::parse(&bytes).unwrap();
        assert_eq!(p, t);
    }

    #[test]
    fn field_offsets_match_codemasters_layout() {
        let bytes = sample().to_bytes();
        let at = |off: usize| f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
        assert_eq!(at(28), 27.0);
        assert_eq!(at(100), 30.0);
        assert_eq!(at(116), 0.75);
        assert_eq!(at(120), -0.25);
        assert_eq!(at(124), 0.0);
        assert_eq!(at(132), 3.0);
        assert_eq!(at(148), 650.0);
        assert_eq!(at(252), 800.0);
    }

    #[test]
    fn short_packet_rejected() {
        assert!(Telemetry::parse(&[0u8; 100]).is_none());
    }

    #[test]
    fn basic_packet_without_extradata() {
        let bytes = sample().to_bytes();
        let p = Telemetry::parse(&bytes[..FIELD_COUNT_BASIC * 4]).unwrap();
        assert_eq!(p.speed, 27.0);
        assert_eq!(p.max_rpm, 0.0);
        assert!(p.rpm_fraction().is_none());
    }

    #[test]
    fn nan_values_are_sanitized() {
        let mut bytes = sample().to_bytes();
        bytes[28..32].copy_from_slice(&f32::NAN.to_le_bytes());
        bytes[116..120].copy_from_slice(&f32::INFINITY.to_le_bytes());
        let p = Telemetry::parse(&bytes).unwrap();
        assert_eq!(p.speed, 0.0);
        assert_eq!(p.throttle, 0.0);
    }

    #[test]
    fn pedals_are_clamped() {
        let mut t = sample();
        t.throttle = 3.0;
        t.brake = -1.0;
        let p = Telemetry::parse(&t.to_bytes()).unwrap();
        assert_eq!(p.throttle, 1.0);
        assert_eq!(p.brake, 0.0);
    }

    #[test]
    fn gears() {
        let mut t = sample();
        assert_eq!(t.forward_gear(), Some(3));
        t.gear = 0.0;
        assert_eq!(t.forward_gear(), None);
        t.gear = -1.0;
        assert_eq!(t.forward_gear(), None);
        t.gear = 10.0;
        assert_eq!(t.forward_gear(), None);
    }

    #[test]
    fn rpm_fraction_uses_idle() {
        let t = sample();
        let f = t.rpm_fraction().unwrap();
        assert!((f - 5500.0 / 7000.0).abs() < 1e-4);
    }
}
