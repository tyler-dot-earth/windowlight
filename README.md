# windowlight (spike)

Throwaway spike: sunlight through a window onto a wall as a live niri wallpaper, following the real sun. Inspired by Sunstill (a Mac app), built from scratch.

- `index.html` is the shader and sun math in one file. Open it in a browser with `?ui=1` for a time slider, `?speed=3600` for a time-lapse, `?style=window|blinds|lattice`, `?facing=135`, `?lat=&lon=`
- `gen-glpaper.mjs` turns that shader into a glpaper shader that computes the sun on the GPU, so it keeps following the sun without restarts
- `./run.sh` runs it live on the background layer (default output DP-2, 10 fps at 1280x720). Ctrl-C stops it
- `shot.sh` renders a still with headless Chrome, and `shots/` holds the evidence

Location defaults to 37.77, -122.42 as a Pacific-time stand-in; pass your real lat/lon.
