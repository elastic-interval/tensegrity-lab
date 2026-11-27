#!/usr/bin/env bash
#
# Deploy the gallery to fluxe.art as its front page: build with trunk for
# /gallery/ and mirror the output into ../fluxe_art/gallery, so the old
# hashed .js/.wasm files disappear, and copy its index.html to the site's
# root, where its <base href="/gallery/"> finds them. Then commit and push
# fluxe_art, which GitHub Pages publishes.
#
# Does nothing if the gallery is already deployed from the current commit, or if
# a fresh build is identical to what is deployed; then it reports whether that
# deployment succeeded: the GitHub Pages run and what the live site serves.
set -euo pipefail

cd "$(dirname "$0")/.."

FLUXE_ART="${FLUXE_ART:-../fluxe_art}"
LAB="$FLUXE_ART/gallery"
DIST="target/gallery-dist"

report_status() {
  local commit run wasm url
  commit="$(git -C "$FLUXE_ART" rev-parse HEAD)"
  run="$(cd "$FLUXE_ART" && gh run list --commit "$commit" --json status,conclusion,url \
    -q '.[0] | "\(.status) \(.conclusion) \(.url)"' || true)"
  echo "GitHub Pages run for fluxe_art ${commit:0:7}: ${run:-not found}"
  wasm="$(basename "$LAB"/*.wasm)"
  url="https://$(cat "$FLUXE_ART/CNAME")/"
  if curl -fsL -H 'Cache-Control: no-cache' "$url" | grep -q "$wasm"; then
    echo "Live: $url serves $wasm."
  else
    echo "Not live: $url does not refer to $wasm." >&2
    return 1
  fi
}

if [ -n "$(git status --porcelain)" ]; then
  echo "tensegrity-lab has uncommitted changes; commit them first so the deployment matches a commit." >&2
  exit 1
fi
SHA="$(git rev-parse --short HEAD)"
SUBJECT="$(git log -1 --format=%s)"

if [ "$(git -C "$FLUXE_ART" branch --show-current)" != "main" ] || [ -n "$(git -C "$FLUXE_ART" status --porcelain)" ]; then
  echo "$FLUXE_ART must be clean and on main." >&2
  exit 1
fi
git -C "$FLUXE_ART" fetch -q origin main
# It may be ahead, with site changes to go out with the gallery, never behind.
if ! git -C "$FLUXE_ART" merge-base --is-ancestor origin/main HEAD; then
  echo "$FLUXE_ART is behind origin/main; pull first." >&2
  exit 1
fi

if git -C "$FLUXE_ART" log -1 --format=%s -- gallery | grep -q "tensegrity-lab $SHA"; then
  echo "Already deployed: fluxe.art's gallery is built from tensegrity-lab $SHA."
  report_status
  exit 0
fi

echo "==> Building the gallery from tensegrity-lab $SHA"
rm -rf "$DIST"
trunk build --release --public-url /gallery/ --dist "$DIST"

# wasm-bindgen emits functions in varying order, so the .js/.wasm bytes differ
# between identical builds; trunk's hashed names are stable and identify them.
same_build() {
  [ -d "$LAB" ] && [ "$(ls "$DIST")" = "$(ls "$LAB")" ] || return 1
  for file in "$DIST"/*; do
    case "$(basename "$file")" in tensegrity_lab-*) continue ;; esac
    cmp -s "$file" "$LAB/$(basename "$file")" || return 1
  done
}

if same_build; then
  echo "Already deployed: a fresh build of $SHA is identical to fluxe.art's gallery."
  report_status
  exit 0
fi

echo "==> Replacing $LAB"
rsync -a --delete "$DIST"/ "$LAB"/
cp "$DIST/index.html" "$FLUXE_ART/index.html"
git -C "$FLUXE_ART" add -A gallery index.html
git -C "$FLUXE_ART" status --short -- gallery index.html

git -C "$FLUXE_ART" commit -q -m "Gallery: tensegrity-lab $SHA ($SUBJECT)"
git -C "$FLUXE_ART" push -q origin main
echo "==> Deployed tensegrity-lab $SHA to fluxe.art (GitHub Pages publishes it shortly; rerun this script to check)."
