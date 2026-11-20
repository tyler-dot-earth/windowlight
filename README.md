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
- `./run.sh` runs it live on the background layer (default output DP-2, 10 fps at 1280x720). Ctrl-C stops it. Example: `./run.sh DP-2 --style arch --plant olive --mood terracotta --facing 200 --caustic both`
- Cost on the RX 7800-class card: measured midday with olive leaves, about 3% GPU at the default and 18% at 1080p and 30 fps; caustics add about 1 and 2 points
- `shot.sh` renders a still with headless Chrome, and `shots/v4/` holds the style, mood and scene sheets and a terracotta time-lapse; `shots/v5/` holds the caustics, tilt and day sheets and `ripples.gif`

Location defaults to 37.77, -122.42 as a Pacific-time stand-in; pass your real lat/lon.

## native (round four)

`native/` is the same scene as a Rust program that draws at the monitor's full resolution through OpenGL ES, with no browser or glpaper.

- Slow parts are baked into textures: the plaster once, the window shape when the style or angle changes, and the leaves and pool ripples at a small size each frame. The full-resolution pass only does a handful of texture lookups per pixel; soft shadow edges come from blurrier mip levels
- Cost at 3840x2160 on the RX 7800 XT: 0.6 to 1.4 ms per frame offscreen (worst is olive plus both caustics), about 4% of the card at 30 fps and 9% at 60. Live on DP-2 at 30 fps the desktop's GPU load stayed in the same 5 to 10% range it shows without it
- `native/wl.sh` builds it with nix tools (cargo cache stays in the folder) and runs it:
  - `./wl.sh run --plant olive --caustic both` draws on DP-2 (`--output`, `--fps 30`, `--scale 0.5` for a cheaper half-resolution buffer)
  - `./wl.sh run --workspaces linen,sage,nocturne,terracotta` follows niri and fades the mood when the active workspace on that output changes
  - `./wl.sh run --random` picks a look at start, and `--shuffle 30` picks a new one every 30 minutes
  - `./wl.sh ctl random all`, `ctl mood sage`, `ctl set style=arch tilt=50`, `ctl get` change the running wallpaper; mood-only changes tween the colours and anything else crossfades
  - `./wl.sh ctl sleep` fades to dark and stops drawing; `ctl wake` fades back in while the sun sweeps up from 70 minutes earlier, meant for a locker's before-lock and after-unlock hooks
  - `./wl.sh sample 24 dir` renders 24 random looks in daylight with `dir/looks.txt` holding each one's flags, and `./wl.sh shot out.png --size 3840x2160 ...` renders one still
  - `./wl.sh bench` times frames at 4K
- `native/sheet-random.jpg` is a 24-look sample sheet
