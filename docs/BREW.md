# Homebrew Tap

The installed command is `pin`.

The Homebrew formula is named `pin`, and it installs the `pin` binary.

This repo can also act as the tap repo:

```text
https://github.com/abhidrona/pin
```

## Tap layout

Homebrew expects formulas under `Formula/` in the tap repository:

```text
pin/
└── Formula/
    └── pin.rb
```

The formula template is also mirrored at:

```text
packaging/homebrew/pin.rb
```

## Install from this repo as a tap

Because the repo is named `pin` rather than `homebrew-pin`, tap it with the explicit URL:

```bash
brew tap abhidrona/pin https://github.com/abhidrona/pin
brew install pin
```

Or install directly from the tapped formula:

```bash
brew install abhidrona/pin/pin
```

The command installed on PATH is:

```bash
pin
```

## Release update checklist

The formula uses the Git tag directly, so you only need to update the tag in the formula files.

1. Tag and push a release manually:

```bash
make tag VERSION=0.1.0
make tag-push VERSION=0.1.0
```

Or use the GitHub workflow **Tag, build, test, and release** with the `version` input.

2. Update both formula copies if the version changed:

```text
Formula/pin.rb
packaging/homebrew/pin.rb
```

Set:

```ruby
url "https://github.com/abhidrona/pin.git", tag: "v0.1.0"
```

3. Test locally:

```bash
brew uninstall pin 2>/dev/null || true
brew install --build-from-source ./Formula/pin.rb
pin --version
pin a "brew local smoke test"
pin ls
```

4. Commit the updated formula and push.
