# Third-party notices

Original code in this project is licensed under Apache-2.0; see `LICENSE`. This does not relicense the following third-party material.

| Material | License / preserved information |
| --- | --- |
| legacy frames theme definitions, CSS, frame templates, images | MIT; `licenses/legacy-frames-MIT.txt`, pinned source and modifications in `licenses/legacy-frames-provenance.json` |
| Bundled font files | SIL OFL-1.1; individual `assets/fonts/*-OFL.txt` files and font source/checksum manifest |
| Giallo 0.5.2 | EUPL-1.2; `licenses/giallo-EUPL-1.2.txt`, unchanged dependency source available from the exact crate URL in the dependency inventory |
| TextMate grammar collection | Reviewed license records in `licenses/grammar-policy.json`; full texts in `licenses/grammars-NOTICE.txt`; pinned sources and input checksums in the policy and `assets/grammar-provenance.json` |
| Takumi and other Rust dependencies | Exact versions, declared licenses and source archive URLs in `licenses/rust-dependencies.json`; packaged notices in `licenses/rust-dependency-notices.txt`; missing root notices fetched at the recorded Git revisions in `licenses/upstream/` |

The embedded bundle contains 231 named grammars and 18 helper/injection grammars under MIT, Apache-2.0 (including the LLVM exception), BSD, ISC, MPL-2.0, and the permissive TextMate bundle grant. The latter is recorded as `LicenseRef-TextMate-Bundle`, with its full original text. GPL/LGPL and unresolved grammars are excluded from the registry and public catalog. This is a curated grammar bundle, not an Apache-only claim about all bundled material. See [the grammar review](docs/grammar-licenses.md) for exclusions, corrected classifications, and maintenance checks.

BIRD2, HCL and Terraform grammar source files remain under MPL-2.0. Their original and normalized source forms are supplied in `licenses/grammar-sources/`; recipients may use and modify them under MPL-2.0. Include that directory and these notices with binary/library distributions. These source obligations apply to the covered grammar files.

The embedded Giallo registry is generated data built from the pinned grammar and theme inputs by `examples/compile_registry.rs`. All original source links and rebuild instructions are included. Giallo is unmodified. The vendored `takumi-svg` 0.3.22 has documented outer-shadow and anonymous-container background corrections; see `vendor/takumi-svg/PATCHES.md` and its preserved MIT/Apache licenses. Other Cargo dependencies are unmodified. OpenAI's proprietary Söhne Mono is not included.

Keep these notices, the font licenses, and access to the exact corresponding dependency/grammar sources with release artifacts. No public binary release has been made from this workspace.

Monoprint keeps legacy frames attribution in the project, source provenance and distribution notices. It does not add a legacy frames watermark, caption, or attribution notice to generated PNG, SVG or HTML exports.
