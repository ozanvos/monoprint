# MPL-2.0 grammar source

The BIRD2, HCL and Terraform grammar files in this directory are made available under **Mozilla Public License 2.0**, not the Monoprint code license. The full upstream licenses, including preserved copyright notices, are in `../grammars-NOTICE.txt`; exact source URLs and checksums are in `../grammar-policy.json`.

- `NAME.upstream.json`: original source at the grammar's pinned upstream commit.
- `NAME.json`: normalized/patched source from the pinned Shiki TextMate collection, used as the input to Giallo. Monoprint makes no further edits to these grammar definitions.

The collection's transformations and source configuration can be inspected at [commit 4cb1625](https://github.com/shikijs/textmate-grammars-themes/tree/4cb1625ad87a07fb4674fe97b921a8f756f4694b/scripts/grammars). Monoprint compiles this JSON into the embedded registry; its builder is `examples/compile_registry.rs`. Preserve this source directory and the notices with distributions containing those compiled grammars. Recipients may use and modify these source files under MPL-2.0.
