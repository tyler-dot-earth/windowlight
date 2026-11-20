#!/usr/bin/env bash
# Build and run the native windowlight with nix-provided tools; nothing is installed outside this folder.
# usage: ./wl.sh build | ./wl.sh run [flags] | ./wl.sh shot out.png [flags] | ./wl.sh sample 16 dir | ./wl.sh bench | ./wl.sh ctl random
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
export CARGO_HOME="$here/.cargo-home" CARGO_TARGET_DIR="$here/target"
libs=$(nix build --no-link --print-out-paths nixpkgs#libglvnd nixpkgs#wayland | grep -v -- '-dev$' | sed 's|$|/lib|' | paste -sd:)
export LD_LIBRARY_PATH="$libs${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
bin="$CARGO_TARGET_DIR/release/windowlight"
if [[ "${1:-}" == build || ! -x "$bin" || -n "$(find "$here/src" "$here/shaders" "$here/Cargo.toml" -newer "$bin" 2>/dev/null | head -1)" ]]; then
  (cd "$here" && nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#gcc -c cargo build --release -q)
fi
[[ "${1:-}" == build ]] && exit 0
# offscreen modes: Mesa would pick the integrated GPU first; prefer the discrete card (the live mode uses the compositor's GPU)
[[ "${1:-}" =~ ^(shot|sample|bench)$ ]] && export DRI_PRIME=${DRI_PRIME:-1}
exec "$bin" "$@"
