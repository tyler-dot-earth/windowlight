// Live wallpaper: a layer-shell background surface on one output, rendered at the output's
// physical resolution and scaled to its logical size with wp_viewporter.
use crate::app::{App, Opts};
use crate::egl::Gl;
use crate::gfx::Renderer;
use crate::ipc;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn on_signal(_: libc::c_int) { STOP.store(true, Ordering::Relaxed); }
use wayland_client::{
    delegate_noop,
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
    Connection, Proxy, QueueHandle,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};

struct State {
    registry: RegistryState,
    outputs: OutputState,
    configured: Option<(u32, u32)>,
    frame_ok: bool,
    closed: bool,
}

pub fn run(o: Opts) {
    if let Err(e) = run_inner(o) { eprintln!("windowlight: {e}"); ipc::cleanup(); std::process::exit(1); }
    ipc::cleanup();
}

fn run_inner(o: Opts) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland display: {e}"))?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh).map_err(|e| e.to_string())?;
    let layer_shell = LayerShell::bind(&globals, &qh).map_err(|e| format!("no layer shell: {e}"))?;
    let viewporter: WpViewporter = globals.bind(&qh, 1..=1, ()).map_err(|e| format!("no viewporter: {e}"))?;
    let mut st = State {
        registry: RegistryState::new(&globals), outputs: OutputState::new(&globals, &qh),
        configured: None, frame_ok: true, closed: false,
    };
    queue.roundtrip(&mut st).map_err(|e| e.to_string())?;

    // pick the output and its physical mode size
    let mut chosen = None;
    for out in st.outputs.outputs() {
        let Some(info) = st.outputs.info(&out) else { continue };
        if info.name.as_deref() == Some(o.output.as_str()) {
            let mode = info.modes.iter().find(|m| m.current).map(|m| m.dimensions);
            chosen = Some((out, mode));
        }
    }
    let Some((output, mode)) = chosen else {
        let names: Vec<String> = st.outputs.outputs().filter_map(|x| st.outputs.info(&x)?.name).collect();
        return Err(format!("no output named {}; have {}", o.output, names.join(", ")));
    };

    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Background, Some("windowlight"), Some(&output));
    layer.set_anchor(Anchor::all());
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.commit();
    while st.configured.is_none() && !st.closed { queue.blocking_dispatch(&mut st).map_err(|e| e.to_string())?; }
    let (lw, lh) = st.configured.unwrap_or((1, 1));
    // buffer size: the physical mode (falls back to logical) times --scale
    let (pw, ph) = mode.map(|(w, h)| (w as u32, h as u32)).unwrap_or((lw, lh));
    let (pw, ph) = (pw, ph);
    let (bw, bh) = (((pw as f64) * o.scale).round().max(1.0) as i32, ((ph as f64) * o.scale).round().max(1.0) as i32);
    let viewport: WpViewport = viewporter.get_viewport(layer.wl_surface(), &qh, ());
    viewport.set_destination(lw as i32, lh as i32);
    eprintln!("windowlight: {} logical {lw}x{lh}, drawing {bw}x{bh} at {} fps", o.output, o.fps);

    let wl_egl = wayland_egl::WlEglSurface::new(layer.wl_surface().id(), bw, bh).map_err(|e| e.to_string())?;
    let gl = Gl::wayland(conn.backend().display_ptr() as *mut _, wl_egl.ptr() as *mut _)?;
    let r = unsafe { Renderer::new(gl.glow())? };

    unsafe {
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
    }
    let (tx, rx) = ipc::listen()?;
    if !o.workspaces.is_empty() { ipc::follow_niri(o.output.clone(), tx.clone()); }
    eprintln!("windowlight: control socket {}", ipc::sock_path());
    let frame_dt = |a: &App| Duration::from_secs_f64(1.0 / a.o.fps);
    let mut a = unsafe { App::new(o, &r) };
    let mut next = Instant::now();
    let mut last_frame = Instant::now();

    loop {
        // commands from the socket and from niri
        let mut woke = false;
        while let Ok(c) = rx.try_recv() {
            let reply = unsafe { a.command(&r, &c.line) };
            if c.line.starts_with("wake") { woke = true; }
            if let Some(t) = c.reply { let _ = t.send(reply); }
        }
        if woke { next = Instant::now(); st.frame_ok = true; }

        let now = Instant::now();
        // the compositor stops sending frame callbacks while the wallpaper is hidden; retry once a second anyway
        let can = st.frame_ok || now - last_frame > Duration::from_secs(1);
        if !a.asleep && now >= next && can {
            layer.wl_surface().frame(&qh, layer.wl_surface().clone());
            unsafe { a.render(&r, None, bw, bh) };
            gl.swap()?;
            st.frame_ok = false;
            last_frame = now;
            next += frame_dt(&a);
            if next < now { next = now + frame_dt(&a); }
        }

        // wait for Wayland events, the next frame time, or a 50 ms tick for commands
        queue.flush().map_err(|e| e.to_string())?;
        let wait = if a.asleep { Duration::from_millis(100) } else { next.saturating_duration_since(Instant::now()).min(Duration::from_millis(50)) };
        if let Some(guard) = queue.prepare_read() {
            let mut fds = [libc::pollfd { fd: guard.connection_fd().as_raw_fd(), events: libc::POLLIN, revents: 0 }];
            let n = unsafe { libc::poll(fds.as_mut_ptr(), 1, wait.as_millis() as i32) };
            if n > 0 { guard.read().map_err(|e| e.to_string())?; } else { drop(guard); }
        }
        queue.dispatch_pending(&mut st).map_err(|e| e.to_string())?;
        if st.closed || STOP.load(Ordering::Relaxed) { break; }
        if let Some((w, h)) = st.configured.take() {
            if (w, h) != (lw, lh) { viewport.set_destination(w as i32, h as i32); }
        }
    }
    unsafe { a.free(&r) };
    Ok(())
}

impl CompositorHandler for State {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) { self.frame_ok = true; }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState { &mut self.outputs }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) { self.closed = true; }
    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, l: &LayerSurface, c: LayerSurfaceConfigure, _: u32) {
        let (w, h) = c.new_size;
        self.configured = Some((w.max(1), h.max(1)));
        l.commit();
    }
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry }
    registry_handlers![OutputState];
}

delegate_compositor!(State);
delegate_output!(State);
delegate_layer!(State);
delegate_registry!(State);
delegate_noop!(State: ignore WpViewporter);
delegate_noop!(State: ignore WpViewport);
