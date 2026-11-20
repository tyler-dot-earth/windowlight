// Sun position, SunCalc formulas (Vladimir Agafonkin, BSD)
const RAD: f64 = std::f64::consts::PI / 180.0;

/// (altitude, compass azimuth from north clockwise), radians
pub fn position(ms: f64, lat: f64, lon: f64) -> (f64, f64) {
    let e = RAD * 23.4397;
    let d = ms / 86_400_000.0 - 0.5 + 2440588.0 - 2451545.0;
    let (lw, phi) = (RAD * -lon, RAD * lat);
    let m = RAD * (357.5291 + 0.98560028 * d);
    let c = RAD * (1.9148 * m.sin() + 0.02 * (2.0 * m).sin() + 0.0003 * (3.0 * m).sin());
    let l = m + c + RAD * 102.9372 + std::f64::consts::PI;
    let dec = (l.sin() * e.sin()).asin();
    let ra = (l.sin() * e.cos()).atan2(l.cos());
    let h = RAD * (280.16 + 360.9856235 * d) - lw - ra;
    let az = h.sin().atan2(h.cos() * phi.sin() - dec.tan() * phi.cos());
    let alt = (phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos()).asin();
    (alt, az + std::f64::consts::PI)
}

/// direction toward the sun in the room frame for a window facing `facing_deg`
pub fn room_vec(alt: f64, az: f64, facing_deg: f64) -> [f32; 3] {
    let rel = az - facing_deg * RAD;
    [(-alt.cos() * rel.sin()) as f32, alt.sin() as f32, (alt.cos() * rel.cos()) as f32]
}

/// the light that reaches the window: the sun by day, the full-moon stand-in opposite it by night.
/// returns (altitude, compass azimuth) in radians
pub fn light(ms: f64, lat: f64, lon: f64) -> (f64, f64) {
    let (alt, az) = position(ms, lat, lon);
    if alt > 0.0 { (alt, az) } else { (-alt, az + std::f64::consts::PI) }
}

/// is the light high enough to throw a visible patch on the wall?
pub fn shows(ms: f64, lat: f64, lon: f64) -> bool { light(ms, lat, lon).0 > 8.0 * RAD }

/// a random moment on the local day of `ms` with the sun 8 to 55 degrees up (noon if none is found)
pub fn sunny_moment(rng: &mut crate::look::Rng, ms: f64, lat: f64, lon: f64) -> f64 {
    use chrono::TimeZone;
    let day = chrono::DateTime::from_timestamp_millis(ms as i64).unwrap().with_timezone(&chrono::Local).date_naive();
    let day0 = chrono::Local.from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap()).earliest().unwrap().timestamp_millis() as f64;
    for _ in 0..500 {
        let t = day0 + rng.f() * 86.4e6;
        let alt = position(t, lat, lon).0 / RAD;
        if (8.0..55.0).contains(&alt) { return t; }
    }
    day0 + 12.0 * 3.6e6
}

/// a random window direction that lets the light in at `ms`
pub fn facing_for(rng: &mut crate::look::Rng, ms: f64, lat: f64, lon: f64) -> f64 {
    (light(ms, lat, lon).1 / RAD + (rng.f() - 0.5) * 100.0).rem_euclid(360.0)
}

/// does a window facing `facing_deg` catch enough light at `ms` to show a patch?
pub fn lets_in(ms: f64, lat: f64, lon: f64, facing_deg: f64) -> bool {
    let (alt, az) = light(ms, lat, lon);
    let off = (az / RAD - facing_deg + 540.0).rem_euclid(360.0) - 180.0;
    alt > 8.0 * RAD && off.abs() < 65.0
}

/// how squarely the light meets a window leaning `tilt_deg` (either way) facing `facing_deg`: 0 edge-on, 1 head-on
pub fn squareness(ms: f64, lat: f64, lon: f64, facing_deg: f64, tilt_deg: f64) -> f64 {
    let (alt, az) = light(ms, lat, lon);
    let l = room_vec(alt, az, facing_deg);
    let t = tilt_deg * RAD;
    t.sin() * l[0] as f64 + t.cos() * l[2] as f64
}
