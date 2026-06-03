# pin - tiny directory-aware task tracker
#
# Common targets:
#   make build
#   make test
#   make release
#   make install
#   make package VERSION=0.1.0
#   make tag VERSION=0.1.0

SHELL := /usr/bin/env bash
BIN := pin
CRATE := pin-hud
VERSION ?= $(shell cargo metadata --no-deps --format-version 1 2>/dev/null | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -1)
PREFIX ?= /usr/local
TARGET_DIR := target
RELEASE_DIR := dist
HOST_TRIPLE ?= $(shell rustc -vV 2>/dev/null | sed -n 's/^host: //p')
LINUX_TARGET ?= x86_64-unknown-linux-musl

.PHONY: help fmt lint check build test release install uninstall clean package package-local linux tag tag-push brew-formula man

help:
	@echo "pin build targets"
	@echo ""
	@echo "  make fmt             Format Rust code"
	@echo "  make lint            Run clippy with warnings denied"
	@echo "  make check           Cargo check"
	@echo "  make build           Debug build"
	@echo "  make test            Run all tests"
	@echo "  make release         Optimized release build"
	@echo "  make install         Install binary + man page to PREFIX=$(PREFIX)"
	@echo "  make uninstall       Remove installed binary + man page"
	@echo "  make package         Build local release tarball in dist/"
	@echo "  make linux           Build Linux static-ish binary for $(LINUX_TARGET)"
	@echo "  make tag VERSION=x.y.z      Create signed release tag vX.Y.Z"
	@echo "  make tag-push VERSION=x.y.z Push release tag vX.Y.Z"
	@echo "  make clean           Remove build artifacts"

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -- -D warnings

check:
	cargo check --all-targets --all-features

build:
	cargo build

test:
	cargo test --all-targets --all-features

release:
	cargo build --release

install: release
	install -d "$(PREFIX)/bin"
	install -m 0755 "$(TARGET_DIR)/release/$(BIN)" "$(PREFIX)/bin/$(BIN)"
	install -d "$(PREFIX)/share/man/man1"
	install -m 0644 "man/man1/pin.1" "$(PREFIX)/share/man/man1/pin.1"
	@echo "Installed $(BIN) to $(PREFIX)/bin/$(BIN)"

uninstall:
	rm -f "$(PREFIX)/bin/$(BIN)"
	rm -f "$(PREFIX)/share/man/man1/pin.1"
	@echo "Uninstalled $(BIN) from $(PREFIX)"

clean:
	cargo clean
	rm -rf "$(RELEASE_DIR)"

package: release
	@if [[ -z "$(VERSION)" ]]; then echo "VERSION is empty. Use make package VERSION=0.1.0"; exit 1; fi
	mkdir -p "$(RELEASE_DIR)"
	rm -rf "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)"
	mkdir -p "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)/bin" "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)/man/man1" "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)/docs"
	cp "$(TARGET_DIR)/release/$(BIN)" "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)/bin/"
	cp man/man1/pin.1 "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)/man/man1/"
	cp README.md BUILD.md LICENSE SPEC.md "$(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE)/docs/"
	cd "$(RELEASE_DIR)" && tar -czf "$(BIN)-$(VERSION)-$(HOST_TRIPLE).tar.gz" "$(BIN)-$(VERSION)-$(HOST_TRIPLE)"
	cd "$(RELEASE_DIR)" && shasum -a 256 "$(BIN)-$(VERSION)-$(HOST_TRIPLE).tar.gz" > "$(BIN)-$(VERSION)-$(HOST_TRIPLE).tar.gz.sha256"
	@echo "Created $(RELEASE_DIR)/$(BIN)-$(VERSION)-$(HOST_TRIPLE).tar.gz"

linux:
	rustup target add "$(LINUX_TARGET)"
	cargo build --release --target "$(LINUX_TARGET)"
	mkdir -p "$(RELEASE_DIR)/linux/bin"
	cp "$(TARGET_DIR)/$(LINUX_TARGET)/release/$(BIN)" "$(RELEASE_DIR)/linux/bin/$(BIN)"
	@echo "Linux binary: $(RELEASE_DIR)/linux/bin/$(BIN)"

tag:
	@if [[ -z "$(VERSION)" ]]; then echo "Use make tag VERSION=0.1.0"; exit 1; fi
	git diff --quiet || (echo "Working tree is dirty. Commit changes before tagging." && exit 1)
	git tag -s "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "Created tag v$(VERSION)"

tag-push:
	@if [[ -z "$(VERSION)" ]]; then echo "Use make tag-push VERSION=0.1.0"; exit 1; fi
	git push origin "v$(VERSION)"

brew-formula:
	@echo "Tap formula: Formula/pin.rb"
	@echo "Mirror copy: packaging/homebrew/pin.rb"
	@echo "Tap with:"
	@echo "  brew tap abhidrona/pin https://github.com/abhidrona/pin"
	@echo "Install with:"
	@echo "  brew install pin"
	@echo "Before release, update the formula tag in Formula/pin.rb and packaging/homebrew/pin.rb."

man:
	man ./man/man1/pin.1
