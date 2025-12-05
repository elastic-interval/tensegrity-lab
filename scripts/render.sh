#!/bin/bash

case "$1" in
    clear)
        echo "Removing PNG files from /tmp/..."
        rm -f /tmp/*.png
        echo "Done."
        ;;
    movie)
        fps="${2:-30}"
        output="${3:-movie.mp4}"
        echo "Creating movie from /tmp/*.png -> $output (${fps} fps)"
        ffmpeg -y -framerate "$fps" -pattern_type glob -i '/tmp/*.png' -c:v libx264 -pix_fmt yuv420p "$output"
        echo "Done: $output"
        ;;
    *)
        echo "Usage: $0 <command>"
        echo ""
        echo "Commands:"
        echo "  clear                  Remove all PNG files from /tmp/"
        echo "  movie [fps] [file]     Create movie from PNGs (default: 30 fps, movie.mp4)"
        ;;
esac
