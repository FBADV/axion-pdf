#!/usr/bin/env bash
# Axion macOS disk image: a universal (Apple silicon + Intel) app, ad-hoc signed, in a .dmg with
# an Applications shortcut. Needs the two release builds (see .github/workflows/axion-build.yml).
#   bash packaging/axion/macos.sh VERSION
set -euo pipefail
VERSION="$1"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
: "${APP_TITLE:?}" "${APP_SLUG:?}" "${APP_EXE:?}" "${APP_ID:?}" "${CARGO_BIN:?}"
WORK="$ROOT/target/axion-macos"
APP="$WORK/$APP_TITLE.app"
rm -rf "$WORK" && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$ROOT/out"

lipo -create -output "$APP/Contents/MacOS/$APP_EXE" \
  "target/aarch64-apple-darwin/release/$CARGO_BIN" "target/x86_64-apple-darwin/release/$CARGO_BIN"
cp "assets/app-icon/$CARGO_BIN.icns" "$APP/Contents/Resources/$APP_EXE.icns"

if [[ "${OCR_MODELS:-0}" = 1 ]]; then
  mkdir -p "$APP/Contents/Resources/models"
  cargo xtask models "$APP/Contents/Resources/models"
  rm -f "$APP/Contents/Resources/models"/*.part
fi

# The project's Info.plist, renamed: display name, executable, icon, identifier, version.
CRAFT_TITLE="$(/usr/libexec/PlistBuddy -c "Print :CFBundleName" packaging/macos/Info.plist.in)"
sed -e "s|<string>${CRAFT_TITLE}</string>|<string>${APP_EXE}</string>|g" \
    -e "s|<string>ai\.storyteller\.[a-z]*</string>|<string>${APP_ID}</string>|" \
    -e "s|@SHORT_VERSION@|${VERSION}|g" -e "s|@VERSION@|${VERSION}|g" -e "s|@BUILD_SHA@|${GITHUB_SHA:-local}|g" \
    -e "s|Copyright © the ${CRAFT_TITLE} authors\.|Copyright © the ${CRAFT_TITLE} authors and Bernardo Advogados.|" \
    packaging/macos/Info.plist.in >"$APP/Contents/Info.plist"
# What Finder and the Dock show.
plutil -replace CFBundleDisplayName -string "$APP_TITLE" "$APP/Contents/Info.plist"
plutil -replace CFBundleName -string "$APP_TITLE" "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
printf 'APPL????' >"$APP/Contents/PkgInfo"

codesign --force --deep --sign - --timestamp=none "$APP"
codesign --verify --deep --strict "$APP"

STAGE="$WORK/dmg"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/$APP_TITLE.app"
ln -s /Applications "$STAGE/Applications"
DMG="$ROOT/out/$APP_SLUG-$VERSION-Mac.dmg"
for attempt in 1 2 3 4 5; do
  if hdiutil create -volname "$APP_TITLE" -srcfolder "$STAGE" -ov -format UDZO -fs HFS+ "$DMG"; then break; fi
  echo "hdiutil busy, retrying ($attempt)"; sleep 5
done
ls -lh "$DMG"
