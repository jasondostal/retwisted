#!/bin/bash
# Build the macOS .saver bundle: ALL the retwisted modules in one bundle,
# picked from the Options… sheet's Module popup.
#
#   tools/build_saver.sh                    # -> target/saver/Retwisted.saver
#   tools/build_saver.sh shock-clocks       # same bundle, different DEFAULT module
#   tools/build_saver.sh --install          # build, then install to ~/Library/Screen Savers
#   BUNDLE_NAME=Twisted tools/build_saver.sh --install
#   tools/build_saver.sh --no-packs         # the SHIPPABLE bundle: no packs inside
#   BUNDLE_NAME="Retwisted First Run" BUNDLE_ID=com.retwisted.saver.firstrun \
#       tools/build_saver.sh --no-packs     # ...side by side with an installed one
#
# --no-packs: no Berkeley asset is copied into the bundle (Resources/assets
# is absent); the Options panel asks for the user's own Totally Twisted
# download and rips it in-process into Application Support (docs/saver.md,
# "First run"). The build FAILS if any pack file ends up inside.
#
# A BUNDLE_NAME other than the default also gets its own Objective-C
# principal class and Swift module name (derived from the name), so two
# builds can be installed and loaded into ONE legacyScreenSaver process
# without their classes colliding.
#
# The positional argument is only the default selection (Info.plist's
# RTWModuleSlug) — what a machine with nothing stored comes up running. Every
# module the ABI catalogues and has a local pack for ships in the bundle.
#
# The bundle is self-contained: the Rust staticlib is linked into the bundle
# binary and every asset pack is copied into Contents/Resources/assets/<slug>.
# Assets are NOT in git (see .gitignore) — build from your own local packs,
# which is also why this script refuses to build a bundle with no pack for
# the default module.
#
# See docs/saver.md for the macOS-side gotchas (legacyScreenSaver, caching,
# signing, the keys System Settings actually reads).
set -euo pipefail

SLUG=""
INSTALL=0
NO_PACKS=0
for a in "$@"; do
    case "$a" in
        --install) INSTALL=1 ;;
        --no-packs) NO_PACKS=1 ;;
        -h|--help) sed -n '2,34p' "$0"; exit 0 ;;
        -*) echo "unknown flag: $a" >&2; exit 2 ;;
        *) SLUG="$a" ;;
    esac
done
SLUG="${SLUG:-bungee-roulette}"

# Bundle identity. NAME is the working title — rename here when the project
# gets its real one; the bundle id must stay stable across rebuilds or macOS
# treats the new build as a different saver (see docs/saver.md).
NAME="${BUNDLE_NAME:-Retwisted}"
BUNDLE_ID="${BUNDLE_ID:-com.retwisted.saver}"
VERSION="0.1.0"
DEPLOY_TARGET="13.0"
PRINCIPAL_CLASS="RetwistedSaverView"
SWIFT_MODULE="RetwistedSaver"
if [ "$NAME" != "Retwisted" ]; then
    SAFE="$(printf '%s' "$NAME" | tr -cd 'A-Za-z0-9')"
    [ -n "$SAFE" ] || { echo "error: BUNDLE_NAME needs a letter or digit" >&2; exit 2; }
    SWIFT_MODULE="${SAFE}Saver"
    PRINCIPAL_CLASS="${SAFE}SaverView"
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="$ROOT/target/saver"
BUNDLE="$OUT/$NAME.saver"
WORK="$ROOT/target/saver-build"
ARCH="$(uname -m)"                      # arm64 on Apple Silicon
TARGET_TRIPLE="$ARCH-apple-macos$DEPLOY_TARGET"
SDK="$(xcrun --show-sdk-path)"

if [ "$NO_PACKS" = "0" ] && [ ! -f "$ROOT/assets/$SLUG/meta.json" ]; then
    echo "error: no asset pack at assets/$SLUG (the default module)" >&2
    echo "       rip your own download first: cargo run --release -p ripper --bin twistedrip -- <download> --out assets" >&2
    echo "       or build the pack-less bundle: tools/build_saver.sh --no-packs" >&2
    exit 1
fi

echo "==> all modules -> $NAME.saver ($TARGET_TRIPLE), default $SLUG$([ "$NO_PACKS" = "1" ] && echo ", NO PACKS (first run rips)")"

# 1. the Rust runtime, as a static library
cargo build --release -p saver
LIB="$ROOT/target/release/libretwisted_saver.a"
[ -f "$LIB" ] || { echo "error: $LIB missing" >&2; exit 1; }

# 2. the Swift ScreenSaverView -> one object file
rm -rf "$WORK"; mkdir -p "$WORK"
# The ObjC name is spelled once, in the source's @objc(...); a renamed build
# compiles a copy with its own (see the header comment).
SRC="$ROOT/saver/macos/RetwistedSaverView.swift"
if [ "$PRINCIPAL_CLASS" != "RetwistedSaverView" ]; then
    sed "s/@objc(RetwistedSaverView)/@objc($PRINCIPAL_CLASS)/" "$SRC" > "$WORK/view.swift"
    grep -q "@objc($PRINCIPAL_CLASS)" "$WORK/view.swift" \
        || { echo "error: could not rename the principal class" >&2; exit 1; }
    SRC="$WORK/view.swift"
fi
xcrun swiftc \
    -module-name "$SWIFT_MODULE" \
    -swift-version 5 \
    -parse-as-library \
    -emit-object \
    -O \
    -target "$TARGET_TRIPLE" \
    -sdk "$SDK" \
    -import-objc-header "$ROOT/saver/include/retwisted_saver.h" \
    -o "$WORK/view.o" \
    "$SRC"

# 3. link a loadable BUNDLE (MH_BUNDLE), not a dylib: a .saver's executable
#    is dlopened by CFBundle. `-emit-library -Xlinker -bundle` is the one
#    incantation that gets there through swiftc while still picking up the
#    Swift runtime link flags.
rm -rf "$BUNDLE"; mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
xcrun swiftc \
    -emit-library -Xlinker -bundle \
    -target "$TARGET_TRIPLE" \
    -sdk "$SDK" \
    -o "$BUNDLE/Contents/MacOS/$NAME" \
    "$WORK/view.o" "$LIB" \
    -framework ScreenSaver -framework AppKit -framework AVFoundation \
    -framework CoreGraphics -framework QuartzCore

# 4. Info.plist. NSPrincipalClass is what System Settings instantiates; get
#    it wrong and the saver silently never appears.
cat > "$BUNDLE/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>$NAME</string>
	<key>CFBundleIdentifier</key>
	<string>$BUNDLE_ID</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$NAME</string>
	<key>CFBundleDisplayName</key>
	<string>$NAME</string>
	<key>CFBundlePackageType</key>
	<string>BNDL</string>
	<key>CFBundleShortVersionString</key>
	<string>$VERSION</string>
	<key>CFBundleVersion</key>
	<string>$VERSION</string>
	<key>LSMinimumSystemVersion</key>
	<string>$DEPLOY_TARGET</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSPrincipalClass</key>
	<string>$PRINCIPAL_CLASS</string>
	<key>RTWModuleSlug</key>
	<string>$SLUG</string>
</dict>
</plist>
PLIST

# 5. the asset packs — ALL of them (never committed; copied in at build
#    time). The catalogue is the one in app/src/lib.rs, which is also what
#    rtw_module_slug() hands the Module popup, so a module the sheet offers
#    and a pack the bundle carries cannot drift apart. A module with no
#    local pack is reported and skipped: picking it in the sheet would get
#    a NULL runtime and a black screen, and it is better to know now.
SHIPPED=0
if [ "$NO_PACKS" = "1" ]; then
    echo "    packs: none (--no-packs) — the Options panel rips the user's own download"
else
mkdir -p "$BUNDLE/Contents/Resources/assets"
SLUGS=$(sed -n '/MODULE_TITLES/,/^];/p' "$ROOT/app/src/lib.rs" \
        | sed -n 's/^ *("\([a-z0-9-]*\)", *".*$/\1/p')
[ -n "$SLUGS" ] || { echo "error: could not read MODULE_TITLES from app/src/lib.rs" >&2; exit 1; }
SHIPPED=0
MISSING=""
for s in $SLUGS; do
    if [ -f "$ROOT/assets/$s/meta.json" ]; then
        ditto "$ROOT/assets/$s" "$BUNDLE/Contents/Resources/assets/$s"
        SHIPPED=$((SHIPPED + 1))
    else
        MISSING="$MISSING $s"
    fi
done
echo "    packs: $SHIPPED shipped$([ -n "$MISSING" ] && echo ", MISSING:$MISSING")"

# assets/_shared — collection-wide pack content, not a module: the Options…
# panel's faceplate banner lives there (tools/twistedrip/pack.py's
# build_faceplate). Optional: a bundle built from an older pack tree just
# draws the banner's own fallback.
if [ -d "$ROOT/assets/_shared" ]; then
    ditto "$ROOT/assets/_shared" "$BUNDLE/Contents/Resources/assets/_shared"
    echo "    shared: $(ls "$ROOT/assets/_shared" | tr '\n' ' ')"
else
    echo "    shared: MISSING assets/_shared (no faceplate banner in the panel)"
fi
fi

# The shippable bundle must carry nothing Berkeley: no pack dir, and none of
# the file kinds a pack is made of anywhere under Resources.
if [ "$NO_PACKS" = "1" ]; then
    LEAK=$(find "$BUNDLE/Contents/Resources" \( -name assets -o -name _shared \
            -o -name meta.json -o -name pens500.json -o -name '*.wav' -o -name '*.png' \
            -o -name '*.mid' \) -print | head -5)
    if [ -n "$LEAK" ]; then
        echo "error: --no-packs bundle contains pack files:" >&2
        echo "$LEAK" >&2
        exit 1
    fi
    echo "    check: no pack files inside (ok)"
fi

# 6. ad-hoc signature. Unsigned code in a bundle that legacyScreenSaver
#    dlopens gets killed on Apple Silicon; ad-hoc ("-") is enough for a
#    local build.
codesign --force --sign - --identifier "$BUNDLE_ID" --timestamp=none "$BUNDLE" >/dev/null
codesign -dv "$BUNDLE" 2>&1 | sed 's/^/    /'
echo "==> built $BUNDLE"
du -sh "$BUNDLE" | sed 's/^/    /'

if [ "$INSTALL" = "1" ]; then
    DEST="$HOME/Library/Screen Savers"
    mkdir -p "$DEST"
    rm -rf "$DEST/$NAME.saver"
    ditto "$BUNDLE" "$DEST/$NAME.saver"
    echo "==> installed $DEST/$NAME.saver"
    # The legacy host caches the loaded bundle for the life of its process,
    # so a rebuilt saver keeps running the OLD code until these die. They
    # relaunch on demand.
    for p in legacyScreenSaver legacyScreenSaver-x86_64 ScreenSaverEngine WallpaperVideoExtension; do
        pkill -f "$p" 2>/dev/null && echo "    killed $p" || true
    done
    # System Settings caches the saver LIST (and the thumbnail) just as hard.
    pkill -f "System Settings" 2>/dev/null && echo "    killed System Settings (reopen it)" || true
    echo "==> System Settings -> Screen Saver: pick \"$NAME\" (Other / bottom of the list)"
    if [ "$NO_PACKS" = "1" ]; then
        echo "    then Options… -> Locate… and choose your Totally Twisted download"
    else
        echo "    then Options… -> Module to choose which of the $SHIPPED modules runs"
    fi
fi
