// Scene state over time: the current look, crossfades between looks, mood tweens, and wake/sleep fades.
use crate::gfx::{Frame, Renderer, SceneTex};
use crate::look::{Look, Mood, Rng, MOODS};
use crate::sun;
use std::time::Instant;

#[derive(Clone)]
pub struct Opts {
    pub lat: f64, pub lon: f64, pub look: Look, pub speed: f64, pub at_ms: Option<f64>, pub anim: f64,
    pub fps: f64, pub scale: f64, pub output: String, pub workspaces: Vec<usize>, pub shuffle_secs: Option<f64>, pub size: (i32, i32), pub lapse: Option<String>,
}

#[derive(Clone, Copy)]
struct Tween { from: f64, to: f64, t0: Instant, dur: f64 }
impl Tween {
    fn still(v: f64) -> Tween { Tween { from: v, to: v, t0: Instant::now(), dur: 0.0 } }
    fn k(&self, now: Instant) -> f64 {
        if self.dur <= 0.0 { return 1.0; }
        let x = ((now - self.t0).as_secs_f64() / self.dur).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }
    fn at(&self, now: Instant) -> f64 { self.from + (self.to - self.from) * self.k(now) }
    fn done(&self, now: Instant) -> bool { (now - self.t0).as_secs_f64() >= self.dur }
}

pub struct App {
    pub o: Opts,
    start: Instant,
    base_ms: f64,
    pub look: Look,
    cur: SceneTex,
    prev: Option<(Look, Mood, SceneTex)>,
    xfade: Tween,
    mood_from: Mood, mood_to: Mood, mood_k: Tween,
    fade: Tween, warp_min: Tween,
    pub asleep: bool,
    rng: Rng,
    last_shuffle: Instant,
    lapse: Option<Lapse>,
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64
}

impl App {
    pub unsafe fn new(o: Opts, r: &Renderer) -> App {
        let m = Mood::named(o.look.mood);
        let mut a = App {
            start: Instant::now(), base_ms: o.at_ms.unwrap_or_else(now_ms), look: o.look.clone(),
            cur: r.scene_tex(), prev: None, xfade: Tween::still(1.0),
            mood_from: m, mood_to: m, mood_k: Tween::still(1.0),
            fade: Tween::still(1.0), warp_min: Tween::still(0.0), asleep: false,
            rng: Rng::seeded(), last_shuffle: Instant::now(), lapse: None, o,
        };
        if let Some(spec) = a.o.lapse.clone() {
            if let Err(e) = a.set_lapse(&spec) { eprintln!("{e}"); std::process::exit(2); }
        }
        a
    }

    pub unsafe fn free(self, r: &Renderer) {
        r.drop_scene(self.cur);
        if let Some((_, _, t)) = self.prev { r.drop_scene(t); }
    }

    pub fn set_lapse(&mut self, spec: &str) -> Result<(), String> {
        self.lapse = parse_lapse(spec, self.sim_ms(Instant::now()), self.o.lat, self.o.lon)?;
        Ok(())
    }

    fn sim_ms(&self, now: Instant) -> f64 {
        if let Some(l) = &self.lapse { return l.ms(now); }
        self.base_ms + (now - self.start).as_secs_f64() * 1000.0 * self.o.speed + self.warp_min.at(now) * 60_000.0
    }

    fn jump_to(&mut self, ms: f64, now: Instant) {
        self.base_ms = ms; self.start = now; self.warp_min = Tween::still(0.0);
    }

    fn clock(&self, now: Instant) -> String {
        chrono::DateTime::from_timestamp_millis(self.sim_ms(now) as i64).unwrap().with_timezone(&chrono::Local).format("%Y-%m-%dT%H:%M").to_string()
    }

    fn mood(&self, now: Instant) -> Mood { self.mood_from.lerp(&self.mood_to, self.mood_k.k(now) as f32) }

    fn frame(&self, look: &Look, mood: Mood, now: Instant) -> Frame {
        let (alt, az) = sun::position(self.sim_ms(now), self.o.lat, self.o.lon);
        let anim = (self.o.anim + (now - self.start).as_secs_f64()) % 86_400.0;
        Frame {
            time: anim as f32, style: look.style as i32, plant: look.plant as i32, caustic: look.caustic as i32,
            tilt: (look.tilt.to_radians()) as f32, sun: sun::room_vec(alt, az, look.facing), sun_alt: alt as f32,
            mood, fade: (self.fade.at(now) * self.lapse.map_or(1.0, |l| l.edge(now))) as f32,
        }
    }

    /// change the look: a mood-only change tweens the colours, anything else crossfades the whole scene
    pub unsafe fn set_look(&mut self, r: &Renderer, next: Look, now: Instant) {
        if next == self.look { return; }
        let cur_mood = self.mood(now);
        self.mood_from = cur_mood;
        self.mood_to = Mood::named(next.mood);
        let mut only_mood = next.clone();
        only_mood.mood = self.look.mood;
        if only_mood == self.look {
            self.mood_k = Tween { from: 0.0, to: 1.0, t0: now, dur: 1.4 };
        } else {
            if let Some((_, _, old)) = self.prev.take() { r.drop_scene(old); }
            let old = std::mem::replace(&mut self.cur, r.scene_tex());
            self.prev = Some((self.look.clone(), cur_mood, old));
            self.xfade = Tween { from: 0.0, to: 1.0, t0: now, dur: 1.8 };
            self.mood_from = self.mood_to;
            self.mood_k = Tween::still(1.0);
        }
        self.look = next;
    }

    pub unsafe fn render(&mut self, r: &Renderer, fb: Option<glow::Framebuffer>, w: i32, h: i32) {
        let now = Instant::now();
        if let Some(m) = self.o.shuffle_secs {
            if (now - self.last_shuffle).as_secs_f64() > m { self.command(r, "random all"); }
        }
        if self.prev.is_some() && self.xfade.done(now) {
            let (_, _, old) = self.prev.take().unwrap();
            r.drop_scene(old);
        }
        let f = self.frame(&self.look.clone(), self.mood(now), now);
        let pf = self.prev.as_ref().map(|(pl, pm, _)| self.frame(pl, *pm, now));
        match (pf, self.prev.as_mut()) {
            (Some(pf), Some((_, _, ptex))) => {
                r.draw(ptex, &pf, fb, w, h, 1.0);
                r.draw(&mut self.cur, &f, fb, w, h, self.xfade.at(now) as f32);
            }
            _ => r.draw(&mut self.cur, &f, fb, w, h, 1.0),
        }
        if self.fade.to == 0.0 && self.fade.done(now) { self.asleep = true; }
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
    /// handle one control line; returns the reply
    pub unsafe fn command(&mut self, r: &Renderer, line: &str) -> String {
        let now = Instant::now();
        let mut words = line.split_whitespace();
        let cmd = words.next().unwrap_or("");
        let rest: Vec<&str> = words.collect();
        match cmd {
            "random" => {
                self.last_shuffle = now;
                let facing = if rest.first() == Some(&"all") {
                    // too dark or too grazing to see anything: jump to a sunny moment today (ctl now goes back)
                    // during a time-lapse, aim the window at the light halfway through the loop
                    let (lat, lon) = (self.o.lat, self.o.lon);
                    if let Some(l) = self.lapse {
                        Some(sun::facing_for(&mut self.rng, l.middle(), lat, lon))
                    } else {
                        let ms = self.sim_ms(now);
                        if !sun::shows(ms, lat, lon) {
                            let t = sun::sunny_moment(&mut self.rng, ms, lat, lon);
                            self.jump_to(t, now);
                        }
                        let ms = self.sim_ms(now);
                        Some(sun::facing_for(&mut self.rng, ms, lat, lon))
                    }
                } else { None };
                let next = Look::random(&mut self.rng, facing, &self.look);
                self.set_look(r, next, now);
                self.look.flags()
            }
            "set" | "mood" | "style" | "plant" | "caustic" => {
                let pairs: Vec<(String, String)> = if cmd == "set" {
                    rest.iter().filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))).collect()
                } else {
                    vec![(cmd.to_string(), rest.first().unwrap_or(&"").to_string())]
                };
                let mut next = self.look.clone();
                for (k, v) in pairs {
                    match next.set(&k, &v) {
                        Ok(true) => {}
                        Ok(false) if k == "timelapse" => if let Err(e) = self.set_lapse(&v) { return format!("error: {e}"); },
                        Ok(false) if k == "shuffle" => match parse_secs(&v) {
                            Some(s) => { self.o.shuffle_secs = s; self.last_shuffle = now; }
                            None => return format!("error: shuffle wants 20s, 5m, 1h or off"),
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
                self.set_look(r, next, now);
                self.look.flags()
            }
            "workspace" => {
                let idx: usize = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1);
                if self.o.workspaces.is_empty() { return "no workspace moods set".into(); }
                let mut next = self.look.clone();
                next.mood = self.o.workspaces[(idx.max(1) - 1) % self.o.workspaces.len()];
                self.set_look(r, next, now);
                format!("mood {}", MOODS[self.look.mood])
            }
            // lock screen hooks: sleep fades to dark and stops drawing; wake fades in while the sun sweeps up to now
            "sleep" => { self.fade = Tween { from: self.fade.at(now), to: 0.0, t0: now, dur: 0.8 }; "sleeping".into() }
            "wake" => {
                self.asleep = false;
                self.fade = Tween { from: 0.0, to: 1.0, t0: now, dur: 2.2 };
                self.warp_min = Tween { from: -70.0, to: 0.0, t0: now, dur: 3.2 };
                "awake".into()
            }
            "now" => { self.lapse = None; self.jump_to(now_ms(), now); format!("following the real sun at {}", self.clock(now)) }
            "get" => match self.o.shuffle_secs { Some(s) => format!("{} --at {} (shuffling every {s:.0}s)", self.look.flags(), self.clock(now)), None => format!("{} --at {}", self.look.flags(), self.clock(now)) },
            _ => "commands: random [all] | set key=value ... (shuffle=20s|off, timelapse=day/2m|off) | mood/style/plant/caustic NAME | now | sleep | wake | get".into(),
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
