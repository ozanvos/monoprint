# Asset maintenance

Normal builds use the checked-in assets and need no importer or browser.

## Original styles

The catalog contains 48 Monoprint Originals and 21 curated editor presets, with
80 appearances. Original palettes and frames live in `assets/originals`,
`assets/selected`, and `assets/code-styles`; their native renderers live in `src/`.
Edit the JSON and CSS directly. Keep matching palettes under `assets/themes`
in sync, rebuild the registry after palette changes, then update the native tables:

```sh
cargo run --locked --release --example compile_pierre_palettes
cargo run --locked --release --example gui_thumbnails
npm run build --prefix bindings/node
```

`assets/retired-themes.json` supports saved-draft migration. Removed IDs are
rejected by the CLI and APIs.

## Syntax registry

The grammar collection, source hashes, and permitted licenses are pinned in
`licenses/grammar-policy.json`. See [grammar licensing](grammar-licenses.md)
before changing those inputs.

```sh
curl -L https://codeload.github.com/shikijs/textmate-grammars-themes/tar.gz/4cb1625ad87a07fb4674fe97b921a8f756f4694b -o /tmp/grammars.tar.gz
mkdir -p /tmp/monoprint-grammars
tar -xzf /tmp/grammars.tar.gz -C /tmp/monoprint-grammars --strip-components=1
node scripts/import-grammar-metadata.mjs /tmp/monoprint-grammars
cargo run --locked --release --example compile_registry -- /tmp/monoprint-grammars
cargo run --locked --release --example compile_pierre_palettes
```

The builder checks the reviewed inputs and records the generated registry hash
in `assets/grammar-provenance.json`. Rebuild stale output rather than refreshing
its receipt alone. Keep `assets/themes/monoprint-comments.json`, which preserves
Giallo comment classification for separate comment fonts.

Additional local grammars require pinned source and license records, input
hashes, and any required corresponding source files.

## Editor themes and fonts

Editor palettes are pinned in `assets/editor/sources.json`. To regenerate them:

```sh
npm ci
npm run import:editor
node scripts/import-editor-themes.mjs --check
```

Rebuild the syntax registry, native tables, and thumbnails after palette changes.
Keep the upstream notices and exact source records under `licenses/`.

Fonts are pinned in `assets/fonts/manifest.json`. Use `scripts/fetch-fonts.py`
for explicit updates. Fallback fonts have separate checksums and notices;
see [font fallback](font-fallback.md).

## Verify

```sh
node --test scripts/grammar-policy.test.mjs
node scripts/import-grammar-metadata.mjs --check
node scripts/import-editor-themes.mjs --check
cargo run --locked --release --example compile_pierre_palettes -- --check
cargo test --locked --release
cargo test --locked --release --no-default-features
```

Review rendered samples when changing layouts or palettes. The selected-style
and code-style tests can write samples through `MONOPRINT_SELECTED_SAMPLES` and
`MONOPRINT_CODE_SAMPLES`. Test the desktop app for interaction changes.

Vendored backend fixes and their rationale are recorded in each vendor's patch
notes. Keep their licenses and corresponding sources with distributions.
