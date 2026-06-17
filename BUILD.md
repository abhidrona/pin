# Build and Release Guide

This document is intentionally separate from `README.md`.

`README.md` is for users. This file is for maintainers building, testing, installing, and releasing `pin`.

## Prerequisites

- Rust stable toolchain
- `cargo`
- `make`
- `tmux` for popup overlay testing
- Optional: `rustup` for cross-target Linux builds

## Build

```bash
make build
```

Equivalent:

```bash
cargo build
```


## Format, lint, and fix

Format Rust code before committing or releasing:

```bash
make fmt
```

Aliases are also available:

```bash
make format
make fix-format
```

Check formatting without changing files:

```bash
make fmt-check
```

Run Clippy with warnings denied, matching CI/release behavior:

```bash
make lint
```

Apply safe Clippy fixes automatically:

```bash
make lint-fix
```

Run both rustfmt and safe Clippy fixes:

```bash
make fix
```

Use this before opening a PR or cutting a release:

```bash
make fix
make release-check
```

`make fmt` only changes formatting and line wrapping. It will not fix Clippy issues such as `useless_format`, `&PathBuf` vs `&Path`, or `manual_clamp`. Use `make lint` to detect those and `make lint-fix` / `make fix` to apply safe automatic fixes.

Run the full local release gate:

```bash
make release-check
```

This runs formatting checks, Clippy, tests, and the optimized release build.

## Test

```bash
make test
```

Equivalent:

```bash
cargo test --all-targets --all-features
```

Run specific integration tests:

```bash
cargo test --test cli_add_test
cargo test --test session_workflow_test
cargo test --test session_negative_test
```

## Release build

```bash
make release
```

This runs:

```bash
cargo build --release
```

The binary will be available at:

```text
target/release/pin
```

## Local install

```bash
make install
```

Default install prefix:

```text
/usr/local
```

Override it:

```bash
make install PREFIX=$HOME/.local
```

This installs:

```text
$PREFIX/bin/pin
$PREFIX/share/man/man1/pin.1
```

Make sure `$HOME/.local/bin` is on your `PATH` if using a home prefix.

## Package current platform binary

```bash
make package VERSION=0.1.0
```

This creates:

```text
dist/pin-0.1.0-<host-triple>.tar.gz
dist/pin-0.1.0-<host-triple>.tar.gz.sha256
```

## Build Linux binary

Default Linux target:

```text
x86_64-unknown-linux-musl
```

Build:

```bash
make linux
```

Override target:

```bash
make linux LINUX_TARGET=aarch64-unknown-linux-musl
```

Output:

```text
dist/linux/bin/pin
```

## Tag a release

Commit all changes first, then:

```bash
make tag VERSION=0.1.0
```

This creates a signed git tag:

```text
v0.1.0
```

Push it:

```bash
make tag-push VERSION=0.1.0
```


## GitHub release workflow

The release workflow builds and publishes GitHub Release assets when a `v*` tag is pushed, or when the workflow is run manually with a version.

It currently builds:

```text
x86_64-unknown-linux-gnu
x86_64-apple-darwin
aarch64-apple-darwin
```

Both Darwin targets use `macos-latest`. Avoid pinning Intel macOS labels such as `macos-13` in the workflow because those runners can remain queued for a long time.

The workflow also sets job timeouts so real build/test hangs fail instead of running indefinitely.

## Homebrew tap release flow

This repo can be tapped directly from:

```text
https://github.com/abhidrona/pin
```

The tap formula lives at:

```text
Formula/pin.rb
```

A mirrored copy is kept at:

```text
packaging/homebrew/pin.rb
```

The formula uses a Git tag URL, so release updates are simple:

```ruby
url "https://github.com/abhidrona/pin.git", tag: "v0.1.0"
```

Release flow:

1. Create a tag using the GitHub workflow or local Make target:

```bash
make tag VERSION=0.1.0
make tag-push VERSION=0.1.0
```

2. Update both formula files to the new tag.

3. Test the formula locally:

```bash
brew uninstall pin 2>/dev/null || true
brew install --build-from-source ./Formula/pin.rb
pin --version
pin a "brew local smoke test"
pin ls
```

Users install with:

```bash
brew tap abhidrona/pin https://github.com/abhidrona/pin
brew install pin
```

Or:

```bash
brew install abhidrona/pin/pin
```

## Linux binary release flow

For each target, build and upload the binary tarball:

```bash
make linux LINUX_TARGET=x86_64-unknown-linux-musl
make package VERSION=0.1.0
```

Recommended release assets:

```text
pin-0.1.0-x86_64-unknown-linux-musl.tar.gz
pin-0.1.0-aarch64-unknown-linux-musl.tar.gz
pin-0.1.0-x86_64-apple-darwin.tar.gz
pin-0.1.0-aarch64-apple-darwin.tar.gz
```

## Verify man page locally

```bash
make man
```

Or:

```bash
man ./man/man1/pin.1
```


## Overlay backend debugging

`pin overlay` should not corrupt shell prompts. It uses tmux popup in tmux, a TUI only for supported interactive terminals, and plain output otherwise.

For terminals with custom redraw behavior, such as Warp, test with:

```bash
PIN_OVERLAY_BACKEND=plain pin overlay
PIN_OVERLAY_BACKEND=tui pin overlay
```

Use `pin ui` to test the raw TUI path directly.
