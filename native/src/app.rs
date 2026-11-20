// Scene state over time: the current look, crossfades between looks, mood tweens, and wake/sleep fades.
use crate::gfx::{Frame, Renderer, SceneTex};
use crate::look::{Look, Mood, Rng, MOODS};
use crate::sun;
use std::time::Instant;

#[derive(Clone)]
pub struct Opts {
    pub lat: f64, pub lon: f64, pub look: Look, pub speed: f64, pub at_ms: Option<f64>, pub anim: f64,
    pub fps: f64, pub scale: f64, pub output: String, pub workspaces: Vec<usize>, pub shuffle_secs: Option<f64>, pub size: (i32, i32),
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
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64
}

impl App {
    pub unsafe fn new(o: Opts, r: &Renderer) -> App {
        let m = Mood::named(o.look.mood);
        App {
            start: Instant::now(), base_ms: o.at_ms.unwrap_or_else(now_ms), look: o.look.clone(),
            cur: r.scene_tex(), prev: None, xfade: Tween::still(1.0),
            mood_from: m, mood_to: m, mood_k: Tween::still(1.0),
            fade: Tween::still(1.0), warp_min: Tween::still(0.0), asleep: false,
            rng: Rng::seeded(), last_shuffle: Instant::now(), o,
        }
    }

    pub unsafe fn free(self, r: &Renderer) {
        r.drop_scene(self.cur);
        if let Some((_, _, t)) = self.prev { r.drop_scene(t); }
    }

    fn sim_ms(&self, now: Instant) -> f64 {
        self.base_ms + (now - self.start).as_secs_f64() * 1000.0 * self.o.speed + self.warp_min.at(now) * 60_000.0
    }

    fn mood(&self, now: Instant) -> Mood { self.mood_from.lerp(&self.mood_to, self.mood_k.k(now) as f32) }

    fn frame(&self, look: &Look, mood: Mood, now: Instant) -> Frame {
        let (alt, az) = sun::position(self.sim_ms(now), self.o.lat, self.o.lon);
        let anim = (self.o.anim + (now - self.start).as_secs_f64()) % 86_400.0;
        Frame {
            time: anim as f32, style: look.style as i32, plant: look.plant as i32, caustic: look.caustic as i32,
            tilt: (look.tilt.to_radians()) as f32, sun: sun::room_vec(alt, az, look.facing), sun_alt: alt as f32,
            mood, fade: self.fade.at(now) as f32,
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
                    let (alt, az) = sun::position(self.sim_ms(now), self.o.lat, self.o.lon);
                    Some(if alt > 0.0 { az.to_degrees() + (self.rng.f() - 0.5) * 80.0 } else { 90.0 + self.rng.f() * 180.0 })
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
            "get" => match self.o.shuffle_secs { Some(s) => format!("{} (shuffling every {s:.0}s)", self.look.flags()), None => self.look.flags() },
            _ => "commands: random [all] | set key=value ... (shuffle=20s|off) | mood/style/plant/caustic NAME | sleep | wake | get".into(),
        }
    }
}
