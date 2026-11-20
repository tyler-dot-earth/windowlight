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
mod sun;
mod wl;

use app::{App, Opts};
use look::{Look, Rng};
use std::time::Instant;

fn usage() -> ! {
    eprintln!("usage: windowlight run|shot OUT.png|sample N DIR|bench|ctl WORDS...\n\
  look:   --style S --plant P --mood M --caustic C --tilt DEG --facing DEG --random\n\
  place:  --lat X --lon Y\n\
  time:   --at 2026-11-19T11:00 --speed X --anim SECONDS\n\
  live:   --output DP-2 --fps 30 --scale 1.0 --workspaces linen,sage,... --shuffle MINUTES\n\
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

struct Cli { opts: Opts, pos: Vec<String>, random: bool, frames: usize, size_set: bool }

fn parse(args: &[String]) -> Cli {
    let mut o = Opts {
        lat: 37.77, lon: -122.42, look: Look::default(), speed: 1.0, at_ms: None, anim: 0.0,
        fps: 30.0, scale: 1.0, output: "DP-2".into(), workspaces: vec![], shuffle_min: None, size: (1920, 1080),
    };
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
            "shuffle" => o.shuffle_min = Some(num()),
            "frames" => frames = num() as usize,
            "at" => o.at_ms = Some(parse_at(v)),
            "output" => o.output = v.clone(),
            "size" => {
                let (w, h) = v.split_once('x').unwrap_or_else(|| usage());
                o.size = (w.parse().unwrap_or_else(|_| usage()), h.parse().unwrap_or_else(|_| usage()));
                size_set = true;
            }
            "workspaces" => {
                o.workspaces = v.split(',').map(|m| look::MOODS.iter().position(|x| *x == m).unwrap_or_else(|| {
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
    Cli { opts: o, pos, random, frames, size_set }
}

fn write_png(path: &str, w: i32, h: i32, rgb: &[u8]) {
    let f = std::fs::File::create(path).unwrap_or_else(|e| { eprintln!("{path}: {e}"); std::process::exit(1) });
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgb).unwrap();
}

/// a random facing that puts the sun through the window at time `ms`
fn sunny_facing(rng: &mut Rng, ms: f64, o: &Opts) -> f64 {
    let (alt, az) = sun::position(ms, o.lat, o.lon);
    if alt > 0.0 { az.to_degrees() + (rng.f() - 0.5) * 80.0 } else { 90.0 + rng.f() * 180.0 }
}

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
    let mut cli = parse(&args[1..]);
    let mut rng = Rng::seeded();
    if cli.random {
        let ms = cli.opts.at_ms.unwrap_or_else(app::now_ms);
        let f = sunny_facing(&mut rng, ms, &cli.opts);
        cli.opts.look = Look::random(&mut rng, Some(f), &cli.opts.look);
        eprintln!("look: {}", cli.opts.look.flags());
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
                let today = chrono::Local::now().date_naive();
                for i in 0..n {
                    let mut o = cli.opts.clone();
                    // a random time today with the sun 5 to 55 degrees up, unless --at pins it
                    let ms = o.at_ms.unwrap_or_else(|| {
                        use chrono::TimeZone;
                        let day0 = chrono::Local.from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap()).earliest().unwrap().timestamp_millis() as f64;
                        let mut ms = day0 + 12.0 * 3.6e6;
                        for _ in 0..500 {
                            let t = day0 + rng.f() * 86.4e6;
                            let alt = sun::position(t, o.lat, o.lon).0.to_degrees();
                            if (5.0..55.0).contains(&alt) { ms = t; break; }
                        }
                        ms
                    });
                    o.at_ms = Some(ms);
                    o.anim = rng.f() * 600.0;
                    let f = sunny_facing(&mut rng, ms, &o);
                    o.look = Look::random(&mut rng, Some(f), &o.look);
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
