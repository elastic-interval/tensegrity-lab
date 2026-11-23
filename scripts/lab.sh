#!/bin/bash
# Opens the lab on one of the sculptures, at one of its sizes, over and over.
# It shows a menu; press a letter to open that one. While it is open, press
# any key here to close it and come back to the menu, or another letter to
# go straight to that one. Closing the window comes back to the menu too.
# Q quits.
#
#   Large   as designed
#   Medium  about 1.5 m, in the 2021 Headless Hug's materials, on the
#           wobble table (Quake breeds its movements)
#   Small   a 1:10 scale model, every length in model millimetres

cd "$(dirname "$0")/.." || exit 1

fabrics=("Headless Hug" "Open Claw" "Quadruped")
sizes=(large medium small)
letters=(A B C D E F G H I)

menu() {
    echo
    echo "Tensegrity Lab ($(git branch --show-current))"
    echo
    local index=0
    for fabric in "${fabrics[@]}"; do
        for size in "${sizes[@]}"; do
            printf "  %s) %-13s %s\n" "${letters[$index]}" "$fabric" "$size"
            index=$((index + 1))
        done
    done
    echo "  Q) quit"
    echo
}

# The fabric and size for a letter, or nothing for any other key.
chosen() {
    local key
    key=$(echo "$1" | tr '[:lower:]' '[:upper:]')
    for index in "${!letters[@]}"; do
        if [ "${letters[$index]}" = "$key" ]; then
            fabric="${fabrics[$((index / 3))]}"
            size="${sizes[$((index % 3))]}"
            return 0
        fi
    done
    return 1
}

# Opens `fabric` at `size`, and waits for it to close or a key to be pressed.
# A letter pressed meanwhile is left in `next`.
run() {
    echo "Building..."
    if ! cargo build --release --quiet; then
        echo "The build failed; see the errors above."
        return
    fi
    echo "Opening $fabric, $size. Press a key to close it, or another letter to switch."
    ./target/release/tensegrity_lab --fabric "$fabric" --size "$size" > /dev/null 2>&1 &
    local app=$!
    next=""
    while kill -0 "$app" 2> /dev/null; do
        if read -r -s -n 1 -t 1 key; then
            kill "$app" 2> /dev/null
            wait "$app" 2> /dev/null
            if chosen "$key"; then
                next="$key"
            fi
            return
        fi
    done
    wait "$app" 2> /dev/null
}

next=""
while true; do
    if [ -z "$next" ]; then
        menu
        read -r -s -n 1 -p "Press a letter: " key || exit 0
        echo
    else
        key="$next"
    fi
    case "$key" in
        q | Q) exit 0 ;;
    esac
    if chosen "$key"; then
        run
    else
        next=""
        echo "No $key on the menu."
    fi
done
