#!/usr/bin/env bash
# Axion Android app: the WebAssembly build (apps/$WEB_APP, built by trunk into dist/web) in the
# Android shell (android/), with the app icon at every density. Run after `trunk build --release`.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
: "${APP_SLUG:?}" "${CARGO_BIN:?}" "${AXION_VERSION_NAME:?}"

SITE=""
for d in dist/web "apps/$WEB_APP/dist"; do
  [[ -f "$d/index.html" ]] && SITE="$d" && break
done
[[ -n "$SITE" ]] || { echo "error: no web build (dist/web/index.html)" >&2; exit 1; }

A="$ROOT/android"
WEB="$A/app/src/main/assets/web"
rm -rf "$WEB" && mkdir -p "$WEB"
cp -R "$SITE/." "$WEB/"
# Phones and tablets: no pinch-zoom of the whole page (the app zooms documents itself).
sed -i 's|<meta name="viewport" content="[^"]*"|<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover"|' "$WEB/index.html"

# Launcher icons are pre-rendered in android/app/src/main/res/mipmap-*/ (from assets/app-icon).

# Documents the app opens from other apps (Files, e-mail, WhatsApp…).
FILTERS=""
IFS=',' read -ra TYPES <<<"${AXION_ACCEPT:-*/*}"
for action in VIEW SEND; do
  FILTERS+="            <intent-filter>\n                <action android:name=\"android.intent.action.$action\" />\n                <category android:name=\"android.intent.category.DEFAULT\" />\n"
  [[ "$action" = VIEW ]] && FILTERS+="                <category android:name=\"android.intent.category.BROWSABLE\" />\n                <data android:scheme=\"content\" />\n                <data android:scheme=\"file\" />\n"
  for t in "${TYPES[@]}"; do FILTERS+="                <data android:mimeType=\"$t\" />\n"; done
  FILTERS+="            </intent-filter>\n"
done
python3 - "$A/app/src/main/AndroidManifest.xml.in" "$A/app/src/main/AndroidManifest.xml" "$FILTERS" <<'EOF'
import sys
src, dst, filters = sys.argv[1], sys.argv[2], sys.argv[3].replace("\\n", "\n")
open(dst, "w").write(open(src).read().replace("<!--AXION_FILTERS-->", filters.rstrip("\n")))
EOF

if ! (cd "$A" && gradle --no-daemon --console=plain assembleRelease) >"$ROOT/gradle.log" 2>&1; then
  # The run log is not always reachable: surface the failure as annotations too.
  grep -E "error|FAILURE|What went wrong|wrong:|Exception|> " "$ROOT/gradle.log" | grep -v "^> Task" | head -40 | while IFS= read -r line; do echo "::error::$line"; done
  tail -60 "$ROOT/gradle.log"
  exit 1
fi
tail -5 "$ROOT/gradle.log"
mkdir -p "$ROOT/out"
cp "$A/app/build/outputs/apk/release/app-release.apk" "$ROOT/out/$APP_SLUG-$AXION_VERSION_NAME-Android.apk"
ls -lh "$ROOT/out"
