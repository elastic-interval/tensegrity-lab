#!/usr/bin/env bash
#
# Deploy the WASM lab to fluxe.art: build with trunk and mirror the output
# into ../fluxe_art/lab, so the old hashed .js/.wasm files disappear and
# index.html refers to the new ones. Then commit and push fluxe_art, which
# GitHub Pages publishes.
#
# Does nothing if the lab is already deployed from the current commit, or if
# a fresh build is identical to what is deployed; then it reports whether that
# deployment succeeded: the GitHub Pages run and what the live site serves.
set -euo pipefail

cd "$(dirname "$0")/.."

FLUXE_ART="${FLUXE_ART:-../fluxe_art}"
LAB="$FLUXE_ART/lab"
DIST="target/lab-dist"

report_status() {
  local commit run wasm url
  commit="$(git -C "$FLUXE_ART" rev-parse HEAD)"
  run="$(cd "$FLUXE_ART" && gh run list --commit "$commit" --json status,conclusion,url \
    -q '.[0] | "\(.status) \(.conclusion) \(.url)"' || true)"
  echo "GitHub Pages run for fluxe_art ${commit:0:7}: ${run:-not found}"
  wasm="$(basename "$LAB"/*.wasm)"
  url="https://$(cat "$FLUXE_ART/CNAME")/lab/"
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
if [ "$(git -C "$FLUXE_ART" rev-parse HEAD)" != "$(git -C "$FLUXE_ART" rev-parse origin/main)" ]; then
  echo "$FLUXE_ART is not in sync with origin/main; pull or push first." >&2
  exit 1
fi

if git -C "$FLUXE_ART" log -1 --format=%s -- lab | grep -q "tensegrity-lab $SHA"; then
  echo "Already deployed: fluxe.art/lab is built from tensegrity-lab $SHA."
  report_status
  exit 0
fi

echo "==> Building the lab from tensegrity-lab $SHA"
rm -rf "$DIST"
trunk build --release --public-url /lab --dist "$DIST"

# wasm-bindgen emits functions in varying order, so the .js/.wasm bytes differ
# between identical builds; trunk's hashed names are stable and identify them.
same_build() {
  [ "$(ls "$DIST")" = "$(ls "$LAB")" ] || return 1
  for file in "$DIST"/*; do
    case "$(basename "$file")" in tensegrity_lab-*) continue ;; esac
    cmp -s "$file" "$LAB/$(basename "$file")" || return 1
  done
}

if same_build; then
  echo "Already deployed: a fresh build of $SHA is identical to fluxe.art/lab."
  report_status
  exit 0
fi

echo "==> Replacing $LAB"
rsync -a --delete "$DIST"/ "$LAB"/
git -C "$FLUXE_ART" add -A lab
git -C "$FLUXE_ART" status --short -- lab

git -C "$FLUXE_ART" commit -q -m "Lab: tensegrity-lab $SHA ($SUBJECT)"
git -C "$FLUXE_ART" push -q origin main
echo "==> Deployed tensegrity-lab $SHA to fluxe.art/lab (GitHub Pages publishes it shortly; rerun this script to check)."
