#!/bin/sh
# Builds the decky-macro Rust daemon and copies it to $OUT_DIR (default ../bin).
#
#   Local:  ./backend/build.sh                           # host libc
#           DECKY_MACRO_TARGET=static ./backend/build.sh # static musl (runs on
#                                                         # SteamOS from any host)
#   CI:     the backend Dockerfile's entrypoint sets OUT_DIR=./out and builds
#           natively inside the SteamOS-based container (see Dockerfile).
set -eu

cd "$(dirname "$0")"

BIN=decky-macro-daemon
OUT_DIR="${OUT_DIR:-../bin}"

case "${DECKY_MACRO_TARGET:-native}" in
    static|musl) TARGET=x86_64-unknown-linux-musl ;;
    native|"")   TARGET="" ;;
    *)           TARGET="$DECKY_MACRO_TARGET" ;;
esac

if [ -n "$TARGET" ]; then
    rustup target add "$TARGET" >/dev/null 2>&1 || true
    # A static musl build still needs a linker driver: use the host C compiler if
    # present, else Rust's bundled rust-lld (self-contained for musl).
    if [ "$TARGET" = "x86_64-unknown-linux-musl" ] \
        && ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
        export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C linker=rust-lld"
    fi
    cargo build --release --target "$TARGET"
    SRC="target/$TARGET/release/$BIN"
else
    cargo build --release
    SRC="target/release/$BIN"
fi

mkdir -p "$OUT_DIR"
cp "$SRC" "$OUT_DIR/$BIN"
echo "Built $OUT_DIR/$BIN"
