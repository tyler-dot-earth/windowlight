#!/usr/bin/env bash
# Run windowlight as a live niri wallpaper on one output. Ctrl-C to stop; nothing is installed or configured.
# usage: ./run.sh [output] [extra gen-glpaper args, e.g. --style window --facing 135 --lat 47.6 --lon -122.3]
# cheap default: 10 fps rendered at 1280x720 (about 1% GPU on the RX 7800-class card), upscaled to the monitor
set -euo pipefail
cd "$(dirname "$0")"
out=${1:?usage: ./run.sh OUTPUT [gen-glpaper args], e.g. ./run.sh DP-1}; shift
node gen-glpaper.mjs "$@" > windowlight.frag
exec nix run nixpkgs#glpaper -- "$out" windowlight.frag --fps "${FPS:-10}" -W "${W:-1280}" -H "${H:-720}"
