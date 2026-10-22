# Release packaging

Packaging scripts produce archives, checksums and build records. They do not
upload or publish. Release builds require clean source; `--local` allows working
changes and marks the result non-release. Use a new output directory each time.

## CLI

Requires Rust, a C compiler, Node.js 22+ and Python 3:

```sh
node scripts/package-cli.mjs --out output/cli-release
node scripts/test-cli-package.mjs --from output/cli-release
```

The archive includes the executable, notices, corresponding dependency sources
and a build record. The `CLI release archives` workflow targets macOS arm64 and
Linux x64. Add `--local` for a local verification archive.

## macOS app

Requires Python 3.11+ and Xcode 26+. The default bundle is ad-hoc signed:

```sh
python3 apps/monoprint-gui/scripts/bundle-macos.py
open target/gui/Monoprint.app
```

For direct distribution, install a Developer ID Application certificate and
configure a `notarytool` keychain profile:

```sh
python3 apps/monoprint-gui/scripts/bundle-macos.py \
  --sign-identity 'Developer ID Application: Your Name (TEAMID)' \
  --notary-profile monoprint-release --archive-dir output/app-signed
python3 scripts/test-app-package.py --from output/app-signed --require-notarized
```

The script signs inner executables before the app, enables the hardened runtime,
submits to Apple, requires an Accepted result, staples the ticket and writes the
final ZIP. Keep signing credentials outside Git.

See [desktop updates](desktop-updates.md) for Sparkle, Windows installers,
Linux AppImages and update feeds.

## Mac App Store

`--app-store` excludes the audited private macOS APIs, enables App Sandbox and
omits the CLI and shell-based Shortcuts. The packager scans the executable and
fails if an audited private API name remains. Patch details are in
[the macOS patch notes](../vendor/gpui-pre-macos/MONOPRINT-PATCH.md).

```sh
python3 apps/monoprint-gui/scripts/bundle-macos.py --app-store
python3 apps/monoprint-gui/scripts/bundle-macos.py --app-store \
  --archive-dir output/app-store-local --local
# Executable only; this does not sign a bundle or enable the OS sandbox:
cargo build --release --locked --manifest-path apps/monoprint-gui/Cargo.toml --features app-store
```

Store bundles use user-selected read/write access and read-only security-scoped
bookmarks for custom fonts. Drafts stay in the app container. The normal app's
draft is not migrated. Help links to the support and privacy pages.

For an uploadable installer, use application and installer distribution
identities from the same team, plus a Mac App Store profile for
`app.monoprint.studio`:

```sh
python3 apps/monoprint-gui/scripts/bundle-macos.py --app-store \
  --sign-identity 'Apple Distribution: Your Name (TEAMID)' \
  --installer-identity '3rd Party Mac Developer Installer: Your Name (TEAMID)' \
  --app-store-profile /private/path/Monoprint.provisionprofile \
  --build-number 1 --archive-dir output/app-store-distribution
```

The packager checks profile expiry, app/team identifiers, certificates and
private keys. It embeds the profile, signs the app, creates a `.pkg` with
`productbuild` and verifies both signatures. Ad-hoc builds produce unsigned
verification installers and cannot be uploaded.

See [App Store assets](../release/app-store/README.md) for screenshots and listing
files. Keep private App Review contact details and signing files outside Git.

## npm and Rust crates

See [Node package builds](node-packaging.md) for the five-target build and
six-package publication order.

Rust registry publishing is disabled because the raster/SVG fixes use path
dependencies. Publish compatible backend packages or wait for upstream fixes
before enabling it. Adding an upstream registry version would discard the path
and use unpatched code. Also check the final crate size with bundled fonts.
See [Cargo packaging](https://doc.rust-lang.org/cargo/commands/cargo-package.html).

## Before distribution

Test the exact signed download on a clean machine: installation, launch, file
access, saved drafts, custom fonts, clipboard, exports and drag-out. For direct
macOS downloads, verify Gatekeeper and notarization. For Store builds, verify
through Apple's installed distribution path. Record the commit, archive checksum,
OS and machine. Unit tests and headless captures do not cover these checks.

Preserve project, font, grammar and dependency licenses and corresponding sources
in every package.

## Workflows and publishing

`ci.yml` checks the source. The release workflows build packages and receipts;
`release.yml` assembles CLI, Node, and desktop artifacts at one commit. These
workflows are dispatched explicitly. A push or tag does not publish packages.

```sh
node scripts/release.mjs check
node --test scripts/release.test.mjs scripts/package-receipts.test.mjs
```

Public distribution requires `publicPublishing: true` in `release/config.json`
and explicit approval for the invocation. Keep credentials outside Git. Apple
signing uses the `apple-signing` environment and `scripts/with-apple-signing.py`;
the workflow declares the required certificate and App Store Connect secrets.
See [Node packaging](node-packaging.md) for npm publication requirements and
[website commands](../website/README.md) for the separate website deployment.
