// GL ES 3 renderer. Slow or sun-independent parts are baked into textures; the per-frame
// full-resolution pass only does a handful of texture lookups per pixel.
use crate::look::Mood;
use glow::HasContext;

const COMMON: &str = include_str!("../shaders/common.glsl");
const VERT: &str = include_str!("../shaders/fullscreen.vert");

// 2048 texels per wall unit, finer than a 4K screen draws it (about 1540 px per unit)
const PLASTER: (i32, i32) = (4096, 4096);
const AP: (i32, i32) = (1536, 1728);
const FOL: (i32, i32) = (2048, 1906);
const CAUS: (i32, i32) = (1024, 1152);
const AP_SIZE: (f32, f32) = (1.2, 1.35);
const FOL_SIZE: (f32, f32) = (2.9, 2.7);

pub struct Target { pub tex: glow::Texture, pub fb: glow::Framebuffer, pub w: i32, pub h: i32, mips: bool }

/// everything one frame of one scene needs
#[derive(Clone)]
pub struct Frame {
    pub time: f32, pub style: i32, pub plant: i32, pub caustic: i32, pub tilt: f32,
    pub sun: [f32; 3], pub sun_alt: f32, pub mood: Mood, pub fade: f32,
    pub rain: f32, pub gust: f32, pub cat: [f32; 4], pub light_k: f32,
    pub prism: f32, pub passing: [f32; 4], pub season: [f32; 4],
    /// from signals: birds (x, y, perched, beat) and how many, dust, frost, extra suncatcher turn
    pub flock: [[f32; 4]; 6], pub flock_n: i32, pub dust: f32, pub frost: f32, pub spin: f32,
}

/// per-scene textures (two scenes exist while crossfading)
pub struct SceneTex { ap: Target, fol: Target, caus: Target, ap_key: Option<(i32, i32, bool, i32)> }

pub struct Renderer {
    pub gl: glow::Context,
    vao: glow::VertexArray,
    plaster: Target,
    p_ap: glow::Program, p_fol: glow::Program, p_caus: glow::Program, p_comp: glow::Program,
}

unsafe fn program(gl: &glow::Context, name: &str, frag: &str) -> Result<glow::Program, String> {
    let p = gl.create_program()?;
    for (kind, src) in [(glow::VERTEX_SHADER, format!("#version 300 es\n{VERT}")),
                        (glow::FRAGMENT_SHADER, format!("#version 300 es\n{COMMON}\n{frag}"))] {
        let s = gl.create_shader(kind)?;
        gl.shader_source(s, &src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) { return Err(format!("{name}: {}", gl.get_shader_info_log(s))); }
        gl.attach_shader(p, s);
    }
    gl.link_program(p);
    if !gl.get_program_link_status(p) { return Err(format!("{name} link: {}", gl.get_program_info_log(p))); }
    Ok(p)
}

unsafe fn target(gl: &glow::Context, (w, h): (i32, i32), internal: u32, mips: bool, repeat: bool) -> Target {
    let tex = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    let levels = if mips { 32 - (w.max(h) as u32).leading_zeros() } else { 1 } as i32;
    gl.tex_storage_2d(glow::TEXTURE_2D, levels, internal, w, h);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, if mips { glow::LINEAR_MIPMAP_LINEAR } else { glow::LINEAR } as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    let wrap = if repeat { glow::REPEAT } else { glow::CLAMP_TO_EDGE } as i32;
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap);
    let fb = gl.create_framebuffer().unwrap();
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fb));
    gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
    Target { tex, fb, w, h, mips }
}

impl Renderer {
    pub unsafe fn new(gl: glow::Context) -> Result<Renderer, String> {
        if std::env::var_os("WL_VERBOSE").is_some() { eprintln!("GL: {} / {}", gl.get_parameter_string(glow::RENDERER), gl.get_parameter_string(glow::VERSION)); }
        let exts = gl.supported_extensions();
        if !exts.contains("GL_EXT_color_buffer_float") && !exts.contains("GL_EXT_color_buffer_half_float") {
            return Err("GPU can't render to half-float textures".into());
        }
        let vao = gl.create_vertex_array()?;
        let p_plaster = program(&gl, "plaster", include_str!("../shaders/plaster.frag"))?;
        let r = Renderer {
            p_ap: program(&gl, "aperture", include_str!("../shaders/aperture.frag"))?,
            p_fol: program(&gl, "foliage", include_str!("../shaders/foliage.frag"))?,
            p_caus: program(&gl, "caustic", include_str!("../shaders/caustic.frag"))?,
            p_comp: program(&gl, "composite", include_str!("../shaders/composite.frag"))?,
            plaster: target(&gl, PLASTER, glow::R16F, true, true),
            vao, gl,
        };
        r.pass(p_plaster, &r.plaster, None);
        r.gl.delete_program(p_plaster);
        Ok(r)
    }

    pub unsafe fn scene_tex(&self) -> SceneTex {
        SceneTex {
            ap: target(&self.gl, AP, glow::RGBA16F, true, false),
            fol: target(&self.gl, FOL, glow::R8, true, false),
            caus: target(&self.gl, CAUS, glow::R16F, true, false),
            ap_key: None,
        }
    }
    pub unsafe fn drop_scene(&self, s: SceneTex) {
        for t in [s.ap, s.fol, s.caus] { self.gl.delete_framebuffer(t.fb); self.gl.delete_texture(t.tex); }
    }

    unsafe fn uniforms(&self, p: glow::Program, f: &Frame) {
        let gl = &self.gl;
        let u = |n: &str| gl.get_uniform_location(p, n);
        gl.uniform_1_f32(u("time").as_ref(), f.time);
        gl.uniform_1_i32(u("style").as_ref(), f.style);
        gl.uniform_1_i32(u("plantType").as_ref(), f.plant);
        gl.uniform_1_i32(u("caustic").as_ref(), f.caustic);
        gl.uniform_1_f32(u("tilt").as_ref(), f.tilt);
        gl.uniform_3_f32_slice(u("sun").as_ref(), &f.sun);
        gl.uniform_1_f32(u("sunAlt").as_ref(), f.sun_alt);
        gl.uniform_1_f32(u("haze").as_ref(), f.mood.haze);
        gl.uniform_1_f32(u("rain").as_ref(), f.rain);
        gl.uniform_1_f32(u("gust").as_ref(), f.gust);
        gl.uniform_4_f32_slice(u("cat").as_ref(), &f.cat);
        gl.uniform_1_f32(u("prism").as_ref(), f.prism);
        gl.uniform_4_f32_slice(u("passing").as_ref(), &f.passing);
        gl.uniform_4_f32_slice(u("season").as_ref(), &f.season);
        gl.uniform_4_f32_slice(u("flock").as_ref(), f.flock.as_flattened());
        gl.uniform_1_i32(u("flockN").as_ref(), f.flock_n);
        gl.uniform_1_f32(u("dust").as_ref(), f.dust);
        gl.uniform_1_f32(u("frostSig").as_ref(), f.frost);
        gl.uniform_1_f32(u("spinPh").as_ref(), f.spin);
    }

    /// draw a fullscreen pass of `p` into `t` (or the given framebuffer), then rebuild mips
    unsafe fn pass(&self, p: glow::Program, t: &Target, f: Option<&Frame>) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.fb));
        gl.viewport(0, 0, t.w, t.h);
        gl.disable(glow::BLEND);
        gl.use_program(Some(p));
        if let Some(f) = f { self.uniforms(p, f); }
        gl.uniform_2_f32(gl.get_uniform_location(p, "texSize").as_ref(), t.w as f32, t.h as f32);
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        if t.mips { gl.bind_texture(glow::TEXTURE_2D, Some(t.tex)); gl.generate_mipmap(glow::TEXTURE_2D); }
    }

    /// render one scene into `fb` (None = the window), blended over what's there by `alpha`
    pub unsafe fn draw(&self, s: &mut SceneTex, f: &Frame, fb: Option<glow::Framebuffer>, w: i32, h: i32, alpha: f32) {
        let gl = &self.gl;
        // window light: only when the style or tilt changes, or every frame while something on the glass moves
        // (the sheer curtain, rain, or the cat)
        let moving = f.cat[3] != 0.0;
        let key = (f.style, (f.tilt * 1000.0) as i32, moving, (f.season[3] * 64.0) as i32 + 1000 * (f.frost * 24.0) as i32);
        if s.ap_key != Some(key) || f.style == 6 || f.rain > 0.0 || moving { self.pass(self.p_ap, &s.ap, Some(f)); s.ap_key = Some(key); }
        if f.plant > 0 { self.pass(self.p_fol, &s.fol, Some(f)); }
        let pool = f.caustic == 1 || f.caustic == 3;
        if pool && f.sun_alt > 0.0 { self.pass(self.p_caus, &s.caus, Some(f)); }

        gl.bind_framebuffer(glow::FRAMEBUFFER, fb);
        gl.viewport(0, 0, w, h);
        if alpha < 1.0 {
            gl.enable(glow::BLEND);
            gl.blend_color(0.0, 0.0, 0.0, alpha);
            gl.blend_func(glow::CONSTANT_ALPHA, glow::ONE_MINUS_CONSTANT_ALPHA);
        } else {
            gl.disable(glow::BLEND);
        }
        let p = self.p_comp;
        gl.use_program(Some(p));
        self.uniforms(p, f);
        let u = |n: &str| gl.get_uniform_location(p, n);
        gl.uniform_2_f32(u("res").as_ref(), w as f32, h as f32);
        gl.uniform_2_f32(u("apTexel").as_ref(), AP.0 as f32 / AP_SIZE.0, AP.1 as f32 / AP_SIZE.1);
        gl.uniform_2_f32(u("folTexel").as_ref(), FOL.0 as f32 / FOL_SIZE.0, FOL.1 as f32 / FOL_SIZE.1);
        let m = &f.mood;
        gl.uniform_3_f32_slice(u("wallA").as_ref(), &m.wall);
        gl.uniform_3_f32_slice(u("sunTint").as_ref(), &m.sun_tint);
        gl.uniform_3_f32_slice(u("shadeTint").as_ref(), &m.shade_tint);
        gl.uniform_3_f32_slice(u("nightTint").as_ref(), &m.night_tint);
        for (n, v) in [("mottle", m.mottle), ("bumpAmt", m.bump), ("expo", m.expo), ("sat", m.sat),
                       ("contrast", m.contrast), ("ambAmt", m.amb), ("fade", f.fade), ("lightK", f.light_k)] {
            gl.uniform_1_f32(u(n).as_ref(), v);
        }
        for (i, (n, t)) in [("uPlaster", &self.plaster), ("uAp", &s.ap), ("uFol", &s.fol), ("uCaus", &s.caus)].iter().enumerate() {
            gl.active_texture(glow::TEXTURE0 + i as u32);
            gl.bind_texture(glow::TEXTURE_2D, Some(t.tex));
            gl.uniform_1_i32(u(n).as_ref(), i as i32);
        }
        gl.active_texture(glow::TEXTURE0);
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.disable(glow::BLEND);
    }

    /// offscreen RGBA8 target for stills
    pub unsafe fn canvas(&self, w: i32, h: i32) -> Target { target(&self.gl, (w, h), glow::RGBA8, false, false) }

    pub unsafe fn drop_target(&self, t: Target) { self.gl.delete_framebuffer(t.fb); self.gl.delete_texture(t.tex); }

    pub unsafe fn read_rgb(&self, t: &Target) -> Vec<u8> {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.fb));
        let mut px = vec![0u8; (t.w * t.h * 4) as usize];
        gl.read_pixels(0, 0, t.w, t.h, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut px)));
        let row = (t.w * 4) as usize;
        let mut out = Vec::with_capacity((t.w * t.h * 3) as usize);
        for y in (0..t.h as usize).rev() {
            for x in 0..t.w as usize { out.extend_from_slice(&px[y * row + x * 4..y * row + x * 4 + 3]); }
        }
        out
    }
}
