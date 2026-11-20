// Scene state over time: the current look, transitions between looks, gusts, the cat, favourites,
// and wake/sleep fades.
use crate::gfx::{Frame, Renderer, SceneTex};
use crate::look::{Look, Mood, Rng, CAUSTICS, HISTORIC, LAMPS, MATERIALS, MOODS, PLANTS, STYLES, TREES};
pub const SEASONS: [&str; 5] = ["spring", "summer", "autumn", "winter", "auto"];
use crate::signals::Signals;
use crate::sun;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Opts {
    pub lat: f64, pub lon: f64, pub look: Look, pub speed: f64, pub at_ms: Option<f64>, pub anim: f64,
    pub fps: f64, pub scale: f64, pub output: String, pub workspaces: Vec<usize>, pub shuffle_secs: Option<f64>, pub size: (i32, i32), pub lapse: Option<String>,
    pub cat_secs: Option<f64>, pub pool: &'static str, pub breeze: bool, pub ui_port: Option<u16>,
    /// a fixed season (index into SEASONS), or None to follow the date
    pub season: Option<usize>, pub pass_secs: Option<f64>,
    /// the folder signal files land in (None = signals off) and the map from signals to behaviours
    pub signals: Option<String>, pub signal_map: String,
}

#[derive(Clone, Copy)]
struct Tween { from: f64, to: f64, t0: Instant, dur: f64 }
impl Tween {
    fn still(v: f64) -> Tween { Tween { from: v, to: v, t0: Instant::now(), dur: 0.0 } }
    fn k(&self, now: Instant) -> f64 {
        if self.dur <= 0.0 { return 1.0; }
        smooth(secs(now, self.t0) / self.dur)
    }
    fn at(&self, now: Instant) -> f64 { self.from + (self.to - self.from) * self.k(now) }
    fn done(&self, now: Instant) -> bool { secs(now, self.t0) >= self.dur }
}
fn smooth(x: f64) -> f64 { let x = x.clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) }
fn secs(now: Instant, t0: Instant) -> f64 { now.saturating_duration_since(t0).as_secs_f64() }

/// the parts of a look that can glide from one value to the next
#[derive(Clone, Copy)]
struct Cont { mood: Mood, tilt: f64, facing: f64, rain: f64, prism: f64, lamp: f64, life: f64, shelf: f64, corner: f64, switch: f64, hook: f64 }
impl Cont {
    fn of(l: &Look) -> Cont { Cont { mood: Mood::named(l.mood), tilt: l.tilt, facing: l.facing, rain: if l.rain { 1.0 } else { 0.0 }, prism: if l.prism { 1.0 } else { 0.0 }, lamp: if l.lamp > 0 { 1.0 } else { 0.0 }, life: if l.life { 1.0 } else { 0.0 }, shelf: if l.shelf { 1.0 } else { 0.0 }, corner: if l.corner { 1.0 } else { 0.0 }, switch: if l.switch { 1.0 } else { 0.0 }, hook: if l.hook { 1.0 } else { 0.0 } } }
    fn lerp(&self, o: &Cont, k: f64) -> Cont {
        let turn = (o.facing - self.facing + 540.0).rem_euclid(360.0) - 180.0;   // the short way round
        Cont {
            mood: self.mood.lerp(&o.mood, k as f32), tilt: self.tilt + (o.tilt - self.tilt) * k,
            facing: (self.facing + turn * k).rem_euclid(360.0), rain: self.rain + (o.rain - self.rain) * k,
            prism: self.prism + (o.prism - self.prism) * k, lamp: self.lamp + (o.lamp - self.lamp) * k,
            life: self.life + (o.life - self.life) * k,
            shelf: self.shelf + (o.shelf - self.shelf) * k,
            corner: self.corner + (o.corner - self.corner) * k,
            switch: self.switch + (o.switch - self.switch) * k,
            hook: self.hook + (o.hook - self.hook) * k,
        }
    }
}

/// a cloud passing: the light fades out, the window or plant swaps unseen, and the light comes back
const CLOUD_DOWN: f64 = 1.3;
const CLOUD_HOLD: f64 = 0.35;
const CLOUD_UP: f64 = 1.8;
const CLOUD: f64 = CLOUD_DOWN + CLOUD_HOLD + CLOUD_UP;
#[derive(Clone, Copy)]
struct Cloud { t0: Instant, k0: f64, swapped: bool }
impl Cloud {
    fn light(&self, now: Instant) -> f64 {
        let e = secs(now, self.t0);
        if e < CLOUD_DOWN { self.k0 * (1.0 - smooth(e / CLOUD_DOWN)) }
        else if e < CLOUD_DOWN + CLOUD_HOLD { 0.0 }
        else { smooth((e - CLOUD_DOWN - CLOUD_HOLD) / CLOUD_UP) }
    }
}

/// one walk of the cat along the sill (window coordinates; the glass spans x -0.42..0.42)
#[derive(Clone, Copy)]
struct CatWalk { t0: Instant, dir: f64, sit_x: f64, sit_for: f64 }
const CAT_SPEED: f64 = 0.15;
const CAT_STRIDE: f64 = 0.13;
const CAT_FROM: f64 = 0.68;
impl CatWalk {
    fn new(rng: &mut Rng, now: Instant) -> CatWalk {
        let sits = rng.f() < 0.75;
        CatWalk {
            t0: now, dir: if rng.f() < 0.5 { 1.0 } else { -1.0 }, sit_x: (rng.f() - 0.5) * 0.5,
            sit_for: if sits { 4.0 + rng.f() * 7.0 } else { 0.0 },
        }
    }
    /// (x, pose, leg phase) at elapsed seconds `e`, or None once it has left
    fn at(&self, e: f64) -> Option<(f64, f64, f64)> {
        let x0 = -CAT_FROM * self.dir;
        let d_in = (self.sit_x - x0).abs();
        let t_in = d_in / CAT_SPEED;
        let (sit, settle) = (self.sit_for, if self.sit_for > 0.0 { 0.8 } else { 0.0 });
        let t_rest = t_in + settle + sit + settle;
        let d_out = (CAT_FROM * self.dir - self.sit_x).abs();
        let phase = |d: f64| d / CAT_STRIDE * std::f64::consts::TAU;
        if e < t_in {
            // slow down for the last step before sitting
            let ease = if sit > 0.0 { 1.0 - 0.35 * smooth((e - t_in + 0.8) / 0.8) } else { 1.0 };
            let d = e * CAT_SPEED * ease;
            Some((x0 + d * self.dir, 0.0, phase(d)))
        } else if e < t_rest {
            let s = e - t_in;
            let pose = smooth(s / settle.max(1e-3)) * (1.0 - smooth((s - settle - sit) / settle.max(1e-3)));
            Some((self.sit_x, if sit > 0.0 { pose } else { 0.0 }, phase(d_in)))
        } else {
            let d = (e - t_rest) * CAT_SPEED;
            if d > d_out { return None; }
            Some((self.sit_x + d * self.dir, 0.0, phase(d_in + d)))
        }
    }
}

/// something going past outside: birds across the light by day, a car's headlights by night
#[derive(Clone, Copy)]
struct Passing { t0: Instant, kind: f64, dur: f64, seed: f64, dir: f64 }
impl Passing {
    fn new(rng: &mut Rng, now: Instant, car: bool) -> Passing {
        Passing { t0: now, kind: if car { 2.0 } else { 1.0 }, dur: if car { 4.5 } else { 2.4 + rng.f() * 1.2 }, seed: rng.f(),
                  dir: if rng.f() < 0.5 { 1.0 } else { -1.0 } }
    }
    fn at(&self, now: Instant) -> Option<[f32; 4]> {
        let k = secs(now, self.t0) / self.dur;
        (k < 1.0).then_some([self.kind as f32, k as f32, self.seed as f32, self.dir as f32])
    }
}

pub struct App {
    pub o: Opts,
    start: Instant,
    base_ms: f64,
    /// where the scene is heading
    pub look: Look,
    /// the window, plant, caustic and wall material being drawn (lags `look` until the cloud covers the swap)
    drawn: Look,
    cur: SceneTex,
    c_from: Cont, c_to: Cont, c_k: Tween,
    cloud: Option<Cloud>,
    fade: Tween, warp_min: Tween,
    pub asleep: bool,
    rng: Rng,
    last_shuffle: Instant,
    lapse: Option<Lapse>,
    gust_from: f64, gust_peak: f64, gust_t0: Instant,
    cat: Option<CatWalk>, next_cat: Instant,
    passing: Option<Passing>, next_pass: Instant,
    /// lightning in the rain: when the last strike was, its side of the sky and seed, and when the next one comes
    bolt: Option<(Instant, f32, f32)>, next_bolt: Instant,
    /// a clock jump waiting for the cloud to cover it
    jump: Option<f64>,
    pub sig: Signals,
    spin: f64, last_t: Instant,
    /// the lamp last switched on, so it can fade out after the look says none
    last_lamp: usize,
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64
}

/// $XDG_CONFIG_HOME/windowlight/NAME, or ~/.config/windowlight/NAME
pub fn config_file(name: &str) -> String {
    let base = std::env::var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}/.config", std::env::var("HOME").unwrap_or_else(|_| ".".into())));
    format!("{base}/windowlight/{name}")
}
pub fn favs_path() -> String {
    std::env::var("WINDOWLIGHT_FAVS").unwrap_or_else(|_| config_file("favourites.txt"))
}
pub fn load_favs() -> Vec<String> {
    std::fs::read_to_string(favs_path()).unwrap_or_default().lines().map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && Look::from_flags(l).is_some()).collect()
}
fn save_favs(f: &[String]) -> Result<(), String> {
    let mut s = String::from("# windowlight favourites, one look per line (keep / drop add and remove them)\n");
    for l in f { s.push_str(l); s.push('\n'); }
    let p = favs_path();
    if let Some(d) = std::path::Path::new(&p).parent() { let _ = std::fs::create_dir_all(d); }
    std::fs::write(&p, s).map_err(|e| format!("{}: {e}", favs_path()))
}

/// where the sun is worked out for: "LAT LON" in the config folder, written when the page picks a place
pub fn place_path() -> String { config_file("place") }
pub fn load_place() -> Option<(f64, f64)> {
    let s = std::fs::read_to_string(place_path()).ok()?;
    let mut w = s.split_whitespace().map(|x| x.parse::<f64>());
    match (w.next()?, w.next()?) { (Ok(a), Ok(b)) if a.abs() <= 90.0 && b.abs() <= 180.0 => Some((a, b)), _ => None }
}
fn save_place(lat: f64, lon: f64) {
    let p = place_path();
    if let Some(d) = std::path::Path::new(&p).parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::write(&p, format!("{lat:.3} {lon:.3}\n"));
}

/// favourites match whatever way the window faces, since that gets re-aimed at the light
fn fav_key(l: &Look) -> String { let mut l = l.clone(); l.facing = 0.0; l.flags() }

impl App {
    pub unsafe fn new(o: Opts, r: &Renderer) -> App {
        let c = Cont::of(&o.look);
        let now = Instant::now();
        let mut a = App {
            start: now, base_ms: o.at_ms.unwrap_or_else(now_ms), look: o.look.clone(), drawn: o.look.clone(),
            cur: r.scene_tex(), c_from: c, c_to: c, c_k: Tween::still(1.0), cloud: None,
            fade: Tween::still(1.0), warp_min: Tween::still(0.0), asleep: false,
            rng: Rng::seeded(), last_shuffle: now, lapse: None,
            gust_from: 0.0, gust_peak: 0.0, gust_t0: now, cat: None, next_cat: now, passing: None, next_pass: now, bolt: None, next_bolt: now + Duration::from_secs(20), jump: None,
            sig: Signals::new(o.signals.clone(), o.signal_map.clone()), spin: 0.0, last_t: now, last_lamp: o.look.lamp, o,
        };
        // the first cat comes along a minute or three in, then about every --cat
        a.next_cat = now + Duration::from_secs_f64(60.0 + a.rng.f() * 120.0);
        a.next_pass = now + Duration::from_secs_f64(30.0 + a.rng.f() * 90.0);
        if let Some(spec) = a.o.lapse.clone() {
            if let Err(e) = a.set_lapse(&spec) { eprintln!("{e}"); std::process::exit(2); }
        }
        a
    }

    pub unsafe fn free(self, r: &Renderer) { r.drop_scene(self.cur); }

    pub fn set_lapse(&mut self, spec: &str) -> Result<(), String> {
        self.lapse = parse_lapse(spec, self.sim_ms(Instant::now()), self.o.lat, self.o.lon)?;
        self.o.lapse = self.lapse.map(|_| spec.to_string());
        Ok(())
    }

    fn sim_ms(&self, now: Instant) -> f64 {
        if let Some(l) = &self.lapse { return l.ms(now); }
        self.base_ms + secs(now, self.start) * 1000.0 * self.o.speed + self.warp_min.at(now) * 60_000.0
    }

    fn jump_to(&mut self, ms: f64, now: Instant) {
        self.base_ms = ms; self.start = now; self.warp_min = Tween::still(0.0);
    }

    fn clock(&self, now: Instant) -> String {
        chrono::DateTime::from_timestamp_millis(self.sim_ms(now) as i64).unwrap().with_timezone(&chrono::Local).format("%Y-%m-%dT%H:%M").to_string()
    }

    fn cont(&self, now: Instant) -> Cont { self.c_from.lerp(&self.c_to, self.c_k.k(now)) }
    fn light_k(&self, now: Instant) -> f64 { self.cloud.map_or(1.0, |c| c.light(now)) }

    fn gust(&self, now: Instant) -> f64 {
        let e = secs(now, self.gust_t0);
        const ATTACK: f64 = 0.5;
        if e < ATTACK { self.gust_from + (self.gust_peak - self.gust_from) * smooth(e / ATTACK) }
        else { self.gust_peak * (-(e - ATTACK) / 2.6).exp() }
    }

    fn cat_vec(&self, now: Instant) -> [f32; 4] {
        match self.cat.and_then(|c| c.at(secs(now, c.t0)).map(|p| (c, p))) {
            Some((c, (x, pose, ph))) => [x as f32, pose as f32, ph as f32, c.dir as f32],
            None => [0.0; 4],
        }
    }

    fn frame(&self, now: Instant) -> Frame {
        let c = self.cont(now);
        let (alt, az) = sun::position(self.sim_ms(now), self.o.lat, self.o.lon);
        let anim = (self.o.anim + secs(now, self.start)) % 86_400.0;
        let s = &self.sig;
        let lk = self.light_k(now) * (1.0 - 0.75 * s.amount("shade"));
        let rain = c.rain.max(s.amount("rain")) * (c.rain.max(s.amount("rain")) > 0.02) as i32 as f64;
        let mut mood = c.mood;
        // rain and a passing cloud both soften the light
        mood.haze = (mood.haze + 0.12 * rain as f32 + 0.6 * (1.0 - lk as f32) + 0.55 * s.amount("haze") as f32).min(1.0);
        let (flock, flock_n) = s.birds(now, anim);
        let l = &self.drawn;
        Frame {
            time: anim as f32, style: l.style as i32, plant: l.plant as i32, tree: l.tree as i32, caustic: l.caustic as i32, material: l.material as i32,
            tilt: c.tilt.to_radians() as f32, sun: sun::room_vec(alt, az, c.facing), sun_alt: alt as f32, evening: (sun::position(self.sim_ms(now) + 6e5, self.o.lat, self.o.lon).0 < alt) as i32 as f32,
            mood, fade: (self.fade.at(now) * self.lapse.map_or(1.0, |l| l.edge(now))) as f32,
            rain: rain as f32, gust: self.gust(now).max(0.75 * s.amount("wind")) as f32, cat: self.cat_vec(now), light_k: lk as f32,
            prism: c.prism.max((s.amount("suncatcher") * 5.0).min(1.0)) as f32, passing: self.passing.and_then(|p| p.at(now)).unwrap_or([0.0; 4]), season: self.season(now),
            flock, flock_n, lamp: if self.look.lamp > 0 { self.look.lamp } else { self.last_lamp } as i32, lamp_k: c.lamp as f32, dust: s.amount("dust") as f32, frost: s.amount("frost") as f32, spin: self.spin as f32,
            // the radiator is on through the winter, a little, whatever the signals say
            path: sun::year_path(self.sim_ms(now), self.o.lat, self.o.lon, c.facing),
            path_key: ((c.facing * 2.0).round() as i64) | (((self.o.lat * 10.0).round() as i64 + 1000) << 12) | (((self.o.lon * 10.0).round() as i64 + 2000) << 24),
            life: c.life as f32, shelf: c.shelf as f32, corner: c.corner as f32, switch: c.switch as f32, hook: c.hook as f32,
            heat: s.amount("heat").max(0.4 * self.season(now)[3] as f64) as f32,
            bolt: self.bolt.map_or([-1.0, 0.0, 0.0], |(t, side, seed)| [secs(now, t) as f32, side, seed]),
        }
    }

    fn season(&self, now: Instant) -> [f32; 4] {
        match self.o.season {
            Some(i) if i < 4 => { let mut w = [0.0; 4]; w[i] = 1.0; w }
            _ => sun::seasons(self.sim_ms(now), self.o.lat),
        }
    }

    /// birds by day, a car by night, nothing in the twilight between
    fn strike(&mut self, now: Instant) { self.bolt = Some((now, self.rng.f() as f32 - 0.5, self.rng.f() as f32 * 100.0)); }

    fn send_past(&mut self, now: Instant) -> Option<bool> {
        let alt = sun::position(self.sim_ms(now), self.o.lat, self.o.lon).0.to_degrees();
        let car = if alt > 4.0 { false } else if alt < -4.0 { true } else { return None };
        self.passing = Some(Passing::new(&mut self.rng, now, car));
        Some(car)
    }

    /// head for a new look: colours, tilt, facing and rain glide there; a new window, plant or
    /// caustic waits behind a passing cloud
    pub fn set_look(&mut self, next: Look, now: Instant) {
        if next == self.look { return; }
        let here = self.cont(now);
        let swap = self.jump.is_some() || (next.style, next.plant, next.caustic, next.material, next.tree) != (self.drawn.style, self.drawn.plant, self.drawn.caustic, self.drawn.material, self.drawn.tree);
        let dur = if swap {
            match self.cloud {
                Some(c) if !c.swapped => {}
                _ => self.cloud = Some(Cloud { t0: now, k0: self.light_k(now), swapped: false }),
            }
            (CLOUD - secs(now, self.cloud.unwrap().t0)).max(1.0)
        } else {
            let mut m = next.clone(); m.mood = self.look.mood;
            if m == self.look { 1.4 } else { 2.2 }
        };
        self.c_from = here;
        if next.lamp > 0 { self.last_lamp = next.lamp; }
        self.c_to = Cont::of(&next);
        self.c_k = Tween { from: 0.0, to: 1.0, t0: now, dur };
        self.look = next;
    }

    pub unsafe fn render(&mut self, r: &Renderer, fb: Option<glow::Framebuffer>, w: i32, h: i32) {
        let now = Instant::now();
        if let Some(m) = self.o.shuffle_secs {
            if secs(now, self.last_shuffle) > m { self.command(r, "random all"); }
        }
        if let Some(c) = &mut self.cloud {
            let e = secs(now, c.t0);
            if !c.swapped && e >= CLOUD_DOWN {
                c.swapped = true;
                self.drawn = self.look.clone();
                if let Some(t) = self.jump.take() { self.base_ms = t; self.start = now; self.warp_min = Tween::still(0.0); }
            }
            if e >= CLOUD { self.cloud = None; }
        }
        if let Some(c) = self.cat { if c.at(secs(now, c.t0)).is_none() { self.cat = None; } }
        if let Some(every) = self.o.cat_secs {
            if self.cat.is_none() && now >= self.next_cat {
                self.cat = Some(CatWalk::new(&mut self.rng, now));
                // exponential gaps, so it feels like a cat and not a timer
                let gap = (-every * (1.0 - self.rng.f()).ln()).clamp(40.0, every * 4.0);
                self.next_cat = now + Duration::from_secs_f64(gap);
            }
        }
        let day = { use chrono::Timelike; let t = chrono::DateTime::from_timestamp_millis(self.sim_ms(now) as i64).unwrap().with_timezone(&chrono::Local);
                    t.num_seconds_from_midnight() as f64 / 86_400.0 };
        self.sig.update(now, day);
        let dt = secs(now, self.last_t).min(0.5);
        self.last_t = now;
        self.spin += dt * 2.5 * self.sig.amount("suncatcher");
        for p in std::mem::take(&mut self.sig.pulses) {
            match p {
                "wind" => { self.command(r, "gust 0.9"); }
                "passing" if self.passing.is_none() => { self.send_past(now); }
                "cat" if self.cat.is_none() => self.cat = Some(CatWalk::new(&mut self.rng, now)),
                "suncatcher" => self.spin += 1.6,
                _ => {}
            }
        }
        if let Some(p) = self.passing { if p.at(now).is_none() { self.passing = None; } }
        // a storm: while it rains, a strike now and then, at exponential gaps about a minute and a half apart
        if self.c_to.rain.max(self.sig.amount("rain")) > 0.5 && now >= self.next_bolt {
            self.strike(now);
            let gap = (-90.0 * (1.0 - self.rng.f()).ln()).clamp(8.0, 360.0);
            self.next_bolt = now + Duration::from_secs_f64(gap);
        }
        if let Some(every) = self.o.pass_secs {
            if self.passing.is_none() && self.cat.is_none() && now >= self.next_pass {
                self.send_past(now);
                let gap = (-every * (1.0 - self.rng.f()).ln()).clamp(20.0, every * 4.0);
                self.next_pass = now + Duration::from_secs_f64(gap);
            }
        }
        let f = self.frame(now);
        r.draw(&mut self.cur, &f, fb, w, h, 1.0);
        if self.fade.to == 0.0 && self.fade.done(now) { self.asleep = true; }
    }

    /// pick the next shuffle look: from the favourites when the pool says so, else anything
    fn pick(&mut self, facing: Option<f64>) -> Look {
        if self.o.pool == "historic" {
            let mut l = Look::random(&mut self.rng, facing, &self.look);
            l.style = HISTORIC + self.rng.below(STYLES.len() - HISTORIC);
            return l;
        }
        if self.o.pool == "favourites" {
            let favs = load_favs();
            if !favs.is_empty() {
                let mut l = Look::from_flags(&favs[self.rng.below(favs.len())]).unwrap();
                if let Some(f) = facing { l.facing = f.round(); }
                return l;
            }
        }
        Look::random(&mut self.rng, facing, &self.look)
    }

    pub fn state(&self, now: Instant) -> String {
        let l = &self.look;
        let favs = load_favs();
        let key = fav_key(l);
        let (alt, az) = sun::position(self.sim_ms(now), self.o.lat, self.o.lon);
        let hex = |c: [f32; 3]| format!("#{:02x}{:02x}{:02x}", (c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8);
        let walls: Vec<String> = (0..MOODS.len()).map(|i| hex(Mood::named(i).wall)).collect();
        serde_json::json!({
            "look": { "style": STYLES[l.style], "plant": PLANTS[l.plant], "tree": TREES[l.tree], "mood": MOODS[l.mood], "material": MATERIALS[l.material], "caustic": CAUSTICS[l.caustic],
                      "tilt": l.tilt, "facing": l.facing, "rain": l.rain, "prism": l.prism, "life": l.life, "shelf": l.shelf, "corner": l.corner, "switch": l.switch, "hook": l.hook, "lamp": LAMPS[l.lamp] },
            "flags": l.flags(), "clock": self.clock(now), "asleep": self.asleep || self.fade.to == 0.0,
            "shuffle": self.o.shuffle_secs, "timelapse": self.o.lapse, "pool": self.o.pool,
            "cat": self.o.cat_secs, "cat_now": self.cat.is_some(), "passing": self.o.pass_secs,
            "season": SEASONS[self.o.season.unwrap_or(4)], "season_now": SEASONS[self.season(now).iter().enumerate().fold(0, |b, (i, x)| if *x > self.season(now)[b] { i } else { b })],
            "kept": favs.iter().any(|f| Look::from_flags(f).map(|x| fav_key(&x)) == Some(key.clone())),
            "favs": favs,
            "place": { "lat": self.o.lat, "lon": self.o.lon }, "ms": self.sim_ms(now),
            "sun": { "alt": alt.to_degrees().round(), "az": az.to_degrees().rem_euclid(360.0).round() },
            "signals": self.sig.state(),
            "choices": { "style": STYLES, "historic_from": HISTORIC, "plant": PLANTS, "tree": TREES, "mood": MOODS, "mood_wall": walls, "material": MATERIALS, "caustic": CAUSTICS, "lamp": LAMPS, "season": SEASONS },
        }).to_string()
    }
}

/// which looks a shuffle draws from
pub fn parse_pool(v: &str) -> Option<&'static str> {
    match v {
        "all" | "everything" => Some("all"),
        "favourites" | "favorites" | "favs" => Some("favourites"),
        "historic" | "historical" | "history" => Some("historic"),
        _ => None,
    }
}

/// "20s", "5m", "1h" or plain seconds; "off" or 0 turns it off
pub fn parse_secs(v: &str) -> Option<Option<f64>> {
    if v == "off" { return Some(None); }
    let (n, mul) = match v.chars().last()? { 's' => (&v[..v.len() - 1], 1.0), 'm' => (&v[..v.len() - 1], 60.0), 'h' => (&v[..v.len() - 1], 3600.0), _ => (v, 1.0) };
    let x = n.parse::<f64>().ok()? * mul;
    Some(if x > 0.0 { Some(x.max(3.0)) } else { None })
}

impl App {
    /// a window can lean either way; when the chosen way turns it nearly edge-on to the light, lean the other way
    fn lean_into_light(&self, l: &mut Look, now: Instant) {
        let ms = self.jump.unwrap_or_else(|| self.lapse.map(|t| t.middle()).unwrap_or_else(|| self.sim_ms(now)));
        let sq = |t: f64| sun::squareness(ms, self.o.lat, self.o.lon, l.facing, t);
        if sq(l.tilt) < 0.55 && sq(-l.tilt) > sq(l.tilt) { l.tilt = -l.tilt; }
    }

    /// aim the window at the light for a shuffle; in the dark, jump to a sunny moment first
    fn aim(&mut self, now: Instant) -> f64 {
        let (lat, lon) = (self.o.lat, self.o.lon);
        if let Some(l) = self.lapse { return sun::facing_for(&mut self.rng, l.middle(), lat, lon); }
        let ms = self.sim_ms(now);
        if !sun::shows(ms, lat, lon) {
            // the clock jumps while the cloud hides it
            let t = sun::sunny_moment(&mut self.rng, ms, lat, lon);
            self.jump = Some(t);
            return sun::facing_for(&mut self.rng, t, lat, lon);
        }
        sun::facing_for(&mut self.rng, ms, lat, lon)
    }

    /// handle one control line; returns the reply
    pub unsafe fn command(&mut self, r: &Renderer, line: &str) -> String {
        let now = Instant::now();
        let mut words = line.split_whitespace();
        let cmd = words.next().unwrap_or("");
        let rest: Vec<&str> = words.collect();
        let arg = rest.first().copied().unwrap_or("");
        match cmd {
            "random" => {
                self.last_shuffle = now;
                let facing = if arg == "all" { Some(self.aim(now)) } else { None };
                let mut next = self.pick(facing);
                self.lean_into_light(&mut next, now);
                self.set_look(next, now);
                self.look.flags()
            }
            "set" | "mood" | "material" | "style" | "plant" | "tree" | "caustic" | "tilt" | "facing" | "rain" | "prism" | "suncatcher" | "life" | "shelf" | "corner" | "switch" | "hook" | "lamp" | "season" => {
                let pairs: Vec<(String, String)> = if cmd == "set" {
                    rest.iter().filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect()
                } else {
                    vec![(cmd.to_string(), arg.to_string())]
                };
                let mut next = self.look.clone();
                for (k, v) in pairs {
                    match next.set(&k, &v) {
                        Ok(true) => {}
                        Ok(false) if k == "timelapse" => if let Err(e) = self.set_lapse(&v) { return format!("error: {e}"); },
                        Ok(false) if k == "season" => match SEASONS.iter().position(|x| *x == v) {
                            Some(i) => self.o.season = (i < 4).then_some(i),
                            None => return "error: season wants auto, spring, summer, autumn or winter".into(),
                        },
                        Ok(false) if k == "shuffle" || k == "cat" || k == "passing" => match parse_secs(&v) {
                            Some(s) if k == "shuffle" => { self.o.shuffle_secs = s; self.last_shuffle = now; }
                            Some(s) if k == "passing" => { self.o.pass_secs = s; if let Some(s) = s { self.next_pass = now + Duration::from_secs_f64(s.min(300.0) * self.rng.f()); } }
                            Some(s) => {
                                self.o.cat_secs = s;
                                if let Some(s) = s { self.next_cat = now + Duration::from_secs_f64(s.min(600.0) * self.rng.f()); }
                            }
                            None => return format!("error: {k} wants 20s, 5m, 1h or off"),
                        },
                        Ok(false) if k == "pool" => match parse_pool(&v) {
                            Some(p) => self.o.pool = p,
                            None => return "error: pool wants all, favourites or historic".into(),
                        },
                        Ok(false) => match (k.as_str(), v.parse::<f64>()) {
                            ("lat", Ok(x)) => { self.o.lat = x.clamp(-90.0, 90.0); save_place(self.o.lat, self.o.lon); }
                            ("lon", Ok(x)) => { self.o.lon = (x + 180.0).rem_euclid(360.0) - 180.0; save_place(self.o.lat, self.o.lon); }
                            ("speed", Ok(x)) => { self.base_ms = self.sim_ms(now); self.start = now; self.o.speed = x; }
                            ("fps", Ok(x)) => self.o.fps = x.max(1.0),
                            _ => return format!("error: unknown setting {k}={v}"),
                        },
                        Err(e) => return format!("error: {e}"),
                    }
                }
                self.set_look(next, now);
                self.look.flags()
            }
            "workspace" => {
                let idx: usize = arg.parse().unwrap_or(1);
                if self.o.workspaces.is_empty() { return "no workspace moods set".into(); }
                let mut next = self.look.clone();
                next.mood = self.o.workspaces[(idx.max(1) - 1) % self.o.workspaces.len()];
                self.set_look(next, now);
                format!("mood {}", MOODS[self.look.mood])
            }
            // desktop activity: a puff of wind through the leaves and the curtain
            "gust" => {
                let x: f64 = arg.parse().unwrap_or(0.6);
                let g = self.gust(now);
                self.gust_from = g;
                self.gust_peak = (g + x.clamp(0.0, 1.0) * (1.0 - 0.5 * g)).min(1.0);
                self.gust_t0 = now;
                format!("gust {:.2}", self.gust_peak)
            }
            "cat" => {
                if arg == "off" || arg == "every" || arg.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                    return self.command(r, &format!("set cat={}", if arg == "every" { rest.get(1).copied().unwrap_or("10m") } else { arg }));
                }
                if self.cat.is_some() { return "the cat is already out".into(); }
                self.cat = Some(CatWalk::new(&mut self.rng, now));
                "here comes the cat".into()
            }
            "bolt" | "lightning" => { self.strike(now); "a flash of lightning".into() }
            "bird" | "birds" | "car" | "passing" => {
                if self.passing.is_some() { return "something is already going past".into(); }
                match (cmd, self.send_past(now)) {
                    (_, None) => "too dim out for birds and too light for headlights; try at day or night".into(),
                    ("car", Some(false)) | ("bird" | "birds", Some(true)) => {
                        self.passing = None;
                        format!("no {cmd}s now: birds show by day, headlights by night")
                    }
                    (_, Some(true)) => "a car goes by".into(),
                    (_, Some(false)) => "birds fly past".into(),
                }
            }
            // play a source: `signal NAME 0.4|busy|waiting|done|alert|quiet|clear [ttl]`; `signals` lists them
            "air" => {
                use crate::signals::AIR;
                use crate::signals::Curve;
                let show = |s: &Signals| AIR.iter().map(|b| match (s.pinned.get(b), s.curves.get(b)) {
                    (Some(v), _) => format!("{b} {v:.2}"), (None, Some(_)) => format!("{b} curve"), _ => format!("{b} auto")
                }).collect::<Vec<_>>().join(" · ");
                match (rest.first().copied(), rest.get(1).copied()) {
                    (None, _) => show(&self.sig),
                    (Some("auto"), None) => { self.sig.pinned.clear(); self.sig.curves.clear(); format!("air follows the signals: {}", show(&self.sig)) }
                    (Some(name), Some(v)) => {
                        let Some(b) = AIR.iter().find(|x| **x == name) else { return format!("air wants one of {}", AIR.join(", ")) };
                        if v == "auto" { self.sig.pinned.remove(b); self.sig.curves.remove(b); }
                        else if v == "curve" {
                            // no numbers: keep the curve it has, or start from a gentle afternoon swell
                            let n: Vec<f64> = rest[2..].iter().filter_map(|x| x.parse().ok()).collect();
                            let c = match n.len() {
                                5 => Curve::new([n[0], n[1], n[2], n[3], n[4]]),
                                0 => self.sig.curves.get(b).copied().unwrap_or(Curve::new([0.1, 0.45, 1.0, 0.75, 1.0])),
                                _ => return "air NAME curve wants nothing, or Y0 X1 Y1 X2 Y2: midnight's level and two handles, x as a fraction of the day".into(),
                            };
                            self.sig.pinned.remove(b);
                            self.sig.curves.insert(b, c);
                        } else {
                            let Ok(x) = v.parse::<f64>() else { return "air NAME wants a level 0..1, curve or auto".into() };
                            self.sig.curves.remove(b);
                            self.sig.pinned.insert(b, x.clamp(0.0, 1.0));
                        }
                        show(&self.sig)
                    }
                    _ => format!("air wants NAME LEVEL (0..1, curve or auto) or auto, NAME one of {}", AIR.join(", ")),
                }
            }
            "signal" => {
                let Some(dir) = self.sig.dir.clone() else { return "signals are off (--signals DIR turns them on)".into() };
                let (Some(name), Some(v)) = (rest.first(), rest.get(1)) else { return "signal wants NAME VALUE [ttl], value a level 0..1 or busy, waiting, done, alert, quiet or clear".into() };
                match crate::signals::write(&dir, name, v, rest.get(2).and_then(|t| t.parse().ok())) {
                    Ok(m) => { let b = self.sig.behaviours_of(name); if b.is_empty() { format!("{m} (nothing in the map uses it)") } else { format!("{m} -> {}", b.join(", ")) } }
                    Err(e) => format!("error: {e}"),
                }
            }
            "signals" => {
                let s = &self.sig;
                if s.dir.is_none() { return "signals are off".into(); }
                let mut out: Vec<String> = s.live.iter().map(|x| format!("{} {} {:.2} -> {}", x.name, x.state.name(), x.level, s.behaviours_of(&x.name).join(","))).collect();
                if out.is_empty() { out.push(format!("no live signals in {}", s.dir.as_deref().unwrap_or(""))); }
                out.extend(s.map_errs.iter().cloned());
                out.join(" | ")
            }
            "keep" => {
                let mut favs = load_favs();
                let key = fav_key(&self.look);
                if favs.iter().any(|f| Look::from_flags(f).map(|x| fav_key(&x)) == Some(key.clone())) { return "already kept".into(); }
                favs.push(self.look.flags());
                if let Err(e) = save_favs(&favs) { return format!("error: {e}"); }
                format!("kept as favourite {}", favs.len())
            }
            "drop" => {
                let mut favs = load_favs();
                let key = fav_key(&self.look);
                let before = favs.len();
                match arg.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= favs.len() => { favs.remove(n - 1); }
                    Ok(_) => return "no favourite with that number".into(),
                    Err(_) => favs.retain(|f| Look::from_flags(f).map(|x| fav_key(&x)) != Some(key.clone())),
                }
                if favs.len() == before { return "this look isn't a favourite".into(); }
                if let Err(e) = save_favs(&favs) { return format!("error: {e}"); }
                format!("dropped; {} favourites left", favs.len())
            }
            "favs" => {
                let favs = load_favs();
                if favs.is_empty() { return "no favourites yet (keep saves the current look)".into(); }
                favs.iter().enumerate().map(|(i, f)| format!("{}  {f}", i + 1)).collect::<Vec<_>>().join(" | ")
            }
            // show favourite N; the window turns toward the light if it would be dark as saved
            "fav" => {
                let favs = load_favs();
                let Some(mut l) = arg.parse::<usize>().ok().and_then(|n| favs.get(n.wrapping_sub(1))).and_then(|f| Look::from_flags(f))
                    else { return format!("fav wants a number from 1 to {}", favs.len()) };
                let ms = self.sim_ms(now);
                if !sun::lets_in(ms, self.o.lat, self.o.lon, l.facing) { l.facing = self.aim(now).round(); }
                self.set_look(l, now);
                self.look.flags()
            }
            "state" => self.state(now),
            // a small still of the current frame for the control page, as base64 PNG
            "preview" => {
                let (w, h) = (640, 360);
                let c = r.canvas(w, h);
                let f = self.frame(now);
                r.draw(&mut self.cur, &f, Some(c.fb), w, h, 1.0);
                let rgb = r.read_rgb(&c);
                r.drop_target(c);
                base64(&png_bytes(w, h, &rgb))
            }
            // pictures for the control page's buttons: each window's shape and each plant's silhouette, as
            // base64 PNGs, drawn still (no rain, cat, frost or wind)
            "glyphs" => {
                let mut f = self.frame(now);
                (f.rain, f.cat, f.frost, f.gust, f.time) = (0.0, [0.0; 4], 0.0, 0.0, 0.0);
                f.season[3] = 0.0;
                let mut styles = serde_json::Map::new();
                for (i, name) in STYLES.iter().enumerate() {
                    f.style = i as i32;
                    styles.insert(name.to_string(), base64(&png_rgba(210, 238, &r.glyph(&f, false, 210, 238))).into());
                }
                let mut plants = serde_json::Map::new();
                for (i, name) in PLANTS.iter().enumerate().skip(1) {
                    f.plant = i as i32;
                    // the plant fills only part of its plane, so crop to where its leaves are
                    let (w, h) = (192usize, 180usize);
                    let px = r.glyph(&f, true, w as i32, h as i32);
                    let inked: Vec<(usize, usize)> = (0..w * h).filter(|i| px[i * 4 + 3] > 24).map(|i| (i % w, i / w)).collect();
                    let (x0, x1) = (inked.iter().map(|p| p.0).min().unwrap_or(0), inked.iter().map(|p| p.0).max().unwrap_or(w - 1));
                    let (y0, y1) = (inked.iter().map(|p| p.1).min().unwrap_or(0), inked.iter().map(|p| p.1).max().unwrap_or(h - 1));
                    let crop: Vec<u8> = (y0..=y1).flat_map(|y| px[(y * w + x0) * 4..(y * w + x1 + 1) * 4].to_vec()).collect();
                    plants.insert(name.to_string(), base64(&png_rgba((x1 - x0 + 1) as i32, (y1 - y0 + 1) as i32, &crop)).into());
                }
                (f.plant, f.season) = (0, [0.0, 1.0, 0.0, 0.0]);
                let mut trees = serde_json::Map::new();
                for (i, name) in TREES.iter().enumerate().skip(1) {
                    f.tree = i as i32;
                    trees.insert(name.to_string(), base64(&png_rgba(160, 160, &r.glyph(&f, true, 160, 160))).into());
                }
                let mut mats = serde_json::Map::new();
                for (i, name) in MATERIALS.iter().enumerate() {
                    mats.insert(name.to_string(), base64(&png_rgba(160, 128, &r.swatch(i as i32, 160, 128))).into());
                }
                serde_json::json!({ "style": styles, "plant": plants, "tree": trees, "material": mats }).to_string()
            }
            // lock screen hooks: sleep fades to dark and stops drawing; wake fades in while the sun sweeps up to now
            "sleep" => { self.fade = Tween { from: self.fade.at(now), to: 0.0, t0: now, dur: 0.8 }; "sleeping".into() }
            "wake" => {
                self.asleep = false;
                self.fade = Tween { from: 0.0, to: 1.0, t0: now, dur: 2.2 };
                self.warp_min = Tween { from: -70.0, to: 0.0, t0: now, dur: 3.2 };
                "awake".into()
            }
            "golden" => {
                use chrono::TimeZone;
                let ms = self.sim_ms(now);
                let day = chrono::DateTime::from_timestamp_millis(ms as i64).unwrap().with_timezone(&chrono::Local).date_naive();
                let day0 = chrono::Local.from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap()).earliest().unwrap().timestamp_millis() as f64;
                let evening = match arg { "sunrise" | "morning" => false, "sunset" | "evening" => true, _ => sun::position(ms, self.o.lat, self.o.lon).0 > 0.0 && sun::position(ms + 6e5, self.o.lat, self.o.lon).0 < sun::position(ms, self.o.lat, self.o.lon).0 };
                match golden(day0, self.o.lat, self.o.lon, evening) {
                    Some((a, _)) => { self.lapse = None; self.o.lapse = None; self.jump_to(a, now); format!("golden hour from {}, in real time", self.clock(now)) }
                    None => "no golden hour here today".into(),
                }
            }
            "now" => { self.lapse = None; self.o.lapse = None; self.jump_to(now_ms(), now); format!("following the real sun at {}", self.clock(now)) }
            "get" => match self.o.shuffle_secs { Some(s) => format!("{} --at {} (shuffling every {s:.0}s)", self.look.flags(), self.clock(now)), None => format!("{} --at {}", self.look.flags(), self.clock(now)) },
            _ => "commands: random [all] | set key=value ... (shuffle=20s|off, timelapse=day/2m|off, pool=all|favourites|historic, cat=10m|off, passing=4m|off, season=auto|spring|summer|autumn|winter) \
                  | mood/material/style/plant/tree/caustic/tilt/facing/rain/prism/life/shelf/corner/switch/hook/lamp/season VALUE | keep | drop [N] | favs | fav N | cat | bird | car | bolt | gust [0..1] | air [NAME 0..1|curve [Y0 X1 Y1 X2 Y2]|auto] | signal NAME VALUE [ttl] | signals | golden [sunrise|sunset] | now | sleep | wake | get | state".into(),
        }
    }
}

/// the golden hour and its twilight on the local day starting at `day0`: for sunrise from the sun 6 degrees
/// under the horizon until it is 10 degrees up, for sunset the same backwards
pub fn golden(day0: f64, lat: f64, lon: f64, evening: bool) -> Option<(f64, f64)> {
    let alt = |i: usize| sun::position(day0 + i as f64 * 6e4, lat, lon).0.to_degrees();
    let mins: Vec<usize> = (0..1440).collect();
    let noon = *mins.iter().max_by(|a, b| alt(**a).total_cmp(&alt(**b)))?;
    let (lo, hi) = (-6.0, 10.0);
    if evening {
        let a = (noon..1440).find(|i| alt(*i) < hi)?;
        let b = (a..1440).find(|i| alt(*i) < lo).unwrap_or(1439);
        Some((day0 + a as f64 * 6e4, day0 + b as f64 * 6e4))
    } else {
        let a = (0..noon).find(|i| alt(*i) > lo)?;
        let b = (a..noon).find(|i| alt(*i) > hi).unwrap_or(noon);
        Some((day0 + a as f64 * 6e4, day0 + b as f64 * 6e4))
    }
}

/// a looping time-lapse: `span_ms` of sky starting at `start_ms`, replayed every `period` seconds
#[derive(Clone, Copy)]
pub struct Lapse { start_ms: f64, span_ms: f64, period: f64, t0: Instant }

/// parse "day/2m" (sunrise to sunset), "full/5m" (midnight to midnight) or "07:00-18:00/90s"
/// on the local day of `ms`; "off" gives None
pub fn parse_lapse(spec: &str, ms: f64, lat: f64, lon: f64) -> Result<Option<Lapse>, String> {
    use chrono::TimeZone;
    if spec == "off" { return Ok(None); }
    let bad = || format!("timelapse wants day/2m, sunset/1m, sunrise/1m, full/5m, 07:00-18:00/90s or off, not {spec}");
    let (range, dur) = spec.split_once('/').ok_or_else(bad)?;
    let period = parse_secs(dur).flatten().ok_or_else(bad)?;
    let day = chrono::DateTime::from_timestamp_millis(ms as i64).unwrap().with_timezone(&chrono::Local).date_naive();
    let day0 = chrono::Local.from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap()).earliest().unwrap().timestamp_millis() as f64;
    let hm = |s: &str| -> Option<f64> {
        let (h, m) = s.split_once(':').unwrap_or((s, "0"));
        Some(h.parse::<f64>().ok()? * 3.6e6 + m.parse::<f64>().ok()? * 6e4)
    };
    let (a, b) = match range {
        "full" => (0.0, 86.4e6),
        "sunrise" | "sunset" => match golden(day0, lat, lon, range == "sunset") { Some((a, b)) => (a - day0, b - day0), None => return Err("no golden hour here today".into()) },
        "day" => {
            // first and last minute with the sun up, padded 25 minutes for the twilight on each side
            let up: Vec<f64> = (0..1440).map(|i| i as f64 * 6e4).filter(|t| sun::position(day0 + t, lat, lon).0 > 0.0).collect();
            match (up.first(), up.last()) { (Some(a), Some(b)) => (a - 1.5e6, b + 1.5e6), _ => return Err("the sun doesn't rise here today".into()) }
        }
        r => {
            let (a, b) = r.split_once('-').ok_or_else(bad)?;
            let (a, mut b) = (hm(a).ok_or_else(bad)?, hm(b).ok_or_else(bad)?);
            if b <= a { b += 86.4e6; }
            (a, b)
        }
    };
    Ok(Some(Lapse { start_ms: day0 + a, span_ms: b - a, period, t0: Instant::now() }))
}

impl Lapse {
    fn frac(&self, now: Instant) -> f64 { ((now - self.t0).as_secs_f64() / self.period).fract() }
    fn ms(&self, now: Instant) -> f64 { self.start_ms + self.frac(now) * self.span_ms }
    /// a short dip to dark where the loop wraps, so the jump back isn't a hard cut
    fn edge(&self, now: Instant) -> f64 {
        let f = self.frac(now);
        let s = |x: f64| { let x = x.clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) };
        let w = (1.2 / self.period).min(0.1);
        s(f / w) * s((1.0 - f) / w)
    }
    fn middle(&self) -> f64 { self.start_ms + self.span_ms * 0.5 }
}

fn png_bytes(w: i32, h: i32, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    if let Ok(mut wr) = enc.write_header() { let _ = wr.write_image_data(rgb); }
    out
}

fn png_rgba(w: i32, h: i32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    if let Ok(mut wr) = enc.write_header() { let _ = wr.write_image_data(rgba); }
    out
}

fn base64(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 { s.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' }); }
    }
    s
}
