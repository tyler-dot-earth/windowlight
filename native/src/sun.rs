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
