#!/bin/sh
# Rebuild only the derived icon files; never overwrite the authored leaves*.png.
set -eu
command -v magick >/dev/null 2>&1 || { echo 'ImageMagick 7 (magick) is required' >&2; exit 1; }
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source="$root/assets/icons/leaves.png"
output="$root/assets/icons"
# A neutral opaque background suits the existing dark reading shell.
background='#111111'
for size in 16 32 48; do
  magick "$source" -filter Lanczos -resize "${size}x${size}" -strip "$output/favicon-${size}x${size}.png"
done
for size in 192 512; do
  magick "$source" -filter Lanczos -resize "${size}x${size}" -strip "$output/icon-${size}x${size}.png"
done
# Apple home-screen icons should have a deliberate opaque background.
magick "$source" -filter Lanczos -resize 152x152 -background "$background" -gravity center -extent 180x180 -alpha remove -alpha off -strip "$output/apple-touch-icon.png"
# A centered 280px square fits wholly in the 512px icon's 40%-radius safe circle.
# Leave corners square: the operating system supplies its own mask.
magick "$source" -filter Lanczos -resize 280x280 -background "$background" -gravity center -extent 512x512 -alpha remove -alpha off -strip "$output/icon-maskable-512x512.png"
magick "$output/favicon-16x16.png" "$output/favicon-32x32.png" "$output/favicon-48x48.png" "$output/favicon.ico"
printf '%s\n' "Generated favicon pack in $output"
