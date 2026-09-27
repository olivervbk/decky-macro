.PHONY: all clean

all:
	cargo build --release --target x86_64-unknown-linux-musl
	mkdir -p ../bin
	cp target/x86_64-unknown-linux-musl/release/decky-macro-daemon ../bin/decky-macro-daemon

clean:
	cargo clean
	rm -f ../bin/decky-macro-daemon
