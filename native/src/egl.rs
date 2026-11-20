// EGL setup: a GLES 3 context either on a Wayland surface or surfaceless (for stills and benchmarks).
use khronos_egl as egl;
use std::ffi::c_void;

pub type Egl = egl::DynamicInstance<egl::EGL1_5>;

const PLATFORM_WAYLAND: egl::Enum = 0x31D8;
const PLATFORM_SURFACELESS: egl::Enum = 0x31DD;

pub struct Gl { pub egl: Egl, pub dpy: egl::Display, pub cfg: egl::Config, pub ctx: egl::Context, pub surf: Option<egl::Surface> }

impl Gl {
    fn open(platform: egl::Enum, native: *mut c_void, window: bool) -> Result<Gl, String> {
        let egl = unsafe { Egl::load_required() }.map_err(|e| format!("can't load libEGL: {e}"))?;
        let dpy = unsafe { egl.get_platform_display(platform, native, &[egl::ATTRIB_NONE]) }.map_err(|e| e.to_string())?;
        egl.initialize(dpy).map_err(|e| format!("initialize: {e}"))?;
        egl.bind_api(egl::OPENGL_ES_API).map_err(|e| e.to_string())?;
        let mut attrs = vec![egl::RED_SIZE, 8, egl::GREEN_SIZE, 8, egl::BLUE_SIZE, 8, egl::RENDERABLE_TYPE, egl::OPENGL_ES3_BIT];
        attrs.extend([egl::SURFACE_TYPE, if window { egl::WINDOW_BIT } else { egl::PBUFFER_BIT }]);
        if window { attrs.extend([egl::ALPHA_SIZE, 0]); }
        attrs.push(egl::NONE);
        let cfg = egl.choose_first_config(dpy, &attrs).map_err(|e| e.to_string())?.ok_or("no EGL config")?;
        let ctx = egl
            .create_context(dpy, cfg, None, &[egl::CONTEXT_MAJOR_VERSION, 3, egl::CONTEXT_MINOR_VERSION, 0, egl::NONE])
            .map_err(|e| format!("create_context: {e}"))?;
        Ok(Gl { egl, dpy, cfg, ctx, surf: None })
    }

    pub fn surfaceless() -> Result<Gl, String> {
        let g = Gl::open(PLATFORM_SURFACELESS, std::ptr::null_mut(), false)?;
        g.egl.make_current(g.dpy, None, None, Some(g.ctx)).map_err(|e| format!("make_current: {e}"))?;
        Ok(g)
    }

    /// `display` is the wl_display pointer, `window` the wl_egl_window pointer
    pub fn wayland(display: *mut c_void, window: *mut c_void) -> Result<Gl, String> {
        let mut g = Gl::open(PLATFORM_WAYLAND, display, true)?;
        let s = unsafe { g.egl.create_window_surface(g.dpy, g.cfg, window as egl::NativeWindowType, None) }
            .map_err(|e| format!("create_window_surface: {e}"))?;
        g.egl.make_current(g.dpy, Some(s), Some(s), Some(g.ctx)).map_err(|e| format!("make_current: {e}"))?;
        // we pace frames ourselves from wl_surface frame callbacks, so never block in swap
        g.egl.swap_interval(g.dpy, 0).ok();
        g.surf = Some(s);
        Ok(g)
    }

    pub fn glow(&self) -> glow::Context {
        unsafe {
            glow::Context::from_loader_function(|name| {
                self.egl.get_proc_address(name).map_or(std::ptr::null(), |p| p as *const c_void)
            })
        }
    }

    pub fn swap(&self) -> Result<(), String> {
        self.egl.swap_buffers(self.dpy, self.surf.ok_or("no surface")?).map_err(|e| e.to_string())
    }
}
