# Builds the Rust backend inside a SteamOS-based image so the resulting binary
# links against the Steam Deck's glibc (not your dev machine's libc).
#
# The Decky CLI's `decky plugin build -b` (used by the plugin-database store CI)
# builds this image when it is present, mounts:
#   plugin/backend -> /backend
#   <plugin>/bin   -> /backend/out
# and then runs the ENTRYPOINT below.
FROM ghcr.io/steamdeckhomebrew/holo-toolchain-rust:latest

# entrypoint.sh must always be located in the backend folder
ENTRYPOINT [ "/backend/entrypoint.sh" ]
