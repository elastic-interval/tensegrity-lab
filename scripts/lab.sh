#!/bin/bash
# Opens the lab on one of the sculptures. Run it with no arguments for a
# menu, or name one straight away:
#
#   scripts/lab.sh            menu
#   scripts/lab.sh hh         Headless Hug
#   scripts/lab.sh oc         Open Claw
#   scripts/lab.sh quad       Quadruped (on the quadruped branch)
#   scripts/lab.sh model      Headless Hug as a 1:10 scale model
#   scripts/lab.sh model 12   ... or any other scale, here 1:12

cd "$(dirname "$0")/.." || exit 1

choice="$1"
scale="${2:-10}"

if [ -z "$choice" ]; then
    echo "Tensegrity Lab ($(git branch --show-current))"
    echo
    echo "  1) Headless Hug"
    echo "  2) Open Claw"
    echo "  3) Quadruped"
    echo "  4) Headless Hug as a scale model (1:10)"
    echo
    read -r -p "Which one? [1-4] " choice
fi

case "$choice" in
    1 | hh) fabric="Headless Hug"; title="Headless Hug" ;;
    2 | oc) fabric="Open Claw"; title="Open Claw" ;;
    3 | quad) fabric="Quadruped"; title="Quadruped" ;;
    4 | model) fabric="Headless Hug"; title="Headless Hug, scale model 1:$scale"; model=1 ;;
    *)
        echo "Choose 1-4, or one of: hh, oc, quad, model [scale]"
        exit 1
        ;;
esac

if [ "$fabric" = "Quadruped" ] && ! grep -q "Quadruped" src/build/dsl/fabric_library.rs; then
    echo "The Quadruped is on the quadruped branch, and this is $(git branch --show-current)."
    echo "Switch with:  git checkout quadruped"
    exit 1
fi

echo "Building..."
if ! cargo build --release --quiet; then
    echo "The build failed; see the errors above."
    exit 1
fi

echo "Opening $title. Close the window to come back here."
if [ -n "$model" ]; then
    ./target/release/tensegrity_lab --fabric "$fabric" --model-scale "$scale"
else
    ./target/release/tensegrity_lab --fabric "$fabric"
fi
