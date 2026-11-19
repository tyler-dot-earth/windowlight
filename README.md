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
