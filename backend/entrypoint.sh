#!/bin/sh
# Runs inside the backend build container (see backend/Dockerfile).
#
# The Decky CLI mounts:
#   plugin/backend -> /backend
#   <plugin>/bin   -> /backend/out
# so build.sh must write the compiled daemon into ./out (done via OUT_DIR).
set -e

echo "Container's IP address: $(awk 'END{print $1}' /etc/hosts)"

cd /backend
OUT_DIR=./out ./build.sh

# Drop the (root-owned) cargo cache so it isn't left behind in the plugin dir.
cargo clean
