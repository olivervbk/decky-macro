.PHONY: all clean

# Delegates to build.sh so target selection (DECKY_MACRO_TARGET) and the
# linker fallback live in a single place. Examples:
#   make
#   make DECKY_MACRO_TARGET=static
all:
	DECKY_MACRO_TARGET="$(DECKY_MACRO_TARGET)" sh build.sh

clean:
	cargo clean
	rm -f ../bin/decky-macro-daemon
