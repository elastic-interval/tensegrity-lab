#!/usr/bin/env bash
#
# Build a standalone macOS .app for the Open Claw.
#
# The native binary compiles all its assets in (include_bytes!) and
# defaults to FabricName::OpenClaw when launched with no arguments, so the
# bundle is fully self-contained: double-clicking it opens straight into
# the Open Claw. Re-run this script only when you want to rebuild (e.g.
# after a source or macOS change).
#
# Uses only macOS built-ins: cargo, qlmanage, sips, iconutil, codesign.
set -euo pipefail

cd "$(dirname "$0")/.."

APP_NAME="Open Claw"
BIN_NAME="tensegrity_lab"
IDENTIFIER="com.pretenst.open-claw"
VERSION="0.1.0"
ICON_SVG="assets/open-claw-icon.svg"

STAGE="target/macos"
APP="$STAGE/$APP_NAME.app"
CONTENTS="$APP/Contents"

echo "==> Building release binary"
cargo build --release

echo "==> Generating icon from $ICON_SVG"
ICON_TMP="$(mktemp -d)"
ICONSET="$ICON_TMP/AppIcon.iconset"
mkdir -p "$ICONSET"
# Rasterise the SVG to a large PNG via Quick Look, then downscale.
qlmanage -t -s 1024 -o "$ICON_TMP" "$ICON_SVG" >/dev/null 2>&1
MASTER="$ICON_TMP/$(basename "$ICON_SVG").png"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size"                 "$MASTER" --out "$ICONSET/icon_${size}x${size}.png"     >/dev/null
  sips -z $((size*2)) $((size*2))         "$MASTER" --out "$ICONSET/icon_${size}x${size}@2x.png"  >/dev/null
done
iconutil -c icns "$ICONSET" -o "$ICON_TMP/AppIcon.icns"

echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"
cp "target/release/$BIN_NAME" "$CONTENTS/MacOS/$BIN_NAME"
chmod +x "$CONTENTS/MacOS/$BIN_NAME"
cp "$ICON_TMP/AppIcon.icns" "$CONTENTS/Resources/AppIcon.icns"

cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>            <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>     <string>$APP_NAME</string>
    <key>CFBundleExecutable</key>      <string>$BIN_NAME</string>
    <key>CFBundleIdentifier</key>      <string>$IDENTIFIER</string>
    <key>CFBundleVersion</key>         <string>$VERSION</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundlePackageType</key>     <string>APPL</string>
    <key>CFBundleIconFile</key>        <string>AppIcon</string>
    <key>NSHighResolutionCapable</key> <true/>
    <key>NSPrincipalClass</key>        <string>NSApplication</string>
    <key>LSMinimumSystemVersion</key>  <string>11.0</string>
</dict>
</plist>
PLIST

# Ad-hoc sign so recent macOS launches it without complaint.
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true

DEST="$HOME/Desktop/$APP_NAME.app"
echo "==> Installing to $DEST"
rm -rf "$DEST"
cp -R "$APP" "$DEST"

rm -rf "$ICON_TMP"
echo "==> Done. '$APP_NAME' is on your Desktop (also at $APP)."
