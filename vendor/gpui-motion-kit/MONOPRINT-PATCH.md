# Monoprint GPUI Kit backend patch

Source: `integrations/gpui-kit` of https://github.com/1bye/gpui-motion at
`fd9baf910ba0e88b06967bfb322d6be4b2d39723` (`gpui-motion-kit` 0.5.0). `src/`, the README and both licenses are
unchanged; benches, examples and native tests are omitted.

Upstream pins GPUI Kit 0.7.3 with its `gpui-pre` backend. `Cargo.toml` instead
depends on Kit 0.8.0 with `gpui-fast`, the version and backend the app uses, and
the published `gpui-ce-motion` 0.4.0 engine. The adapter compiles against it
without source changes.

Remove this patch when a published `gpui-motion-kit` supports Kit 0.8 on
`gpui-fast`.
