// windowlight: a sunlit-wall wallpaper for Wayland, drawn natively at full resolution.
//   windowlight run [flags]            live wallpaper on one output (layer-shell background)
//   windowlight shot out.png [flags]   one still, offscreen
//   windowlight sample N dir [flags]   N random looks as PNGs plus looks.txt
//   windowlight bench [flags]          time frames offscreen at --size (default 3840x2160)
//   windowlight ctl WORDS...           send a command to the running wallpaper
mod app;
mod egl;
mod gfx;
mod ipc;
mod look;
mod signals;
mod sun;
mod ui;
mod wl;

use app::{App, Opts};
use look::{Look, Rng};
use std::time::Instant;

fn usage() -> ! {
    eprintln!("usage: windowlight run|shot OUT.png|sample N DIR|bench|ctl WORDS...\n\
  look:   --style S --plant P --tree T --mood M --caustic C --tilt DEG --facing DEG --rain on|off --prism on|off --life on|off --shelf on|off --corner on|off --switch on|off --hook on|off --pinhole on|off --lamp L --material M --random\n\
  place:  --lat X --lon Y (else the place picked on the page) --season auto|spring|summer|autumn|winter\n\
  time:   --at 2026-11-19T11:00 --speed X --anim SECONDS\n\
  live:   --output NAME (default: first) --fps 30 --scale 1.0 --workspaces linen,sage,... --shuffle 20s|5m --timelapse day/2m|full/5m|07:00-18:00/90s\n          --pool all|favourites|historic --cat 10m|off --passing 4m|off --breeze on|off --ui 7417|off\n\
  signals: --signals DIR|off (default $XDG_RUNTIME_DIR/windowlight/signals) --signal-map FILE (default signals.map here)\n\
  stills: --size 1920x1080 --frames 120 (bench)");
    std::process::exit(2)
}

fn parse_at(s: &str) -> f64 {
    use chrono::TimeZone;
    let ndt = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap_or_else(|_| {
        eprintln!("--at wants local time like 2026-11-19T11:00");
        std::process::exit(2)
    });
    chrono::Local.from_local_datetime(&ndt).earliest().expect("local time").timestamp_millis() as f64
}

struct Cli { saved: Option<usize>, opts: Opts, pos: Vec<String>, random: bool, frames: usize, size_set: bool, headless_ui: Option<u16>, headless_breeze: bool }

/// `saved`: start from the newest kept look (the live wallpaper does); flags still override it
fn parse(args: &[String], saved: bool) -> Cli {
    let mut o = Opts {
        lat: 37.77, lon: -122.42, look: Look::default(), speed: 1.0, at_ms: None, anim: 0.0,
        fps: 30.0, scale: 1.0, output: String::new(), workspaces: vec![], shuffle_secs: None, size: (1920, 1080), lapse: None,
        cat_secs: Some(600.0), pool: "all", breeze: true, ui_port: Some(7417), season: None, pass_secs: Some(240.0),
        signals: Some(signals::default_dir()), signal_map: signals::default_map(),
    };
    if let Some((lat, lon)) = app::load_place() { (o.lat, o.lon) = (lat, lon); }
    let favs = if saved { app::load_favs() } else { vec![] };
    let from = favs.iter().enumerate().rev().find_map(|(i, f)| Look::from_flags(f).map(|l| (i, l)));
    if let Some((_, l)) = &from { o.look = l.clone(); }
    let (mut pos, mut random, mut frames, mut size_set) = (vec![], false, 120, false);
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let Some(k) = a.strip_prefix("--") else { pos.push(a.clone()); i += 1; continue };
        if k == "random" { random = true; i += 1; continue; }
        let Some(v) = args.get(i + 1) else { usage() };
        let num = || v.parse::<f64>().unwrap_or_else(|_| { eprintln!("--{k} wants a number"); std::process::exit(2) });
        match k {
            "lat" => o.lat = num(),
            "lon" => o.lon = num(),
            "speed" => o.speed = num(),
            "anim" => o.anim = num(),
            "fps" => o.fps = num().max(1.0),
            "scale" => o.scale = num().clamp(0.1, 2.0),
            "shuffle" => o.shuffle_secs = app::parse_secs(v).unwrap_or_else(|| { eprintln!("--shuffle wants 20s, 5m or 1h"); std::process::exit(2) }),
            "frames" => frames = num() as usize,
            "at" => o.at_ms = Some(parse_at(v)),
            "output" => o.output = v.clone(),
            "timelapse" => o.lapse = Some(v.clone()),
            "cat" => o.cat_secs = app::parse_secs(v).unwrap_or_else(|| { eprintln!("--cat wants 10m, 1h or off"); std::process::exit(2) }),
            "pool" => o.pool = app::parse_pool(v).unwrap_or_else(|| { eprintln!("--pool wants all, favourites or historic"); std::process::exit(2) }),
            "breeze" => o.breeze = v != "off",
            "passing" => o.pass_secs = app::parse_secs(v).unwrap_or_else(|| { eprintln!("--passing wants 4m, 1h or off"); std::process::exit(2) }),
            "season" => o.season = match app::SEASONS.iter().position(|x| x == v) {
                Some(i) => (i < 4).then_some(i),
                None => { eprintln!("--season wants auto, spring, summer, autumn or winter"); std::process::exit(2) }
            },
            "signals" => o.signals = (v != "off").then(|| v.clone()),
            "signal-map" => o.signal_map = v.clone(),
            "ui" => o.ui_port = if v == "off" { None } else { Some(v.parse().unwrap_or_else(|_| usage())) },
            "size" => {
                let (w, h) = v.split_once('x').unwrap_or_else(|| usage());
                o.size = (w.parse().unwrap_or_else(|_| usage()), h.parse().unwrap_or_else(|_| usage()));
                size_set = true;
            }
            "workspaces" => {
                o.workspaces = v.split(',').map(|m| look::mood_index(m).unwrap_or_else(|| {
                    eprintln!("unknown mood {m}, pick from {}", look::MOODS.join(" "));
                    std::process::exit(2)
                })).collect()
            }
            _ => match o.look.set(k, v) {
                Ok(true) => {}
                Ok(false) => { eprintln!("unknown flag --{k}"); usage() }
                Err(e) => { eprintln!("{e}"); std::process::exit(2) }
            },
        }
        i += 2;
    }
    // only the live wallpaper serves the page by default; the headless driver serves it when --ui is given
    let headless_ui = args.iter().any(|a| a == "--ui").then_some(o.ui_port).flatten();
    let headless_breeze = args.windows(2).any(|w| w[0] == "--breeze" && w[1] != "off");
    let saved = from.map(|(i, _)| i + 1).filter(|_| !args.iter().any(|a| a == "--facing"));
    Cli { saved, opts: o, pos, random, frames, size_set, headless_ui, headless_breeze }
}

fn write_png(path: &str, w: i32, h: i32, rgb: &[u8]) {
    let f = std::fs::File::create(path).unwrap_or_else(|e| { eprintln!("{path}: {e}"); std::process::exit(1) });
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgb).unwrap();
}

fn sunny_facing(rng: &mut Rng, ms: f64, o: &Opts) -> f64 { sun::facing_for(rng, ms, o.lat, o.lon) }

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = args.first().cloned() else { usage() };
    if mode == "ctl" {
        let line = args[1..].join(" ");
        match ipc::send(&line) {
            Ok(reply) => println!("{reply}"),
            Err(e) => { eprintln!("windowlight isn't running? {e}"); std::process::exit(1) }
        }
        return;
    }
    let live = mode == "run" || (mode == "headless" && args.iter().any(|a| a == "--ui"));
    let mut cli = parse(&args[1..], live);
    let mut rng = Rng::seeded();
    if cli.random {
        let ms = cli.opts.at_ms.unwrap_or_else(app::now_ms);
        let f = sunny_facing(&mut rng, ms, &cli.opts);
        cli.opts.look = Look::random(&mut rng, Some(f), &cli.opts.look);
        eprintln!("look: {}", cli.opts.look.flags());
    } else if let Some(n) = cli.saved {
        // the newest kept look; the window turns toward the light if it would be dark as saved, like `fav N`
        let ms = cli.opts.at_ms.unwrap_or_else(app::now_ms);
        if !sun::lets_in(ms, cli.opts.lat, cli.opts.lon, cli.opts.look.facing) { cli.opts.look.facing = sunny_facing(&mut rng, ms, &cli.opts).round(); }
        eprintln!("look: kept look {n}: {}", cli.opts.look.flags());
    }
    match mode.as_str() {
        "run" => wl::run(cli.opts),
        "shot" => {
            let out = cli.pos.first().cloned().unwrap_or_else(|| usage());
            offscreen(&cli.opts, |r| unsafe {
                let (w, h) = cli.opts.size;
                let c = r.canvas(w, h);
                let mut a = App::new(cli.opts.clone(), r);
                a.render(r, Some(c.fb), w, h);
                write_png(&out, w, h, &r.read_rgb(&c));
                a.free(r);
            });
        }
        "sample" => {
            let n: usize = cli.pos.first().and_then(|s| s.parse().ok()).unwrap_or(12);
            let dir = cli.pos.get(1).cloned().unwrap_or_else(|| "samples".into());
            std::fs::create_dir_all(&dir).unwrap();
            let size = if cli.size_set { cli.opts.size } else { (960, 540) };
            offscreen(&cli.opts, |r| unsafe {
                let c = r.canvas(size.0, size.1);
                let mut log = String::new();
                for i in 0..n {
                    let mut o = cli.opts.clone();
                    // a random sunny moment today unless --at pins it
                    let ms = o.at_ms.unwrap_or_else(|| sun::sunny_moment(&mut rng, app::now_ms(), o.lat, o.lon));
                    o.at_ms = Some(ms);
                    o.anim = rng.f() * 600.0;
                    let f = sunny_facing(&mut rng, ms, &o);
                    o.look = Look::random(&mut rng, Some(f), &o.look);
                    let sq = |t: f64| sun::squareness(ms, o.lat, o.lon, f.round(), t);
                    if sq(o.look.tilt) < 0.55 && sq(-o.look.tilt) > sq(o.look.tilt) { o.look.tilt = -o.look.tilt; }
                    let mut a = App::new(o.clone(), r);
                    a.render(r, Some(c.fb), size.0, size.1);
                    let name = format!("{dir}/{:02}.png", i + 1);
                    write_png(&name, size.0, size.1, &r.read_rgb(&c));
                    a.free(r);
                    let at = chrono::DateTime::from_timestamp_millis(ms as i64).unwrap().with_timezone(&chrono::Local);
                    let line = format!("{:02}  {} --at {} --anim {:.0}\n", i + 1, o.look.flags(), at.format("%Y-%m-%dT%H:%M"), o.anim);
                    eprint!("{line}");
                    log.push_str(&line);
                }
                std::fs::write(format!("{dir}/looks.txt"), log).unwrap();
            });
        }
        // offscreen test driver: one command per stdin line, "wait SECONDS" keeps rendering (and serves the
        // control page when --ui is given); replies go to stdout
        "headless" => {
            let rx = (cli.headless_ui.is_some() || cli.headless_breeze).then(|| {
                let (tx, rx) = std::sync::mpsc::channel::<ipc::Cmd>();
                if let Some(port) = cli.headless_ui { ui::serve(port, tx.clone()); }
                if cli.headless_breeze { ipc::follow_niri(cli.opts.output.clone(), true, tx); }
                rx
            });
            let (hw, hh) = if cli.size_set { cli.opts.size } else { (320, 180) };
            offscreen(&cli.opts, |r| unsafe {
                let c = r.canvas(hw, hh);
                let mut a = App::new(cli.opts.clone(), r);
                for line in std::io::stdin().lines().map_while(Result::ok) {
                    if let Some(s) = line.strip_prefix("wait ") {
                        let end = Instant::now() + std::time::Duration::from_secs_f64(s.trim().parse().unwrap_or(1.0));
                        while Instant::now() < end {
                            while let Some(c) = rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
                                println!("ui> {}", c.line);
                                let reply = a.command(r, &c.line);
                                if let Some(t) = c.reply { let _ = t.send(reply); }
                            }
                            a.render(r, Some(c.fb), hw, hh);
                            std::thread::sleep(std::time::Duration::from_millis(33));
                        }
                    } else if let Some(rest) = line.strip_prefix("record ") {
                        // "record DIR SECONDS [FPS]": numbered frames on a stepped clock, so motion stays even
                        // however long each frame takes to write; later commands keep to that clock
                        let mut w = rest.split_whitespace();
                        let (Some(dir), Some(secs)) = (w.next(), w.next().and_then(|s| s.parse::<f64>().ok())) else { println!("record wants DIR SECONDS [FPS]"); continue };
                        let fps = w.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(30.0);
                        std::fs::create_dir_all(dir).unwrap();
                        let first = std::fs::read_dir(dir).map_or(0, |d| d.count());
                        let step = std::time::Duration::from_secs_f64(1.0 / fps);
                        for i in 0..(secs * fps).round() as usize {
                            a.clock = Some(a.clock.unwrap_or_else(Instant::now) + step);
                            a.render(r, Some(c.fb), hw, hh);
                            write_png(&format!("{dir}/{:05}.png", first + i), hw, hh, &r.read_rgb(&c));
                        }
                    } else if let Some(p) = line.strip_prefix("shot ") {
                        a.render(r, Some(c.fb), hw, hh);
                        write_png(p.trim(), hw, hh, &r.read_rgb(&c));
                    } else {
                        println!("> {line}\n{}", a.command(r, &line));
                    }
                }
                a.free(r);
            });
        }
        "bench" => {
            let (w, h) = if cli.size_set { cli.opts.size } else { (3840, 2160) };
            let frames = cli.frames;
            offscreen(&cli.opts, |r| unsafe {
                use glow::HasContext;
                let c = r.canvas(w, h);
                let mut a = App::new(cli.opts.clone(), r);
                for _ in 0..10 { a.render(r, Some(c.fb), w, h); }
                r.gl.finish();
                let t0 = Instant::now();
                for _ in 0..frames { a.render(r, Some(c.fb), w, h); }
                r.gl.finish();
                let ms = t0.elapsed().as_secs_f64() * 1000.0 / frames as f64;
                println!("{w}x{h}: {ms:.2} ms per frame ({:.0} fps max), {}", 1000.0 / ms, cli.opts.look.flags());
                a.free(r);
            });
        }
        _ => usage(),
    }
}

fn offscreen(_o: &Opts, f: impl FnOnce(&gfx::Renderer)) {
    let ctx = egl::Gl::surfaceless().unwrap_or_else(|e| { eprintln!("EGL: {e}"); std::process::exit(1) });
    let r = unsafe { gfx::Renderer::new(ctx.glow()) }.unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
    f(&r);
}
