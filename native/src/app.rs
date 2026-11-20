// Scene state over time: the current look, transitions between looks, gusts, the cat, favourites,
// and wake/sleep fades.
use crate::gfx::{Frame, Renderer, SceneTex};
use crate::look::{Look, Mood, Rng, CAUSTICS, HISTORIC, MOODS, PLANTS, STYLES};
use crate::sun;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Opts {
    pub lat: f64, pub lon: f64, pub look: Look, pub speed: f64, pub at_ms: Option<f64>, pub anim: f64,
    pub fps: f64, pub scale: f64, pub output: String, pub workspaces: Vec<usize>, pub shuffle_secs: Option<f64>, pub size: (i32, i32), pub lapse: Option<String>,
    pub cat_secs: Option<f64>, pub pool: &'static str, pub breeze: bool, pub ui_port: Option<u16>,
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
struct Cont { mood: Mood, tilt: f64, facing: f64, rain: f64 }
impl Cont {
    fn of(l: &Look) -> Cont { Cont { mood: Mood::named(l.mood), tilt: l.tilt, facing: l.facing, rain: if l.rain { 1.0 } else { 0.0 } } }
    fn lerp(&self, o: &Cont, k: f64) -> Cont {
        let turn = (o.facing - self.facing + 540.0).rem_euclid(360.0) - 180.0;   // the short way round
        Cont {
            mood: self.mood.lerp(&o.mood, k as f32), tilt: self.tilt + (o.tilt - self.tilt) * k,
            facing: (self.facing + turn * k).rem_euclid(360.0), rain: self.rain + (o.rain - self.rain) * k,
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

pub struct App {
    pub o: Opts,
    start: Instant,
    base_ms: f64,
    /// where the scene is heading
    pub look: Look,
    /// the window, plant and caustic being drawn (lags `look` until the cloud covers the swap)
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
    /// a clock jump waiting for the cloud to cover it
    jump: Option<f64>,
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64
}

pub fn favs_path() -> String {
    std::env::var("WINDOWLIGHT_FAVS").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/favourites.txt").into())
}
pub fn load_favs() -> Vec<String> {
    std::fs::read_to_string(favs_path()).unwrap_or_default().lines().map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && Look::from_flags(l).is_some()).collect()
}
fn save_favs(f: &[String]) -> Result<(), String> {
    let mut s = String::from("# windowlight favourites, one look per line (keep / drop add and remove them)\n");
    for l in f { s.push_str(l); s.push('\n'); }
    std::fs::write(favs_path(), s).map_err(|e| format!("{}: {e}", favs_path()))
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
            gust_from: 0.0, gust_peak: 0.0, gust_t0: now, cat: None, next_cat: now, jump: None, o,
        };
        // the first cat comes along a minute or three in, then about every --cat
        a.next_cat = now + Duration::from_secs_f64(60.0 + a.rng.f() * 120.0);
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
        let lk = self.light_k(now);
        let mut mood = c.mood;
        // rain and a passing cloud both soften the light
        mood.haze = (mood.haze + 0.12 * c.rain as f32 + 0.6 * (1.0 - lk as f32)).min(1.0);
        let l = &self.drawn;
        Frame {
            time: anim as f32, style: l.style as i32, plant: l.plant as i32, caustic: l.caustic as i32,
            tilt: c.tilt.to_radians() as f32, sun: sun::room_vec(alt, az, c.facing), sun_alt: alt as f32,
            mood, fade: (self.fade.at(now) * self.lapse.map_or(1.0, |l| l.edge(now))) as f32,
            rain: c.rain as f32, gust: self.gust(now) as f32, cat: self.cat_vec(now), light_k: lk as f32,
        }
    }

    /// head for a new look: colours, tilt, facing and rain glide there; a new window, plant or
    /// caustic waits behind a passing cloud
    pub fn set_look(&mut self, next: Look, now: Instant) {
        if next == self.look { return; }
        let here = self.cont(now);
        let swap = self.jump.is_some() || (next.style, next.plant, next.caustic) != (self.drawn.style, self.drawn.plant, self.drawn.caustic);
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
        serde_json::json!({
            "look": { "style": STYLES[l.style], "plant": PLANTS[l.plant], "mood": MOODS[l.mood], "caustic": CAUSTICS[l.caustic],
                      "tilt": l.tilt, "facing": l.facing, "rain": l.rain },
            "flags": l.flags(), "clock": self.clock(now), "asleep": self.asleep || self.fade.to == 0.0,
            "shuffle": self.o.shuffle_secs, "timelapse": self.o.lapse, "pool": self.o.pool,
            "cat": self.o.cat_secs, "cat_now": self.cat.is_some(),
            "kept": favs.iter().any(|f| Look::from_flags(f).map(|x| fav_key(&x)) == Some(key.clone())),
            "favs": favs,
            "choices": { "style": STYLES, "historic_from": HISTORIC, "plant": PLANTS, "mood": MOODS, "caustic": CAUSTICS },
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
    pub unsafe fn command(&mut self, _r: &Renderer, line: &str) -> String {
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
            "set" | "mood" | "style" | "plant" | "caustic" | "tilt" | "facing" | "rain" => {
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
                        Ok(false) if k == "shuffle" || k == "cat" => match parse_secs(&v) {
                            Some(s) if k == "shuffle" => { self.o.shuffle_secs = s; self.last_shuffle = now; }
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
                            ("lat", Ok(x)) => self.o.lat = x,
                            ("lon", Ok(x)) => self.o.lon = x,
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
                    return self.command(_r, &format!("set cat={}", if arg == "every" { rest.get(1).copied().unwrap_or("10m") } else { arg }));
                }
                if self.cat.is_some() { return "the cat is already out".into(); }
                self.cat = Some(CatWalk::new(&mut self.rng, now));
                "here comes the cat".into()
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
            // lock screen hooks: sleep fades to dark and stops drawing; wake fades in while the sun sweeps up to now
            "sleep" => { self.fade = Tween { from: self.fade.at(now), to: 0.0, t0: now, dur: 0.8 }; "sleeping".into() }
            "wake" => {
                self.asleep = false;
                self.fade = Tween { from: 0.0, to: 1.0, t0: now, dur: 2.2 };
                self.warp_min = Tween { from: -70.0, to: 0.0, t0: now, dur: 3.2 };
                "awake".into()
            }
            "now" => { self.lapse = None; self.o.lapse = None; self.jump_to(now_ms(), now); format!("following the real sun at {}", self.clock(now)) }
            "get" => match self.o.shuffle_secs { Some(s) => format!("{} --at {} (shuffling every {s:.0}s)", self.look.flags(), self.clock(now)), None => format!("{} --at {}", self.look.flags(), self.clock(now)) },
            _ => "commands: random [all] | set key=value ... (shuffle=20s|off, timelapse=day/2m|off, pool=all|favourites|historic, cat=10m|off) \
                  | mood/style/plant/caustic/tilt/facing/rain VALUE | keep | drop [N] | favs | fav N | cat | gust [0..1] | now | sleep | wake | get | state".into(),
        }
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
    let bad = || format!("timelapse wants day/2m, full/5m, 07:00-18:00/90s or off, not {spec}");
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
