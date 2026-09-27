# Bundled grammar licensing

Monoprint embeds a reviewed subset of the pinned Shiki TextMate collection: **231 named grammars and 18 helper/injection grammars**. The public catalog has 229 highlighting selections plus plain text; TypeScript selects TSX and TeX selects LaTeX. Grammar curation does not alter theme assets or add attribution to generated images or HTML.

`licenses/grammar-policy.json` records a decision for all 260 original inputs, including injections. Included entries have input and metadata SHA-256 hashes, an explicit license, a pinned original source, and a pinned license source. `licenses/grammar-license-evidence.json` preserves the reviewed license text or declaration, and the importer generates `licenses/grammars-NOTICE.txt` from it. These records describe this pinned bundle; they are not a legal opinion about every possible use of the project.

## Corrections

- Resolved all 12 upstream `NOASSERTION` labels using source license texts. In particular, NSIS has a grammar-specific **Apache-2.0** license overriding the repository's MIT default; LLVM is **Apache-2.0 WITH LLVM-exception**; Apache configuration is **BSD-2-Clause-Views** and Apex is **BSD-3-Clause**.
- Recovered missing licenses for ABAP, AppleScript, ERB, Logo, MIPS, PO, SSH configuration and TOML from the permissive bundle grant in each pinned README. The grant is preserved as `LicenseRef-TextMate-Bundle`, rather than relabeled MIT.
- Recovered YAML's file-specific MIT license, MATLAB's BSD-2-Clause license, Kusto/SPARQL/Turtle's Apache-2.0 licenses, and the collection MIT license for the generated `ts-tags` grammar.
- Hurl's pinned package manifest and Rel's pinned README explicitly declare MIT. Their declarations are preserved together with the full MIT terms; no missing copyright attribution was invented.
- Replaced GitHub HTML pages masquerading as license text for AutoHotkey and Smithy with pinned raw notices. AutoHotkey v2's actual license is LGPL-3.0, so it is excluded.
- For Fennel, Jsonnet, Registry, RISC-V, Shell Session and Wenyan, license files were added after the grammar's recorded commit. Each grammar was verified byte-identical at the later pinned license revision; both source locations and the comparison hash are recorded in its review note.

All retained license URLs now identify immutable commit revisions. Other known license labels were checked against the retrieved pinned license text, rather than copying the collection's moving `main`/`master` license URLs.

## Exclusions

| Grammar | Reason for excluding this source |
| --- | --- |
| Ada, Gnuplot, Nginx, Org, Racket | GPL-3.0 source; excluded by the bundle's distribution policy |
| AutoHotkey v2 | LGPL-3.0 source; excluded by the same policy |
| APL, DAX, Dream Maker, GLSL, Tcl | No explicit redistribution grant found for the pinned source |

These decisions remove the bytes from the **compiled registry**, not just the catalog. Explicit selection of an excluded language returns an unknown-language error. A filename that no longer identifies a supported language uses the existing plain-text fallback. A future replacement needs its own pinned source/license review and highlighting validation.

GLSL embeddings in C++, its macro helper, Elm, Nim and the tagged-template helper no longer load the excluded GLSL grammar. LaTeX's optional Gnuplot embedding likewise remains unresolved. The surrounding grammars still work; embedded contents receive only the surrounding grammar's tokenization. These deliberate missing references are listed in the policy and checked during rebuilds. No excluded grammar is copied into a retained input.

## MPL source and distribution

BIRD2, HCL and Terraform retain MPL-2.0. Both their original grammar JSON and the normalized JSON fed to Giallo are checked in under `licenses/grammar-sources/`. Preserve this directory, its README, the full license notices and the policy with software distributions. Recipients are informed that these covered source files are available under MPL-2.0. Mozilla's [MPL FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) explains source availability for compiled larger works; retaining these grammars does not make their license Apache-2.0.

Giallo's EUPL license and all other dependency/font/theme obligations are separate and remain documented in `THIRD_PARTY_NOTICES.md`. This change does not publish a release or resolve the separate vendored SVG backend publishing constraint.

## Maintenance safeguards

The importer rejects unknown files, changed grammar bytes or metadata, missing notices, HTML substituted for license text, unapproved license labels and unpinned source references. The registry builder invokes that validation and loads only the explicit included entries. Normal builds use the checked-in registry without Node.js or network access.

CI verifies the notice/catalog/provenance checksums and tests the policy guards. A Rust test loads the actual embedded registry and checks the inclusion/exclusion of every reviewed input. Updating palettes or grammar inputs requires rebuilding the registry; see [maintenance](maintenance.md). These checks detect drift from the reviewed inputs, not undiscovered licensing mistakes inside an upstream project.

## Validation

The [validation record](grammar-license-validation.json) pins the resulting registry hash. Local release tests (34), library-only tests (16), license-policy tests (9), Clippy, rustfmt and rustdoc passed. All 246 exports from the existing 82-case PNG/SVG/HTML fixture matrix were byte-identical to the pre-curation executable at `54662c2`. This checks retained fixture behavior, not the removed languages or all token sequences; remote CI was not run during this change.
