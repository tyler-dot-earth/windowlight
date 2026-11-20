// Signals: anything can drop a small file in a folder to say how something is going, and a map file says
// which behaviour of the window each one drives. Sources never say how they should look, and the window
// never knows what a source is, so any signal can drive any behaviour.
//
// A signal file is named after the signal (`cpu`, `herdr-oxc.json`) and holds one of:
//   {"level": 0.4, "state": "busy", "label": "what it is", "ttl": 10}   (every field optional)
//   0.4                                                                  (a bare level)
//   busy                                                                 (a bare state)
// It counts until `ttl` seconds after the file last changed (5 minutes if not given).
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

/// everything a signal can drive; each takes a level (how much) and reacts to states and pulses
pub const BEHAVIOURS: [&str; 10] = ["birds", "wind", "dust", "frost", "haze", "rain", "shade", "suncatcher", "passing", "cat"];
/// the levels the control page can also set by hand
pub const AIR: [&str; 5] = ["dust", "haze", "wind", "frost", "shade"];
pub const STATES: [&str; 5] = ["quiet", "busy", "waiting", "done", "alert"];
const DEFAULT_TTL: f64 = 300.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State { Quiet, Busy, Waiting, Done, Alert }
impl State {
    pub fn parse(s: &str) -> Option<State> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "quiet" | "idle" | "off" | "none" => State::Quiet,
            "busy" | "working" | "on" | "active" => State::Busy,
            "waiting" | "blocked" | "needs_input" => State::Waiting,
            "done" | "finished" | "ok" => State::Done,
            "alert" | "error" | "failed" => State::Alert,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str { STATES[self as usize] }
    /// how much a state-only signal counts as, for behaviours that want a level
    fn level(self) -> f64 { match self { State::Quiet => 0.0, State::Done => 0.25, State::Busy => 0.6, State::Waiting => 0.8, State::Alert => 1.0 } }
    /// a level-only signal is quiet near zero and busy otherwise
    fn of_level(x: f64) -> State { if x < 0.05 { State::Quiet } else { State::Busy } }
}

#[derive(Clone, Debug)]
pub struct Sig { pub name: String, pub label: String, pub level: f64, pub state: State }

/// one map line: signals matching `pat` drive `behaviour`, with levels `lo..hi` stretched to 0..1
#[derive(Clone, Debug)]
pub struct Rule { pub pat: String, pub behaviour: &'static str, lo: f64, hi: f64 }
impl Rule {
    fn amount(&self, x: f64) -> f64 { ((x - self.lo) / (self.hi - self.lo).max(1e-6)).clamp(0.0, 1.0) }
}

pub fn parse_map(text: &str) -> (Vec<Rule>, Vec<String>) {
    let (mut rules, mut errs) = (vec![], vec![]);
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        let w: Vec<&str> = line.split_whitespace().collect();
        let Some(b) = w.get(1).and_then(|b| BEHAVIOURS.iter().find(|x| **x == *b)) else {
            errs.push(format!("line {}: wants `signal-pattern behaviour [from..to]`, behaviour one of {}", i + 1, BEHAVIOURS.join(", ")));
            continue;
        };
        let (lo, hi) = match w.get(2).and_then(|r| r.split_once("..")) {
            Some((a, b)) => match (a.parse(), b.parse()) { (Ok(a), Ok(b)) => (a, b), _ => { errs.push(format!("line {}: range wants from..to like 0.7..1", i + 1)); continue; } },
            None => (0.0, 1.0),
        };
        rules.push(Rule { pat: w[0].to_string(), behaviour: b, lo, hi });
    }
    (rules, errs)
}

/// `*` matches any run of characters, everything else matches itself
pub fn glob(pat: &str, s: &str) -> bool {
    let parts: Vec<&str> = pat.split('*').collect();
    if parts.len() == 1 { return pat == s; }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !s.starts_with(first) || !s[first.len()..].ends_with(last) || s.len() < first.len() + last.len() { return false; }
    let mut rest = &s[first.len()..s.len() - last.len()];
    for p in &parts[1..parts.len() - 1] {
        match rest.find(p) { Some(i) => rest = &rest[i + p.len()..], None => return false }
    }
    true
}

/// read one signal file; None if it is stale or says nothing
fn read_sig(path: &std::path::Path, name: &str) -> Option<Sig> {
    let text = std::fs::read_to_string(path).ok()?;
    let age = std::fs::metadata(path).ok()?.modified().ok().and_then(|m| SystemTime::now().duration_since(m).ok()).unwrap_or_default().as_secs_f64();
    let t = text.trim();
    let (mut level, mut state, mut label, mut ttl) = (None, None, name.to_string(), DEFAULT_TTL);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
        match &v {
            serde_json::Value::Number(n) => level = n.as_f64(),
            serde_json::Value::Object(o) => {
                level = o.get("level").and_then(|x| x.as_f64());
                state = o.get("state").and_then(|x| x.as_str()).and_then(State::parse);
                if let Some(l) = o.get("label").and_then(|x| x.as_str()) { label = l.chars().take(80).collect(); }
                if let Some(x) = o.get("ttl").and_then(|x| x.as_f64()) { ttl = x; }
            }
            serde_json::Value::String(s) => state = State::parse(s),
            _ => {}
        }
    } else {
        state = State::parse(t);
    }
    if ttl > 0.0 && age > ttl { return None; }
    let level = level.map(|x| x.clamp(0.0, 1.0));
    match (level, state) {
        (None, None) => None,
        (Some(l), s) => Some(Sig { name: name.into(), label, level: l, state: s.unwrap_or(State::of_level(l)) }),
        (None, Some(s)) => Some(Sig { name: name.into(), label, level: s.level(), state: s }),
    }
}

/// a signal name is the file name without `.json`, kept to letters, digits and - _ .
fn sig_name(file: &str) -> Option<String> {
    let n = file.strip_suffix(".json").unwrap_or(file);
    (!n.is_empty() && !n.starts_with('.') && n.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))).then(|| n.to_string())
}

pub fn default_dir() -> String {
    std::env::var("WINDOWLIGHT_SIGNALS").unwrap_or_else(|_| {
        let run = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/tmp/windowlight-{}", unsafe { libc::getuid() }));
        format!("{run}/windowlight/signals")
    })
}
/// the default map ships inside the binary; a file at the config path (or $WINDOWLIGHT_SIGNAL_MAP) overrides it
pub const BUILTIN_MAP: &str = include_str!("../signals.map");
pub fn default_map() -> String {
    std::env::var("WINDOWLIGHT_SIGNAL_MAP").unwrap_or_else(|_| crate::app::config_file("signals.map"))
}

/// write a signal file the way a source would (used by `ctl signal`); atomic, so readers never see half
pub fn write(dir: &str, name: &str, value: &str, ttl: Option<f64>) -> Result<String, String> {
    let name = sig_name(name).ok_or("signal names use letters, digits and - _ .")?;
    let body = if let Ok(x) = value.parse::<f64>() {
        serde_json::json!({ "level": x.clamp(0.0, 1.0), "ttl": ttl.unwrap_or(DEFAULT_TTL) })
    } else if let Some(s) = State::parse(value) {
        serde_json::json!({ "state": s.name(), "ttl": ttl.unwrap_or(DEFAULT_TTL) })
    } else if value == "clear" {
        let _ = std::fs::remove_file(format!("{dir}/{name}.json"));
        return Ok(format!("cleared {name}"));
    } else {
        return Err(format!("a signal value is a level 0..1, one of {}, or clear", STATES.join(", ")));
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
    let tmp = format!("{dir}/.{name}.tmp");
    std::fs::write(&tmp, body.to_string()).map_err(|e| format!("{tmp}: {e}"))?;
    std::fs::rename(&tmp, format!("{dir}/{name}.json")).map_err(|e| e.to_string())?;
    Ok(format!("{name} = {value}"))
}

// ---------------------------------------------------------------- birds, one per signal mapped to them

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode { Circle, Perch, Peck, Leave }

#[derive(Clone, Copy)]
struct Pose { x: f64, y: f64, perch: f64 }

struct Bird { name: String, slot: usize, perch: Option<usize>, seed: f64, mode: Mode, from: Pose, t0: Instant, speed: f64 }

/// where birds sit along the sill (window coordinates, glass spans -0.42..0.42), filled in this order so they spread out
const SLOTS: [f64; 6] = [-0.22, 0.24, 0.01, -0.34, 0.35, 0.13];
pub const MAX_BIRDS: usize = 6;
const MOVE: f64 = 1.8;

impl Bird {
    /// where this mode wants the bird `e` seconds in: flying poses are in the plane outside, perched on the sill
    fn target(&self, e: f64) -> Pose {
        match self.mode {
            Mode::Circle => {
                let a = e * (0.5 + 0.7 * self.speed) * if self.seed > 0.5 { 1.0 } else { -1.0 } + self.seed * 40.0;
                let cx = -0.25 + 0.5 * ((self.slot as f64 * 0.37 + self.seed * 0.3).fract());
                Pose { x: cx + 0.24 * a.cos(), y: 0.1 + 0.14 * self.seed + 0.07 * a.sin() + 0.02 * (a * 3.0).sin(), perch: 0.0 }
            }
            Mode::Perch | Mode::Peck => Pose { x: SLOTS[self.perch.unwrap_or(self.slot) % SLOTS.len()], y: 0.0, perch: 1.0 },
            Mode::Leave => {
                let side = if self.seed > 0.5 { 1.0 } else { -1.0 };
                Pose { x: side * 1.6, y: 0.75, perch: 0.0 }
            }
        }
    }
    fn mode_for(s: &Sig) -> Option<Mode> {
        match s.state { State::Quiet => None, State::Busy => Some(Mode::Circle), State::Waiting => Some(Mode::Peck), State::Done => Some(Mode::Perch), State::Alert => Some(Mode::Leave) }
    }
    fn pose(&self, now: Instant) -> Pose {
        let e = (now - self.t0).as_secs_f64();
        let k = smooth(e / MOVE);
        let t = self.target(e);
        Pose { x: self.from.x + (t.x - self.from.x) * k, y: self.from.y + (t.y - self.from.y) * k, perch: self.from.perch + (t.perch - self.from.perch) * smooth(e / MOVE * 1.2 - 0.2) }
    }
    fn set_mode(&mut self, m: Mode, now: Instant) {
        if m == self.mode { return; }
        self.from = self.pose(now);
        self.mode = m;
        self.t0 = now;
    }
    /// x, y, how perched (0 flying, 1 on the sill), and wing beat in flight or head dip when perched
    fn uniform(&self, now: Instant, anim: f64) -> [f32; 4] {
        let p = self.pose(now);
        let ph = anim + self.seed * 17.0;
        let a = if p.perch > 0.5 {
            match self.mode {
                // short bursts of pecks at the glass
                Mode::Peck => { let burst = ((ph * 0.7).sin() > 0.1) as i32 as f64; burst * (ph * 9.0).sin().max(0.0).powi(2) }
                // now and then a little look round
                _ => 0.15 * ((ph * 0.4).sin().max(0.0)).powi(6),
            }
        } else {
            // flap and glide
            let glide = ((ph * 0.45).sin() > 0.35) as i32 as f64;
            0.55 * (ph * (11.0 + self.seed * 3.0)).sin() * (1.0 - glide) + 0.18 * glide
        };
        [p.x as f32, p.y as f32, p.perch as f32, a as f32]
    }
}
fn smooth(x: f64) -> f64 { let x = x.clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) }

// ---------------------------------------------------------------- the reader

pub struct Signals {
    pub dir: Option<String>,
    pub map_path: String,
    pub rules: Vec<Rule>,
    pub map_errs: Vec<String>,
    map_stamp: Option<SystemTime>,
    pub live: Vec<Sig>,
    last_scan: Option<Instant>,
    last_frame: Option<Instant>,
    /// smoothed amount per behaviour
    amounts: HashMap<&'static str, f64>,
    /// levels set by hand on the control page; they replace what the signals say until set back to auto
    pub pinned: HashMap<&'static str, f64>,
    /// one-off nudges waiting for the app: behaviour names
    pub pulses: Vec<&'static str>,
    birds: Vec<Bird>,
}

impl Signals {
    pub fn new(dir: Option<String>, map_path: String) -> Signals {
        Signals { dir, map_path, rules: vec![], map_errs: vec![], map_stamp: None, live: vec![], last_scan: None, last_frame: None,
                  amounts: HashMap::new(), pinned: HashMap::new(), pulses: vec![], birds: vec![] }
    }

    fn load_map(&mut self) {
        let stamp = std::fs::metadata(&self.map_path).and_then(|m| m.modified()).ok();
        if stamp == self.map_stamp && self.last_scan.is_some() { return; }
        self.map_stamp = stamp;
        match std::fs::read_to_string(&self.map_path) {
            Ok(t) => { let (r, e) = parse_map(&t); self.rules = r; self.map_errs = e; }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => { let (r, e) = parse_map(BUILTIN_MAP); self.rules = r; self.map_errs = e; }
            Err(e) => { self.rules = vec![]; self.map_errs = vec![format!("{}: {e}", self.map_path)]; }
        }
        for e in &self.map_errs { eprintln!("windowlight signals: {e}"); }
    }

    fn scan(&mut self) -> Vec<Sig> {
        let Some(dir) = &self.dir else { return vec![] };
        let mut out: Vec<Sig> = std::fs::read_dir(dir).into_iter().flatten().flatten().filter_map(|e| {
            let name = sig_name(e.file_name().to_str()?)?;
            read_sig(&e.path(), &name)
        }).collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out.dedup_by(|a, b| a.name == b.name);
        out
    }

    pub fn behaviours_of(&self, name: &str) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self.rules.iter().filter(|r| glob(&r.pat, name)).map(|r| r.behaviour).collect();
        v.dedup();
        v
    }

    /// look at the folder about once a second; work out pulses and birds from what changed
    pub fn update(&mut self, now: Instant) {
        let first = self.last_scan.is_none();
        if !first && (now - self.last_scan.unwrap()) < Duration::from_secs(1) { self.ease(now, false); return; }
        self.load_map();
        self.last_scan = Some(now);
        let next = self.scan();
        // pulses: a signal turning done or alert, or a level jumping up past 0.75
        for s in &next {
            let before = self.live.iter().find(|o| o.name == s.name);
            for r in self.rules.iter().filter(|r| glob(&r.pat, &s.name)) {
                let fired = match before {
                    Some(b) => (s.state != b.state && matches!(s.state, State::Done | State::Alert)) || (r.amount(b.level) < 0.75 && r.amount(s.level) >= 0.75),
                    None => !first && matches!(s.state, State::Done | State::Alert),
                };
                if fired && !self.pulses.contains(&r.behaviour) { self.pulses.push(r.behaviour); }
            }
        }
        self.live = next;
        self.flock(now, first);
        self.ease(now, first);
    }

    fn ease(&mut self, now: Instant, snap: bool) {
        let dt = self.last_frame.map_or(0.0, |t| (now - t).as_secs_f64());
        self.last_frame = Some(now);
        let k = if snap { 1.0 } else { 1.0 - (-dt / 1.5).exp() };
        for b in BEHAVIOURS {
            let target = self.live.iter().flat_map(|s| self.rules.iter().filter(|r| r.behaviour == b && glob(&r.pat, &s.name)).map(|r| r.amount(s.level)))
                .fold(0.0, f64::max);
            let target = self.pinned.get(b).copied().unwrap_or(target);
            let a = self.amounts.entry(b).or_insert(0.0);
            *a += (target - *a) * k;
        }
    }

    /// 0..1, how strongly the signals mapped to `behaviour` are pushing it right now
    pub fn amount(&self, behaviour: &str) -> f64 { self.amounts.get(behaviour).copied().unwrap_or(0.0) }

    fn flock(&mut self, now: Instant, snap: bool) {
        let mine: Vec<Sig> = self.live.iter().filter(|s| self.behaviours_of(&s.name).contains(&"birds")).cloned().collect();
        // birds whose signal went quiet or vanished fly off; gone ones are dropped
        for b in &mut self.birds {
            let want = mine.iter().find(|s| s.name == b.name).and_then(Bird::mode_for);
            match want {
                Some(m) => { b.set_mode(m, now); if let Some(s) = mine.iter().find(|s| s.name == b.name) { b.speed = s.level; } }
                None => b.set_mode(Mode::Leave, now),
            }
        }
        self.birds.retain(|b| !(b.mode == Mode::Leave && (now - b.t0).as_secs_f64() > MOVE + 0.3));
        self.seat();
        for s in &mine {
            if self.birds.len() >= MAX_BIRDS { break; }
            if self.birds.iter().any(|b| b.name == s.name) { continue; }
            let Some(m) = Bird::mode_for(s) else { continue };
            if m == Mode::Leave { continue; }
            let slot = (0..SLOTS.len()).find(|i| !self.birds.iter().any(|b| b.slot == *i)).unwrap_or(0);
            let seed = (s.name.bytes().fold(7u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32)) % 1000) as f64 / 1000.0;
            let mut b = Bird { name: s.name.clone(), slot, perch: None, seed, mode: Mode::Leave, from: Pose { x: 0.0, y: 0.0, perch: 0.0 }, t0: now, speed: s.level };
            // arrive from off to one side, or already be there for a still
            b.from = b.target(0.0);
            b.mode = m;
            if snap { b.from = b.target(0.0); b.t0 = now - Duration::from_secs_f64(MOVE); } else { b.t0 = now; }
            self.birds.push(b);
            self.seat();
        }
    }

    /// give each bird on the sill its own spot, keeping the ones already sitting where they are
    fn seat(&mut self) {
        for i in 0..self.birds.len() {
            let sits = matches!(self.birds[i].mode, Mode::Perch | Mode::Peck);
            if !sits { self.birds[i].perch = None; continue; }
            if self.birds[i].perch.is_some() { continue; }
            let free = (0..SLOTS.len()).find(|k| !self.birds.iter().any(|b| b.perch == Some(*k))).unwrap_or(0);
            self.birds[i].perch = Some(free);
        }
    }

    /// the birds as shader values, and how many
    pub fn birds(&self, now: Instant, anim: f64) -> ([[f32; 4]; MAX_BIRDS], i32) {
        let mut out = [[0.0; 4]; MAX_BIRDS];
        for (i, b) in self.birds.iter().take(MAX_BIRDS).enumerate() { out[i] = b.uniform(now, anim); }
        (out, self.birds.len().min(MAX_BIRDS) as i32)
    }

    pub fn state(&self) -> serde_json::Value {
        serde_json::json!({
            "dir": self.dir, "map": self.map_path, "errors": self.map_errs,
            "rules": self.rules.iter().map(|r| format!("{} {}", r.pat, r.behaviour)).collect::<Vec<_>>(),
            "live": self.live.iter().map(|s| serde_json::json!({ "name": s.name, "label": s.label, "level": (s.level * 100.0).round() / 100.0,
                "state": s.state.name(), "drives": self.behaviours_of(&s.name) })).collect::<Vec<_>>(),
            "behaviours": BEHAVIOURS,
            "air": AIR.iter().map(|b| serde_json::json!({ "name": b, "level": (self.amount(b) * 100.0).round() / 100.0, "pinned": self.pinned.get(b) })).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn globs() {
        assert!(glob("herdr-*", "herdr-oxc"));
        assert!(!glob("herdr-*", "cpu"));
        assert!(glob("*", "x"));
        assert!(glob("a*b*c", "aXbYc"));
        assert!(!glob("a*b*c", "aXc"));
        assert!(glob("cpu", "cpu"));
    }
    #[test]
    fn map_lines() {
        let (r, e) = parse_map("# hi\nherdr-* birds\ndisk frost 0.7..1\nbad thing\n");
        assert_eq!(r.len(), 2);
        assert_eq!(e.len(), 1);
        assert!((r[1].amount(0.85) - 0.5).abs() < 1e-9);
        assert_eq!(r[1].amount(0.5), 0.0);
    }
    #[test]
    fn files() {
        let d = std::env::temp_dir().join(format!("wl-sig-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("a.json"), r#"{"state":"blocked","label":"x"}"#).unwrap();
        std::fs::write(d.join("b"), "0.4").unwrap();
        std::fs::write(d.join("c"), "done").unwrap();
        std::fs::write(d.join("old.json"), r#"{"level":1,"ttl":0.001}"#).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let mut s = Signals::new(Some(d.to_string_lossy().into()), "/nonexistent".into());
        let got = s.scan();
        let names: Vec<_> = got.iter().map(|x| (x.name.as_str(), x.state)).collect();
        assert_eq!(names, vec![("a", State::Waiting), ("b", State::Busy), ("c", State::Done)]);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
