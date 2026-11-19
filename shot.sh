#!/usr/bin/env bash
# usage: ./shot.sh out.png "query string" [WxH]
H=~/.cache/ms-playwright/chromium_headless_shell-1255/chrome-headless-shell-linux64/chrome-headless-shell
size=${3:-1600,900}
env -u WAYLAND_DISPLAY -u DISPLAY "$H" --headless --ozone-platform=headless --no-sandbox --enable-unsafe-swiftshader --use-angle=swiftshader --hide-scrollbars \
  --window-size="$size" --virtual-time-budget=1500 --screenshot="$1" \
  "file://$PWD/index.html?still=1&$2" 2>/dev/null >/dev/null
