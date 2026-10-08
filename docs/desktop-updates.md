# Desktop installation and updates

Direct-download macOS builds use Sparkle 2.10.0. Windows installers and Linux
AppImages use Velopack 1.2.161. The CLI, library and Node packages do not include
either updater. App Store builds exclude Sparkle and use Store updates.

## Application behavior

On macOS, **Monoprint → Check for Updates…** opens Sparkle's native interface.
Sparkle owns its schedule, permissions and automatic-check preference; the menu
also lets the user toggle automatic checks. It is loaded from the signed app's
`Contents/Frameworks`, so running the development binary does not load an SDK
from elsewhere on the machine.

Windows and Linux check at launch and every six hours while running. Checks and
downloads run outside GPUI's foreground executor. A downloaded update displays
**Restart to update**. The action saves the current draft before starting the
Velopack helper and uses normal application quit. A failed save or helper launch
keeps the application open. Pending updates survive relaunch; Velopack's
automatic apply-on-startup is disabled so it cannot bypass this flow. The bottom
bar includes manual checks and a persisted automatic-check switch.

On Linux, opening the AppImage offers **Install for this user**. This copies it
to `~/Applications/Monoprint/Monoprint.AppImage`, registers an application menu
entry under `$XDG_DATA_HOME/applications`, and installs the icon. The operation
does not require root and does not overwrite existing files. After copying,
Monoprint saves any edits made during installation and launches the installed
copy. The original download remains available for the user to delete.

**Uninstall…** removes that installation and launcher. It separately offers to
keep or delete the saved draft and automatic-check setting. Deletion disables
queued preview and quit saves before removing the data. Edited launchers and
unrecognized installations require manual removal; arbitrary folders and
`MONOPRINT_SESSION` overrides are never recursively deleted.

Linux update downloads use a per-user cache at
`$XDG_CACHE_HOME/monoprint/packages`, defaulting to `~/.cache/monoprint/packages`.
The update cache and original downloaded AppImage are not removed by uninstall.
Flatpak and Snap defer to their package manager. Other packagers can set
`MONOPRINT_UPDATE_EXPLANATION` to an explanation shown by Check for Updates.
An AppImage in an unwritable location cannot self-update.

## Release configuration

`release/updates.json` pins tool versions and archive hashes. Feed URLs and the
Sparkle public key are initially unset. Development binaries and packages built
without configured updates make no update-feed requests. Configure the feeds
before preparing distributable update-enabled packages.

Use a separate HTTPS directory for each runtime: macOS architecture-specific
appcasts, `win-x64`, and `linux-x64`. Velopack's feed URL points to a directory
containing `releases.CHANNEL.json` and the package files it references; Sparkle's
feed URL points to `appcast.xml`. Never mix architectures in a single feed.

Sparkle requires both Developer ID signing/notarization and an Ed25519 update
key. Generate the key with the SDK's `bin/generate_keys`, commit only its public
key, and keep the private key in the signing environment or Keychain. Preserve
this key across releases. Increment both the package version and, where used,
`CFBundleVersion` on every release.

Velopack verifies package hashes from its feed. The hashes detect corruption; publisher trust depends on the HTTPS feed host. Windows distributable packages
also require Authenticode signing via Velopack's signing template. Linux binaries
need a compatible glibc, graphics driver and desktop session; AppImage packages
collect additional runtime libraries with pinned linuxdeploy. The supplied Linux
workflow builds on Ubuntu 24.04, establishing a glibc 2.39 baseline. FUSE 2 may
need to be installed to run an AppImage normally.

## Prepare macOS packages and feeds

```sh
python3 apps/monoprint-gui/scripts/prepare-sparkle.py \
  --output target/update-tools/sparkle
python3 apps/monoprint-gui/scripts/bundle-macos.py \
  --updates --sparkle-sdk target/update-tools/sparkle \
  --sign-identity 'Developer ID Application: YOUR IDENTITY' \
  --notary-profile YOUR_PROFILE --archive-dir output/macos-updates
python3 scripts/test-app-package.py --from output/macos-updates --require-notarized
python3 apps/monoprint-gui/scripts/generate-appcast.py \
  --from output/macos-updates --sparkle-sdk target/update-tools/sparkle \
  --output output/macos-feed --download-url https://YOUR_HOST/mac-arm64/
```

The last command uses Sparkle's Keychain signing key; `--ed-key-file` instead
accepts a private key file from the signing environment. Feed generation refuses
dirty, unsigned, unnotarized or Store artifacts and checks the shipping ZIP hash.
The appcast references the ZIP produced **after** stapling. Generated ZIP signatures are checked
with CryptoKit against the public key embedded in the app, catching a signing
environment configured with the wrong key. Keep old published
archives available when replacing a feed. This initial generator prepares a full
update without deltas; it does not edit an existing published feed.

The macOS archive workflow and release-candidate workflow have an `updates`
input. It requires `signed: true`. They can use repository variables
`MONOPRINT_MACOS_UPDATE_FEED` and `MONOPRINT_SPARKLE_PUBLIC_KEY`, or the committed
configuration. Feed signing and hosting remain separate preparation/publication
steps.

## Prepare Windows and Linux packages

Run these commands on the matching x64 operating system with .NET 8 installed:

```sh
dotnet tool install --global vpk --version 1.2.161
# Windows: configure a signing template backed by your signing environment.
python apps/monoprint-gui/scripts/bundle-desktop.py \
  --runtime win-x64 --output output/windows-updates \
  --sign-template 'YOUR_SIGNING_COMMAND {{file}}'
# Linux:
python3 apps/monoprint-gui/scripts/prepare-linuxdeploy.py \
  --output target/update-tools/linuxdeploy.AppImage
python3 apps/monoprint-gui/scripts/bundle-desktop.py \
  --runtime linux-x64 --output output/linux-updates \
  --linuxdeploy target/update-tools/linuxdeploy.AppImage
python3 scripts/test-desktop-package.py --from output/linux-updates
```

Feed URLs may be provided with `--update-feed`. The scripts use isolated build
directories and embed the feed and channel as build inputs. They stage only the
application and notices, package through the pinned vpk CLI, and produce hashes
and a source receipt after package verification. Windows verification checks the
installer and packaged executable's Authenticode signatures when signing is
requested. Linux uses a custom AppDir with bundled shared libraries
and an AppRun that preserves arguments. Updates can use full packages; deltas are optional.

`Windows and Linux desktop packages` is a manual GitHub workflow. Set
`MONOPRINT_WINDOWS_UPDATE_FEED` and `MONOPRINT_LINUX_UPDATE_FEED` repository
variables, or commit the feed configuration. It builds and tests both platforms
and uploads verification artifacts only: Windows output is deliberately unsigned
and marked non-release. Verify distribution signing before a public release. Local dirty or unsigned checks require `--local` and cannot
be represented as release artifacts.

## Publishing and testing

These scripts do not publish, create releases or change the existing
`release/config.json` publication gate. After validating the artifacts, upload
package files first and replace the feed last. Serve feed metadata with short
cache lifetimes. Keeping the old feed during package upload prevents references
to unavailable packages.

Before enabling a public channel, qualify two consecutive versions on each
native platform: fresh install, launch, background check/download, restart,
exact version change, latest draft restoration, disabled automatic checks,
offline retry and uninstall. Also verify Developer ID/notarization, Windows
Authenticode and representative Linux Wayland/X11 systems. Run these checks on the installed packages.
