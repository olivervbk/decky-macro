#!/bin/sh
# Assemble a deployable plugin folder and a zip from already-built artifacts:
#   target/decky-macro/                  (copy to ~/homebrew/plugins/ on the Deck)
#   target/decky-macro-v<version>.zip    (install via Decky's "Install from ZIP")
#
# Expects `pnpm run build` (-> dist/) and `pnpm run build:backend[:static]`
# (-> bin/) to have run first. `pnpm run dist` does all of it.
set -eu

cd "$(dirname "$0")/.."

NAME="decky-macro"
VERSION="$(node -p "require('./package.json').version")"
OUT="target/$NAME"
ZIP="target/$NAME-v$VERSION.zip"

for required in dist/index.js bin/decky-macro-daemon; do
  if [ ! -f "$required" ]; then
    echo "error: $required is missing; run the frontend and backend builds first" >&2
    exit 1
  fi
done

rm -rf "$OUT" "$ZIP"
mkdir -p "$OUT"
cp -r bin dist assets main.py plugin.json package.json LICENSE README.md CHANGELOG.md "$OUT/"
echo "Packaged $OUT"

if command -v zip >/dev/null 2>&1; then
  (cd target && zip -qr "$NAME-v$VERSION.zip" "$NAME")
  echo "Created $ZIP"
else
  echo "warning: 'zip' not found, skipping $ZIP" >&2
fi
