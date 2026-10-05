# App Store assets

Listing text, screenshots, icons and support/privacy pages for Monoprint.

## Files

- Screenshots and their generated preview/checksum files are generated locally;
  regenerate and review a current set before submission.
- `icon/`: Icon Composer source, compiled resources and PNG references.
  The uploaded app bundle supplies the App Store icon.
- `metadata/en-US/`: name, subtitle, description, keywords, promotional text
  and review instructions.
- `metadata/app-information.json` and `metadata/privacy-disclosure.json`:
  listing fields and privacy answers.
- `pages/`: support and privacy pages, also used by the website.
- `fixtures/`: sample drafts for the captures.

The capture fixtures use Monoprint, GitHub Default, Dusk and Ground.
Review every newly generated screenshot before uploading.

The support and privacy pages are hosted at
[monoprint.7ea.st/support/](https://monoprint.7ea.st/support/) and
[monoprint.7ea.st/privacy/](https://monoprint.7ea.st/privacy/).
Support email: contact@7ea.st. Keep private App Review contact details outside Git.

## Regenerate

Capture requires macOS Screen Recording access. Packaging requires Pillow and
an existing App Store bundle. The capture helper must provide window-list data.

```sh
python3 apps/monoprint-gui/scripts/bundle-macos.py --app-store
python3 scripts/capture_app_store_assets.py \
  --window-list-helper /path/to/screenshot/scripts/take_screenshot.py
python3 scripts/package_app_store_assets.py
```

Captures use a separate bundle identifier and temporary draft. The light shots
use a capture-only executable that selects the app's existing light palette:

```sh
python3 scripts/capture_app_store_assets.py \
  --window-list-helper /path/to/screenshot/scripts/take_screenshot.py \
  --executable target/gui-app-store/release/monoprint-assets-light \
  --interface light --fixture 02-rust-light
python3 scripts/capture_app_store_assets.py \
  --window-list-helper /path/to/screenshot/scripts/take_screenshot.py \
  --executable target/gui-app-store/release/monoprint-assets-light \
  --interface light --fixture 05-svg-transparent
python3 scripts/package_app_store_assets.py
```

The packager checks interface colors, converts captures to sRGB/RGB, flattens
rounded-corner alpha and makes the half-size copies. It does not retouch the UI.
It writes `output/app-store/Monoprint-App-Store-assets.zip`. Run it after editing
metadata or pages to update the preview, checksums and ZIP.

## Submission

See [release packaging](../../docs/release.md) for signing and installer commands.
Check Apple's [screenshot sizes](https://developer.apple.com/help/app-store-connect/reference/app-information/screenshot-specifications)
and [listing fields](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information)
before submitting. The asset pack includes en-US only; translations and an app
preview video are optional. Verify privacy answers against the final app.
