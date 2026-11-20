# windowlight (spike)

Throwaway spike: sunlight through a window onto a wall as a live niri wallpaper, following the real sun. Inspired by Sunstill (a Mac app), built from scratch.

- `index.html` is the shader and sun math in one file. Open it in a browser with `?ui=1` for a time slider and pickers, or `?speed=3600` for a time-lapse
  - `style`: window, blinds, lattice, georgian, arch, shutters, sheer
  - `plant`: none, ficus, monstera, palm, olive
  - `mood`: linen, terracotta, sage, nocturne, blush, concrete, overcast, noir
  - `caustic`: none, pool (sun bouncing off a pool below the window, rippling higher on the wall), glass (a water carafe on the sill with a focused, rainbow-edged spot), both
  - `tilt`: the window angle against the wall in degrees (default 32, 0 faces the wall straight on)
  - `facing` (window compass direction), `lat`, `lon`, `t` (fixed time)
- Rendering: linear light with ACES tonemapping, bump-lit plaster, warm penumbra fringes, halation, room bounce in the wall colour, faint green light through leaves, dust motes, blue hour, and a full-moon stand-in at night. The view pans to keep the light framed, and the plant stays in front of the window. Plants have a second, farther layer of leaves whose shadows are softer
- `gen-glpaper.mjs` turns that shader into a glpaper shader that computes the sun on the GPU, so it keeps following the sun without restarts
- `./run.sh OUTPUT` runs it live on the background layer (10 fps at 1280x720). Ctrl-C stops it. Example: `./run.sh DP-1 --style arch --plant olive --mood terracotta --facing 200 --caustic both`
- Cost on the RX 7800-class card: measured midday with olive leaves, about 3% GPU at the default and 18% at 1080p and 30 fps; caustics add about 1 and 2 points
- `shot.sh` renders a still with headless Chrome into `shots/`, which git ignores along with every other image

Location defaults to 37.77, -122.42 as a Pacific-time stand-in; pass your real lat/lon.

## native (round four)

`native/` is the same scene as a Rust program that draws at the monitor's full resolution through OpenGL ES, with no browser or glpaper.

- Slow parts are baked into textures: the plaster once, the window shape when the style or angle changes, and the leaves and pool ripples at a small size each frame. The full-resolution pass only does a handful of texture lookups per pixel; soft shadow edges come from blurrier mip levels
- Cost at 3840x2160 on the RX 7800 XT: 0.6 to 1.4 ms per frame offscreen (worst is olive plus both caustics), about 4% of the card at 30 fps and 9% at 60. Live on DP-2 at 30 fps the desktop's GPU load stayed in the same 5 to 10% range it shows without it
- `native/wl.sh` builds it inside the flake's dev shell (cargo cache stays in the folder) and runs it:
  - `./wl.sh run --plant olive --caustic both` draws on the first monitor (`--output NAME` picks one, `--fps 30`, `--scale 0.5` for a cheaper half-resolution buffer)
  - `./wl.sh run --workspaces linen,sage,nocturne,terracotta` follows niri and fades the mood when the active workspace on that output changes
  - `./wl.sh run --random` picks a look at start, and `--shuffle 20s` (or `5m`, `1h`) crossfades to a new random look on that interval; `ctl set shuffle=10s` or `shuffle=off` changes it while running
  - `./wl.sh ctl random all`, `ctl mood sage`, `ctl set style=arch tilt=50`, `ctl get` change the running wallpaper; mood-only changes tween the colours and anything else crossfades
  - `./wl.sh run --timelapse day/2m` loops sunrise to sunset every 2 minutes (`full/5m` for midnight to midnight, `07:00-18:00/90s` for a custom range); `ctl set timelapse=off` or `ctl now` goes back to the real sun
  - When the real light is too low to show anything, `ctl random all` and `--shuffle` jump to a sunny moment today, and night windows face the moon
  - `./wl.sh ctl sleep` fades to dark and stops drawing; `ctl wake` fades back in while the sun sweeps up from 70 minutes earlier, meant for a locker's before-lock and after-unlock hooks
  - `./wl.sh sample 24 dir` renders 24 random looks in daylight with `dir/looks.txt` holding each one's flags, and `./wl.sh shot out.png --size 3840x2160 ...` renders one still
  - `./wl.sh bench` times frames at 4K
- `./wl.sh sample 24 dir` makes a random-look sample sheet; renders and screenshots stay out of git

## native (round five)

- Control page at <http://127.0.0.1:7417/> while `./wl.sh run` is up (`--ui PORT` or `--ui off`):
  - It has buttons for every window, plant, mood and caustic, plus a rain toggle and tilt and facing sliders.
  - It also has shuffle, time-lapse and cat-interval pickers, and keep, sleep and real-sun buttons.
  - It answers only requests addressed to localhost and refuses commands sent from other sites.
- Four more windows: `porthole`, `stained` (coloured light through jewel panes and a rose), `slats` (boards with knots) and `jali` (a carved star screen).
- Rain on the glass, set with `--rain on` or `ctl rain on`:
  - Each bead throws a dark-rimmed shadow with a bright core.
  - Drops slide down and leave trails, and rain softens the light a little.
  - `--random` turns rain on 15% of the time.
- Breeze: niri activity blows wind through the leaves and the sheer curtain.
  - Switching workspace gusts hardest, then opening, closing and focusing windows.
  - `--breeze off` turns it off, and `ctl gust 0.8` sends a gust by hand.
- The cat walks along the sill, usually sits a while, then walks off.
  - It comes about every 10 minutes at random, starting a minute or three after launch.
  - `--cat 30m` or `--cat off` changes that, and `ctl cat` calls it now.
- Favourites:
  - `ctl keep` saves the current look to `~/.config/windowlight/favourites.txt`, and `ctl drop` removes it.
  - `ctl favs` lists them, and `ctl fav 3` shows one, turned toward the light if it would be dark as saved.
  - `--pool favourites` (or `ctl set pool=favourites`) makes shuffles draw only from them.
- Transitions replace the crossfade:
  - Mood, tilt, facing and rain glide in place.
  - A new window, plant or caustic waits behind a passing cloud: the light fades and softens, the scene swaps while it's dark, and the light comes back (about 3.5 s).
  - A shuffle that has to jump the clock to a sunny moment also hides the jump behind the cloud.
- Cost at 4K is still small: 1.1 to 1.8 ms per frame. Rain and a walking cat redraw the window texture each frame, which adds about 0.5 ms.
- `./wl.sh headless --ui 7899 --breeze on` is the offscreen test driver. It serves the page and follows niri while it `wait`s, and `--size` sets its shot size.

## native (round six)

- Nine historical windows, all original drawings in each period's style:
  - `rose`: a Gothic rose with twelve lancet petals, a quatrefoil centre, foils and roundels, in blues, rubies and gold.
  - `tracery`: a Gothic two-light window with bar tracery, a sexfoil oculus, trefoils and diamond quarries.
  - `palladian`: a Venetian window with a round-headed centre light, a fanlight and flat side lights.
  - `shoji`: Japanese paper screens on a kumiko grid, one slid back so part of the window is bare.
  - `moorish`: a Nasrid twin horseshoe arch on a slender column, with a celosia of round holes above.
  - `prairie`: paired art-glass casements with chevron leading and small squares of colour.
  - `moon`: a Chinese garden moon window filled with ice-ray lattice.
  - `persian`: a Qajar orosi with three pointed sashes and a star lattice of jewel-coloured glass.
  - `nouveau`: Art Nouveau stems rising to stylised roses, with leaves and an amber band.
- `--pool historic` (or `ctl set pool=historic`) makes shuffles draw only from these, and the page has its own row of them.
- Cost at 4K: 1.1 to 1.6 ms per frame with rain, a plant and both caustics.

## native (round seven: pretty)

- Suncatcher: `--prism on` (or the page toggle) hangs a slowly turning crystal in the window, which throws small rainbow spots around its shadow on the wall.
- Light shaft and dust: when the sun is out, a faint beam crosses the room with drifting specks in it; haze softens both.
- Things passing outside: birds fly past by day and car headlights sweep the wall at night, every few minutes on average.
  - `--passing 4m|off` (or `set passing=`) sets how often, and `ctl bird` / `ctl car` send one now.
- Seasons follow the date and latitude, or can be pinned with `--season spring|summer|autumn|winter`:
  - spring drops pink petals and greens the leaves;
  - autumn turns the leaves amber and drops falling leaves;
  - winter frosts the glass from the edges and puts snow on the sill.
- Cost at 4K: about 0.4 ms per frame on top of round six, worst case 2.4 ms.
- Weak spots:
  - frost follows the rectangular frame even on arched windows;
  - snow shows in winter whatever the local climate;
  - none of this has been watched live yet.

## native (round eight: signals)

- Anything can drive the window by dropping a small file in `$XDG_RUNTIME_DIR/windowlight/signals/`, named after the signal:
  - `{"level": 0.4, "state": "busy", "label": "what it is", "ttl": 15}`, every field optional;
  - or just `0.4`, or just a state word: `quiet`, `busy`, `waiting`, `done` or `alert`.
- `native/signals.map` says which behaviour each signal drives, one `pattern behaviour [from..to]` per line. It is built into the binary; a copy at `~/.config/windowlight/signals.map` overrides it and reloads on save.
  - Any signal can drive any of: `birds`, `wind`, `dust`, `frost`, `haze`, `rain`, `shade`, `suncatcher`, `passing` or `cat`.
  - Every behaviour takes a level, and reacts when a signal turns done or alert or jumps past 75%.
  - `from..to` uses part of a level: `disk frost 0.8..1` frosts the glass only past 80% full.
- Birds get one per signal: circling outside while busy, pecking at the glass while waiting, sitting on the sill when done, flying off when quiet or on alert.
- Default adapters, each a short script that only reads:
  - `native/adapters/herdr`: one `herdr-<agent>` signal per Herdr agent, from `herdr agent list` every 2 s.
  - `native/adapters/system`: `cpu`, `net` (log scale, 1 KB/s to 100 MB/s) and `disk` (share of / used).
- The default map: Herdr agents are birds, CPU is wind, network is dust, and a disk past 80% is frost.
- `ctl signal NAME VALUE [ttl]` writes a signal by hand, `ctl signals` lists live ones, and the page has a signals list.
- `--signals DIR|off` and `--signal-map FILE` move or turn them off.
- Cost at 4K: about 0.1 ms per frame with six birds.
- Weak spots:
  - birds only show where sunlight or moonlight falls, so an overcast or dark scene hides them;
  - perched birds float on round windows with no straight sill;
  - OpenMausBot bots need to write their own signal files; nothing reads the app's internals.

## Round nine: sharper plaster

- The wall's plaster was baked into a 2048-pixel tile that a 4K screen stretched about 1.5×, and its finest noise sat on a square grid about 9 pixels wide, so raking light showed blocky cells.
- Now the plaster bakes as height only at 4096 per tile (2048 texels per wall unit, finer than 4K draws it), in the same memory as before.
- The slope is taken per screen pixel when drawing, and the noise is gradient noise, so no grid shows.
- The live wallpaper picks the first monitor unless `--output NAME` names one; `run.sh` (the old glpaper path) now needs the output name.

## Setup with Nix

- `nix develop` (or direnv, through `.envrc`) opens a dev shell with Rust, clippy, jq, Node, glpaper and Chromium, plus the GL and Wayland libraries on `LD_LIBRARY_PATH`. `wl.sh`, `run.sh` and `shot.sh` enter it themselves.
- `nix build` (or `nix run . -- run`) builds a packaged `windowlight` plus `windowlight-adapter-herdr` and `windowlight-adapter-system`, with nothing tied to this checkout.
- Your own settings live in `~/.config/windowlight/`: `favourites.txt`, and `signals.map` if you want a different mapping. `WINDOWLIGHT_FAVS` and `WINDOWLIGHT_SIGNAL_MAP` point elsewhere.
- Linux and Wayland only; the wallpaper needs a compositor with layer-shell, and workspace moods and breeze need niri.
