.PHONY: all
all: build

.PHONY: build
build:
	cargo build --release

.PHONY: install
install: target/release/hp-x360-elan-fix
	install -D -m 755 -o root -g root target/release/hp-x360-elan-fix /usr/local/bin/
	install -D -m 644 -o root -g root hp-x360-elan-fix.service /usr/lib/systemd/system/

.PHONY: uninstall
uninstall:
	rm /usr/local/bin/hp-x360-elan-fix
	rm /usr/lib/systemd/system/hp-x360-elan-fix.service

.PHONY: clean
clean:
	cargo clean
