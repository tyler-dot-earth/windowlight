// What a scene looks like: window, plant, mood, caustics, geometry. Plus the mood palette and random picks.
pub const STYLES: [&str; 24] = ["window", "blinds", "lattice", "georgian", "arch", "shutters", "sheer", "porthole", "stained", "slats", "jali",
    "rose", "tracery", "palladian", "shoji", "moorish", "prairie", "moon", "persian", "nouveau",
    "mashrabiya", "girih", "sikri", "jharokha"];
/// the historical windows start here in STYLES
pub const HISTORIC: usize = 11;
pub const PLANTS: [&str; 10] = ["none", "ficus", "monstera", "palm", "olive", "fern", "pothos", "paradise", "pearls", "fiddle"];
/// a tree outside the window, far enough off that its gaps throw round little suns
pub const TREES: [&str; 5] = ["none", "oak", "birch", "willow", "pine"];
pub const MOODS: [&str; 8] = ["linen", "terracotta", "sage", "nocturne", "blush", "pewter", "overcast", "noir"];
/// a mood by name; "concrete" was pewter's name before walls had materials of their own
pub fn mood_index(v: &str) -> Option<usize> { MOODS.iter().position(|x| *x == if v == "concrete" { "pewter" } else { v }) }
/// what the wall is made of, under whichever colour the mood paints it
pub const MATERIALS: [&str; 7] = ["plaster", "limewash", "stucco", "brick", "concrete", "boards", "zellige"];
pub const CAUSTICS: [&str; 4] = ["none", "pool", "glass", "both"];
/// a light of its own: a neon sign across the street, a grow light over the plant, or a flame
/// (sodium and halide are the two HIDs; blurple is red and blue diodes; t5 is a row of fluorescent tubes;
/// then a candle under the plant, a pierced tin lantern, a fireplace and a bonfire in the yard)
pub const LAMPS: [&str; 14] = ["none", "neon", "sodium", "halide", "blurple", "full-spectrum", "t5", "candle", "lantern", "fireplace", "bonfire", "sunset", "rainbow", "afterglow"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    pub wall: [f32; 3], pub sun_tint: [f32; 3], pub shade_tint: [f32; 3], pub night_tint: [f32; 3],
    pub mottle: f32, pub bump: f32, pub expo: f32, pub sat: f32, pub haze: f32, pub contrast: f32, pub amb: f32,
}

impl Mood {
    pub fn named(i: usize) -> Mood {
        let mut m = Mood {
            wall: [0.82, 0.72, 0.61], sun_tint: [1.0, 0.95, 0.87], shade_tint: [0.82, 0.86, 1.0], night_tint: [0.42, 0.52, 0.9],
            mottle: 0.05, bump: 1.0, expo: 1.0, sat: 1.0, haze: 0.0, contrast: 1.0, amb: 0.2,
        };
        match i {
            1 => { m.wall = [0.80, 0.47, 0.34]; m.mottle = 0.22; m.bump = 1.3; m.sun_tint = [1.0, 0.93, 0.84]; m.shade_tint = [0.86, 0.82, 0.98]; }
            2 => { m.wall = [0.60, 0.66, 0.55]; m.mottle = 0.08; m.shade_tint = [0.80, 0.90, 1.0]; m.sun_tint = [1.0, 0.97, 0.88]; }
            3 => { m.wall = [0.34, 0.36, 0.44]; m.sun_tint = [1.0, 0.74, 0.48]; m.shade_tint = [0.55, 0.68, 1.0];
                   m.expo = 1.7; m.contrast = 1.1; m.amb = 0.12; m.mottle = 0.07; }
            4 => { m.wall = [0.88, 0.69, 0.66]; m.haze = 0.3; m.sun_tint = [1.0, 0.9, 0.86]; m.shade_tint = [0.98, 0.86, 0.98]; m.contrast = 0.92; }
            5 => { m.wall = [0.63, 0.63, 0.61]; m.mottle = 0.14; m.bump = 2.4; m.sun_tint = [1.0, 0.98, 0.95]; m.shade_tint = [0.74, 0.84, 1.0]; m.sat = 0.8; }
            6 => { m.wall = [0.76, 0.74, 0.70]; m.haze = 1.0; m.sun_tint = [0.92, 0.95, 1.0]; m.shade_tint = [0.9, 0.94, 1.0]; m.sat = 0.75; m.amb = 0.3; }
            7 => { m.wall = [0.72; 3]; m.sat = 0.0; m.contrast = 1.35; m.expo = 1.05; m.mottle = 0.06; m.amb = 0.12; }
            _ => {}
        }
        m
    }
    pub fn lerp(&self, o: &Mood, t: f32) -> Mood {
        let f = |a: f32, b: f32| a + (b - a) * t;
        let v = |a: [f32; 3], b: [f32; 3]| [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])];
        Mood {
            wall: v(self.wall, o.wall), sun_tint: v(self.sun_tint, o.sun_tint), shade_tint: v(self.shade_tint, o.shade_tint),
            night_tint: v(self.night_tint, o.night_tint), mottle: f(self.mottle, o.mottle), bump: f(self.bump, o.bump),
            expo: f(self.expo, o.expo), sat: f(self.sat, o.sat), haze: f(self.haze, o.haze), contrast: f(self.contrast, o.contrast),
            amb: f(self.amb, o.amb),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Look { pub style: usize, pub plant: usize, pub mood: usize, pub caustic: usize, pub tilt: f64, pub facing: f64, pub rain: bool, pub prism: bool, pub lamp: usize, pub material: usize, pub tree: usize, pub life: bool, pub shelf: bool, pub corner: bool, pub switch: bool, pub hook: bool, pub pinhole: bool }

impl Default for Look {
    fn default() -> Self { Look { style: 1, plant: 1, mood: 0, caustic: 0, tilt: 32.0, facing: 180.0, rain: false, prism: false, lamp: 0, material: 0, tree: 0, life: false, shelf: false, corner: false, switch: true, hook: true, pinhole: false } }
}

fn pick(list: &[&str], v: &str) -> Result<usize, String> {
    list.iter().position(|x| *x == v).ok_or_else(|| format!("unknown {v}, pick one of {}", list.join(" ")))
}

impl Look {
    /// set one field by name; returns Ok(true) if it is a field of Look
    pub fn set(&mut self, k: &str, v: &str) -> Result<bool, String> {
        match k {
            "style" => self.style = pick(&STYLES, v)?,
            "plant" => self.plant = pick(&PLANTS, v)?,
            "mood" => self.mood = mood_index(v).map_or_else(|| pick(&MOODS, v), Ok)?,
            "caustic" => self.caustic = pick(&CAUSTICS, v)?,
            "tilt" => self.tilt = v.parse().map_err(|_| format!("bad tilt {v}"))?,
            "facing" => self.facing = v.parse().map_err(|_| format!("bad facing {v}"))?,
            "rain" => self.rain = matches!(v, "on" | "1" | "true" | "yes"),
            "prism" | "suncatcher" => self.prism = matches!(v, "on" | "1" | "true" | "yes"),
            "lamp" => self.lamp = pick(&LAMPS, v)?,
            "material" | "wall" => self.material = pick(&MATERIALS, v)?,
            "tree" => self.tree = pick(&TREES, v)?,
            "life" => self.life = matches!(v, "on" | "1" | "true" | "yes"),
            "shelf" => self.shelf = matches!(v, "on" | "1" | "true" | "yes"),
            "corner" => self.corner = matches!(v, "on" | "1" | "true" | "yes"),
            "switch" => self.switch = matches!(v, "on" | "1" | "true" | "yes"),
            "hook" => self.hook = matches!(v, "on" | "1" | "true" | "yes"),
            "pinhole" | "obscura" => self.pinhole = matches!(v, "on" | "1" | "true" | "yes"),
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub fn flags(&self) -> String {
        format!("--style {} --plant {} --mood {} --caustic {} --tilt {:.0} --facing {:.0} --rain {} --prism {} --lamp {} --material {} --tree {} --life {} --shelf {} --corner {} --switch {} --hook {} --pinhole {}",
            STYLES[self.style], PLANTS[self.plant], MOODS[self.mood], CAUSTICS[self.caustic], self.tilt, self.facing,
            if self.rain { "on" } else { "off" }, if self.prism { "on" } else { "off" }, LAMPS[self.lamp], MATERIALS[self.material], TREES[self.tree], if self.life { "on" } else { "off" }, if self.shelf { "on" } else { "off" }, if self.corner { "on" } else { "off" }, if self.switch { "on" } else { "off" }, if self.hook { "on" } else { "off" }, if self.pinhole { "on" } else { "off" })
    }
    /// a random look; facing stays unless `facing` is given
    pub fn random(rng: &mut Rng, facing: Option<f64>, base: &Look) -> Look {
        let caustic = match rng.f() { x if x < 0.45 => 0, x if x < 0.65 => 1, x if x < 0.82 => 2, _ => 3 };
        let plant = if rng.f() < 0.12 { 0 } else { 1 + rng.below(PLANTS.len() - 1) };
        Look {
            style: rng.below(STYLES.len()), plant, mood: rng.below(MOODS.len()), caustic,
            tilt: ((12.0 + rng.f() * 50.0) * if rng.f() < 0.5 { -1.0 } else { 1.0 }).round(), facing: facing.unwrap_or(base.facing).round(), rain: rng.f() < 0.15, prism: rng.f() < 0.3,
            lamp: if rng.f() < 0.12 { 1 + rng.below(LAMPS.len() - 1) } else { 0 },
            material: if rng.f() < 0.4 { 0 } else { 1 + rng.below(MATERIALS.len() - 1) },
            tree: if rng.f() < 0.6 { 0 } else { 1 + rng.below(TREES.len() - 1) },
            life: rng.f() < 0.35,
            shelf: rng.f() < 0.3,
            corner: rng.f() < 0.3,
            switch: rng.f() < 0.6,
            hook: rng.f() < 0.5,
            pinhole: rng.f() < 0.08,
        }
    }
    /// a look from flags like the ones flags() prints; unknown flags are ignored
    pub fn from_flags(line: &str) -> Option<Look> {
        let mut l = Look::default();
        let w: Vec<&str> = line.split_whitespace().collect();
        let mut any = false;
        for p in w.windows(2) {
            if let Some(k) = p[0].strip_prefix("--") { if l.set(k, p[1]) == Ok(true) { any = true; } }
        }
        any.then_some(l)
    }
}

pub struct Rng(pub u64);
impl Rng {
    pub fn seeded() -> Rng {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
        Rng(t ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    pub fn next(&mut self) -> u64 { let mut x = self.0; x ^= x << 13; x ^= x >> 7; x ^= x << 17; self.0 = x; x }
    pub fn f(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
    pub fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}
