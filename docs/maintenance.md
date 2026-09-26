# Maintenance

Normal builds only need Cargo and a C compiler. All rendering assets are embedded; the importer and browser tools are development-only.

## Rebuild frame assets

Use Node.js 22.13+ and the locked npm dependencies. Check out the repository and
commit recorded in `assets/legacy-frames/provenance.json`, then import that source tree:

```sh
npm ci
node scripts/import-legacy-frames.mjs /path/to/frame-source
```

The importer reads original components, CSS, palettes, and assets. It generates inert templates and per-theme styles for the CLI, plus a separate development bundle of the original React components for reference captures. It never copies Söhne Mono. Fonts are separately pinned by byte checksum in `assets/fonts/manifest.json`; `scripts/fetch-fonts.py` is an explicit update operation that fetches current upstream font files and updates their provenance.

## Rebuild the Giallo registry

```sh
curl -L https://codeload.github.com/shikijs/textmate-grammars-themes/tar.gz/4cb1625ad87a07fb4674fe97b921a8f756f4694b -o /tmp/grammars.tar.gz
mkdir -p /tmp/monoprint-grammars
tar -xzf /tmp/grammars.tar.gz -C /tmp/monoprint-grammars --strip-components=1
node scripts/import-grammar-metadata.mjs /tmp/monoprint-grammars
cargo run --locked --release --example compile_registry -- /tmp/monoprint-grammars
```

Rebuild the registry after changing palettes. The builder validates the collection against `licenses/grammar-policy.json`, loads only its 250 included grammar/injection files, registers the 232 public grammars' aliases, adds plain text, and writes the registry **before linking**. It requires Node.js for the maintenance-only input check; ordinary Cargo builds and rendering do not. Giallo's hash-map serialization is not byte-deterministic. The builder records the resulting checksum and hashes of the policy, notices, catalog and palettes in `assets/grammar-provenance.json`. The checked-in registry is the build input for releases.

`assets/themes/monoprint-comments.json` is an internal classification palette.
When a separate comment font is requested, Giallo's dual-theme mode uses it in
the same tokenization pass as the visual palette. This preserves comment scopes
independently of theme colors, without replacing or forking the grammar engine.
Rebuild the registry after changing this palette too.

Run `node scripts/import-grammar-metadata.mjs --check` to verify the checked-in notice/catalog/provenance bundle offline, and `node --test scripts/grammar-policy.test.mjs` to test the import guards. CI runs both, plus a Rust test that checks every included and excluded grammar against the actual embedded registry. Do not refresh the receipt to bypass a stale registry: rebuild it with the command above.

Additional grammars outside the collection use a checked-in `assets/grammars/<name>.json` file and a policy entry with `localFile` and `metadata`. These inputs must retain a pinned source and license, exact input/metadata hashes, and reviewed license evidence. Hem is extracted without changing patterns from the `hemLanguage` object in its pinned documentation config. The collection index and file inventory checks remain limited to collection entries.

The importer does not approve new licenses or download evidence. Updating the collection requires reviewing changed grammar sources, file-specific license exceptions, pinned license texts, aliases and embedded dependencies; then updating the reviewed policy and evidence, importing metadata, and rebuilding. Unknown files, changed bytes/metadata, unpinned license records, missing source copies and unapproved licenses fail validation. See [grammar licensing](grammar-licenses.md).

## Validate appearance and speed

```sh
cargo build --locked --release --target-dir target
# import-legacy-frames.mjs must already have produced the reference bundle.
CHROME_PATH=/path/to/chrome node scripts/references.mjs
python3 scripts/compare.py             # Pillow required
node scripts/verify-exports.mjs        # Offline HTML and SVG verification
BENCH_CPU='Apple M1 Max' python3 scripts/benchmark.py
```

Reference captures run pinned upstream React frames with their browser effects and Shiki 1.3.0, using the bundled fonts. They are independent of the CLI's static templates and native effect adapters. Both browser screenshots and upstream-style `html-to-image` PNG exports are retained. The latter also applies its own font-size adjustment. Auto-fit is derived separately from browser font measurements. OpenAI's font substitution is explicit. References use pinned source rather than a live website.

`output/playwright/reference/manifest.json` records browser version, source preservation, controls, dimensions, and blocked network requests. `scripts/compare.py` produces dimension checks, per-pixel RGB differences, difference images, contact sheets, and `docs/visual-results.json`. `output/playwright/reference/review.html` provides full-resolution pairs with links to SVG/HTML. The comparator also refreshes format comparisons and alpha/composited-color diagnostics. Review actual images as well as scores; antialiasing and blur differences are not layout proof. `CASES=candy-dark,nuxt-dark` limits a reference run.

`scripts/smoke.py target/release/monoprint --all-modes` is a quick PNG smoke test. The Cargo integration suite checks all three formats across all 65 variants. A network-disabled Linux container can run the same suite; the CI workflow targets native Ubuntu x86-64 and macOS runners.

`scripts/notices.py` collects Cargo's locked license declarations and packaged notice files. It needs a populated Cargo cache, and is an attribution inventory rather than a license compatibility analyzer.

Monoprint directly depends on `vendor/takumi-svg` for SVG shadow compositing and duplicate background painting on anonymous layout containers; it calls that backend rather than Takumi's SVG re-export. Downstream path dependencies therefore keep these fixes without a root Cargo patch. Its upstream version, changed files, notices, and exact core-version pin are preserved there. Revalidate the SVG/PNG comparison before removing or upgrading this dependency. Registry publishing is disabled until an upstream release or published fork includes the fixes. See [the library guide](library.md).

Use `cargo fmt --package monoprint -- --check` to check project code while retaining the vendor's upstream formatting. Library checks include `cargo test --locked --release --no-default-features` and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-default-features --no-deps`.

Visual regression PNGs in `tests/visual/` come from the independent browser references. See their README before updating them. The native CSS adapter compensates for Takumi 2.14's inline wrapping, relative pseudo-element positioning, ignored `font: inherit` shorthand and raster shadow list order; recheck these adapters when upgrading Takumi. Nuxt's glow is blurred in an SVG filter before viewport clipping. Code and Firecrawl decoration sizes follow the pinned html-to-image export adjustment.

## Curated editor themes

`assets/editor/sources.json` pins the 31 selected input files, upstream revisions,
license texts, and their SHA-256 hashes. It also records the correction of Tokyo
Night Light's upstream `type: dark` metadata. It never imports wallpapers, logos,
or extension code. SynthWave's small token glow mapping is adapted in
`src/editor_theme.rs`; its pinned source is recorded with the theme inputs.
The local raster/SVG backends resolve per-token text shadows; their patch notes
and `tests/editor_themes.rs` cover SynthWave's visible glow and shadow overrides.

```sh
npm ci
node scripts/import-editor-themes.mjs
node scripts/import-editor-themes.mjs --check
cargo run --locked --release --example compile_registry -- /tmp/monoprint-grammars
cargo run --locked --release --example gui_thumbnails
npm --prefix bindings/node run build
```

The importer fetches only pinned, checksum-verified resources and caches them in
`/tmp/monoprint-editor-themes` (or a directory passed as its first argument).
The offline `--check` needs only Node's built-ins. Reimporting does not refresh
source pins; updating them is an explicit license/source review. Keep
`licenses/editor-themes-NOTICE.txt` and the provenance receipt with distributions.
The original companion gradients and CSS patterns are generated without bitmap
assets, network access, or a browser at rendering time.
