# TO-DO:
# Remove the `musl` target cross-compilation before publishing the plugin.
# Decky plugins are typically compiled targeting the standard SteamOS glibc environment 
# (`x86_64-unknown-linux-gnu`) via the decky-cli docker builder. The musl target is 
# currently used as a workaround for local WSL glibc version mismatches.
