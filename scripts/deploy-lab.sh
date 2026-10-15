#!/usr/bin/env bash
#
# Deploy the WASM lab to fluxe.art: build with trunk and mirror the output
# into ../fluxe_art/lab, so the old hashed .js/.wasm files disappear and
# index.html refers to the new ones. Then commit and push fluxe_art, which
# GitHub Pages publishes.
#
# Does nothing if the lab is already deployed from the current commit, or if
# a fresh build is identical to what is deployed.
set -euo pipefail

cd "$(dirname "$0")/.."

FLUXE_ART="${FLUXE_ART:-../fluxe_art}"
LAB="$FLUXE_ART/lab"
DIST="target/lab-dist"

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
  exit 0
fi

echo "==> Replacing $LAB"
rsync -a --delete "$DIST"/ "$LAB"/
git -C "$FLUXE_ART" add -A lab
git -C "$FLUXE_ART" status --short -- lab

git -C "$FLUXE_ART" commit -q -m "Lab: tensegrity-lab $SHA ($SUBJECT)"
git -C "$FLUXE_ART" push -q origin main
echo "==> Deployed tensegrity-lab $SHA to fluxe.art/lab (GitHub Pages publishes it shortly)."
