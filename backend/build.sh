#!/bin/sh
# Builds the decky-macro Rust daemon and copies it to ../bin/decky-macro-daemon.
#
# Target selection (environment variable DECKY_MACRO_TARGET):
#   (unset) | native   -> build against the host libc (default)
#   static  | musl     -> build a statically-linked musl binary (recommended for
#                         SteamOS; avoids glibc-version mismatches between your
#                         dev machine, e.g. WSL Ubuntu, and the Steam Deck)
#   <triple>           -> any explicit Rust target triple, e.g.
#                         x86_64-unknown-linux-musl
#
# Usage:
#   ./backend/build.sh
#   DECKY_MACRO_TARGET=static ./backend/build.sh
#
# This script is written for POSIX sh so it also runs on the Alpine-based
# Decky plugin-database builder (busybox ash), which has no bash.
set -eu

SCRIPT_DIR="$( cd "$( dirname "$0" )" && pwd )"
cd "$SCRIPT_DIR"

BIN_NAME="decky-macro-daemon"
OUT_DIR="../bin"

TARGET_REQUEST="${DECKY_MACRO_TARGET:-native}"

# Resolve the requested value into a cargo target triple ("" = host/native).
case "$TARGET_REQUEST" in
    ""|native|host|gnu)
        CARGO_TARGET=""
        ;;
    static|musl)
        CARGO_TARGET="x86_64-unknown-linux-musl"
        ;;
    *)
        # Treat as an explicit triple.
        CARGO_TARGET="$TARGET_REQUEST"
        ;;
esac

echo "[build-backend] DECKY_MACRO_TARGET='${TARGET_REQUEST}' -> cargo target='${CARGO_TARGET:-<host>}'"

# Make sure the requested target is installed (best effort, rustup only).
if [ -n "$CARGO_TARGET" ] && command -v rustup >/dev/null 2>&1; then
    if ! rustup target list --installed 2>/dev/null | grep -qx "$CARGO_TARGET"; then
        echo "[build-backend] Installing Rust target $CARGO_TARGET..."
        rustup target add "$CARGO_TARGET"
    fi
fi

# A statically-linked musl build still needs a linker *driver*. Prefer a C
# compiler (cc/gcc) when present; otherwise fall back to Rust's bundled
# rust-lld, which is self-contained for the musl target.
EXTRA_RUSTFLAGS=""
case "$CARGO_TARGET" in
    *musl*)
        if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
            if command -v rust-lld >/dev/null 2>&1 || rustc --print sysroot >/dev/null 2>&1; then
                echo "[build-backend] No C compiler found; linking with rust-lld."
                EXTRA_RUSTFLAGS="-C linker=rust-lld"
            else
                echo "[build-backend] ERROR: no C compiler and no rust-lld available to link." >&2
                echo "[build-backend]        Install a C toolchain (e.g. build-essential / musl-tools)." >&2
                exit 1
            fi
        fi
        ;;
esac

if [ -n "$EXTRA_RUSTFLAGS" ]; then
    if [ -n "${RUSTFLAGS:-}" ]; then
        RUSTFLAGS="$RUSTFLAGS $EXTRA_RUSTFLAGS"
    else
        RUSTFLAGS="$EXTRA_RUSTFLAGS"
    fi
    export RUSTFLAGS
fi

if [ -n "$CARGO_TARGET" ]; then
    echo "[build-backend] Running: cargo build --release --target $CARGO_TARGET"
    cargo build --release --target "$CARGO_TARGET"
else
    echo "[build-backend] Running: cargo build --release"
    cargo build --release
fi

ARTIFACT="target/${CARGO_TARGET:+$CARGO_TARGET/}release/$BIN_NAME"
if [ ! -f "$ARTIFACT" ]; then
    echo "[build-backend] ERROR: expected artifact not found: $ARTIFACT" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
cp "$ARTIFACT" "$OUT_DIR/$BIN_NAME"
echo "[build-backend] Copied $ARTIFACT -> $OUT_DIR/$BIN_NAME"

if command -v file >/dev/null 2>&1; then
    file "$OUT_DIR/$BIN_NAME"
fi
