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
  - It shows a live preview, with buttons for every window, plant, mood and caustic, plus a rain toggle and tilt and facing sliders.
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
  - Added later, from Arab, Persian and Indian traditions:
    - `mashrabiya`: a Cairene screen of turned spindles with beads, closer-set below, a hatch open at eye level and a frieze of small arches.
    - `girih`: Persian strapwork of ten-pointed stars running on to meet their neighbours, in a pointed arch.
    - `sikri`: a Mughal marble jali of six-pointed stars on a hexagonal grid, in a cusped arch.
    - `jharokha`: a Rajput balcony window of three trefoil arches over a parapet pierced with quatrefoils.
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

## Round ten: a tidier control page

- The control page opens on a live preview of the wallpaper, a 640×360 still the app renders on request (`ctl preview` returns it as base64 PNG) and refreshes every 4 seconds.
- Shuffle, keep and sleep sit under the preview, with cat, birds, car and breeze below them.
- Window, room and light controls are grouped into their own cards. Wall moods show their colour, and facing reads as a compass direction.
- Each window and plant button shows a small picture of its shape, drawn by the wallpaper's own shaders on page load (`ctl glyphs` returns them as base64 PNGs); windows show their glass colours.
- Round eleven, the control page redesigned: the window and plant pictures are the buttons (windows as warm light on a dark wall that lights up when chosen, plants as their shadow in a patch of sun on the current wall), walls as paint chips, lamps grouped by kind with a dot of their colour (flames flicker), drawn icons instead of emoji, segmented choices and switches, and a compass for facing that shows the window's half of the sky and where the sun stands. Favourites show their window's light on their wall. Window pictures are now 120×136 for sharp screens.
- A Where card shows the sun, the earth and the moon where they are at that moment: day and night on the globe, the moon on its orbit lit from the sun, and its phase. Drag to turn the scene; click the earth (or use the arrow keys) to live there. The place is saved to `~/.config/windowlight/place` and read at start-up; `--lat`/`--lon` still override it. Coastlines are Natural Earth 1:110m land (public domain), packed by `native/land.mjs` into `native/land.txt`.
- Walls have a material of their own, separate from the paint: plaster, limewash, stucco, brick, concrete (tie holes and air pockets), boards (shiplap) or zellige (glazed tiles that catch the sun one by one). `--material M` or `ctl material brick`; a new material swaps behind a passing cloud. Each bakes its relief and its colour and gloss into a tile when chosen. The page shows each material as a swatch in the current paint. The mood "concrete" is now "pewter", and the old name still works.
- The header shows the clock, the season and where the sun is, plus badges for time-lapse, shuffling or sleep.
- Favourites read as "georgian · ficus" with a wall swatch; signals show a coloured state dot, a level bar and what each one drives.
- Replies appear as a short toast. On a phone-width screen the cards stack with the scene controls straight after the preview.
- `state` now also reports the sun's altitude and compass bearing and each mood's wall colour.
- An air card has sliders for dust, haze, wind, frost and shade, the levels only signals could set before.
  - Moving one sets it by hand, replacing what the signals say; its `×` chip, or "all back to signals", hands it back.
  - `ctl air dust 0.6`, `ctl air dust auto` and `ctl air auto` do the same; hand-set levels last until the wallpaper restarts.
  - ∿ beside a slider shapes that level across the day with one Bézier curve: drag its two handles, and the ends, which stay joined so midnight loops.
  - Signals can still push a curved level higher, a time-lapse plays the curve through, and `ctl air dust curve Y0 X1 Y1 X2 Y2` sets one from the terminal.
- A lamp gives the room a light of its own: `--lamp`, `ctl lamp NAME`, or the lamp row in the room card.
  - `neon` is a pink and teal sign across the street; each tube throws the window onto the wall, brightest after dark, and the teal one stutters now and then.
  - The rest are grow lights hung over the plant, each throwing its own plant shadow down the wall: `sodium` and `halide` are the two HIDs (amber and cool white, sharp shadows), `blurple` is red and blue diodes (shadows with red and blue fringes), `full-spectrum` is a warm-white LED panel (soft shadows) and `t5` is a row of tubes (a shadow per tube).
  - Four flames flicker, each with a flame that wanders so its shadows breathe: `candle` sits just under the plant and throws its shadow up the wall, shivering when a draught catches it; `lantern` is pierced tin, a field of swaying dots; `fireplace` is low and off to the left, several dancing plant shadows in a warm wash; `bonfire` is out in the yard, throwing the window up the wall in orange, wavering through the heat.
  - Shuffles turn one on about one look in eight; looks saved before lamps existed come back with none.

## Round twelve: weather and wear

- Sunset projectors, a new kind of lamp: `sunset` (gold fading to red), `rainbow` (gold through red and magenta to blue at the rim) and `afterglow` (peach and pink). Each one stands on the floor across the room and throws a big graded disc up the wall, with a sharp plant shadow inside it and a glow past the rim.
- Lightning: while it rains, a strike comes about every minute and a half at random. The room jumps blue-white and the window lands on the wall hard-edged, raindrops and all, often with a second or third flicker after it. `ctl bolt` sends one now.
- Heat shimmer: warm air rising off a radiator under the window makes the shadows waver and faint bright ripples climb through the patch, strongest just above the sill. It runs a little all winter, and `heat` is a new air level and signal behaviour, so a signal or the slider can turn it up.
- Sun fade: the paint is paler and duller where the window's light falls over a year, a soft ghost of the patch that shows best when the light is elsewhere. It is worked out for the window, its angle and facing and the place, from 72 sun positions across six dates, and redrawn only when one of those changes.
- Cost at 4K: about 1.6 ms per frame at the bench default.

## Round thirteen: the world outside

- A tree outside: `oak`, `birch`, `willow` or `pine`, picked on the page under the plants or with `--tree`. It stands about four units past the wall, so every gap in its leaves acts as a pinhole and throws a small round image of the sun (or the moon). Where gaps crowd, the images run into soft washes; haze makes every spot bigger and softer.
- Each tree has its own character. The oak has a broad, lumpy crown on thick limbs. The birch is open and its small leaves shiver fast. The willow's strands hang from a high dome and swing in the wind. The pine has dark needled shelves.
- Seasons: leaves are thin in spring, full in summer, thinning and amber in autumn. In winter only the bare limbs and twigs remain, apart from the pine. The tree sways with the wind and gusts, the top most.
- Its shade also dims the glow round the patch and the shaft in the air.
- Five more plants: `fern` (a Boston fern in a hanging pot, fronds arching over and falling), `pothos` (heart-leaved vines trailing from a hanging pot), `paradise` (bird of paradise: tall torn paddle leaves and one crane-headed flower), `pearls` (string of pearls, strands of little round beads) and `fiddle` (fiddle-leaf fig, big violin-shaped leaves on a straight stem). The hanging ones swing a little on their cords.
- Small things for scale: a light switch left of the window, standing 9 mm proud so it casts a crisp little shadow when the patch reaches it; a brass picture hook high on the right with an unfaded rectangle below it where a frame used to hang; a nail hole near the patch.
- Tiny life (`--life on`, or the toggle under extras): an old orb web fills the window's top corner. The silk is too fine to see, but the sun draws it on the wall as faint dusty threads that breathe in the draft, with the spider a soft dot at the hub that now and then walks out along a spoke. In spring and summer a dandelion seed sometimes blows in and drifts across the room, its rays glowing white when it crosses the shaft, its ring of a shadow sliding over the patch. Costs about 0.24 ms a frame at 4K.
- A shelf (`--shelf on`, or the toggle under extras): a floating oak board with a glass of water and a small round mirror on a brass stand, ray-traced from where we stand and lit by the room, any lamp and the sun through the window. The board throws its shadow down the wall, and leaves and frames fall across it. The glass is a fat lens: its shadow has dark edges, a bright line down the middle and a sharp water line, with a gold spot on the board by its foot. The mirror is turned toward the window; in the morning it throws a second, softer oval of sun out onto the dark wall, and the spot slides as the sun moves. Costs about 0.2 ms a frame at 4K.
- A corner (`--corner on`, or the toggle under extras): a side wall comes toward us on the left, with a painted skirting board along both walls and oak floorboards below. Each is lit through the same window. Near sunrise the patch reaches the corner and folds onto the side wall, squeezed thin by the angle, and the patch warms the side wall a little by bounce. Later in the morning its foot crosses the skirting and spills forward onto the boards, which reflect the lit wall softly. The shelf mirror's spot folds into the corner too. Costs about 0.4 ms a frame at 4K.
- The wall details have their own switches (`--switch on|off`, `--hook on|off`, or the toggles under extras): the light switch, and the picture hook with the ghost of its frame and the old nail hole. Both are on by default.
- Golden hour is several effects at once, not one tint. Sunlight is coloured by how much air it crosses (Kasten–Young air mass, with Rayleigh, haze and ozone absorption), so it goes gold, then orange, then coral, and dims as it goes. Shade stays sky-blue, so the warm patch sits on a cooler wall. Evening air is hazier than morning air. In the last minutes the disc sinks behind the skyline, so the light fades and its edges sharpen. After sunset a rose afterglow gives way to blue hour, and in very dim light colours shift toward blue the way eyes see at dusk (the Purkinje shift). The control page's "golden hour" menu jumps to this morning's or this evening's in real time, and `golden sunrise|sunset` does the same from `ctl`. The time-lapse menu has `sunrise/1m`, `sunset/1m` and `sunset/3m` loops, and `--timelapse sunset/2m` works too.
- The pinhole blind (`--pinhole on|off`, or the toggle under extras) turns the room into a camera obscura. A blackout blind with a small hole covers the window, the room goes dim, and the wall shows the outdoors upside down and mirrored: sky and clouds, the sun as a soft blob (the moon at night), a row of houses across the street with lit windows after dark, the road, the tree and the plant. The hole is about 2 cm, so the picture is soft the way real ones are, which also hides how simple the scene behind it is. It costs about 0.45 ms a frame at 4K, only while it is on.
- Cost at 4K: about 0.1 ms more per frame with a tree (1.9 ms at the bench default with an oak).

## Setup with Nix

- `nix develop` (or direnv, through `.envrc`) opens a dev shell with Rust, clippy, jq, Node, glpaper and Chromium, plus the GL and Wayland libraries on `LD_LIBRARY_PATH`. `wl.sh`, `run.sh` and `shot.sh` enter it themselves.
- `nix build` (or `nix run . -- run`) builds a packaged `windowlight` plus `windowlight-adapter-herdr` and `windowlight-adapter-system`, with nothing tied to this checkout.
- Your own settings live in `~/.config/windowlight/`: `favourites.txt`, and `signals.map` if you want a different mapping. `WINDOWLIGHT_FAVS` and `WINDOWLIGHT_SIGNAL_MAP` point elsewhere.
- Linux and Wayland only; the wallpaper needs a compositor with layer-shell, and workspace moods and breeze need niri.
