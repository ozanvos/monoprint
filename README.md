# Monoprint

Create code images locally. Monoprint has a macOS app, a CLI, and Rust and
TypeScript APIs. It exports PNG, SVG and self-contained HTML using
[Giallo](https://github.com/getzola/giallo) and [Takumi](https://github.com/kane50613/takumi).

Fonts, themes and syntax grammars are bundled. Rendering needs no browser,
network connection or installed fonts. Public downloads and registry packages
are not available yet.

## Desktop app

```sh
cargo run --locked --release --manifest-path apps/monoprint-gui/Cargo.toml
```

Edit code, choose a theme and export an image. The app has a live preview,
line and text highlights, custom fonts, clipboard export and a saved local draft.
See the [Studio guide](apps/monoprint-gui/README.md) for requirements and controls.

## CLI

Build with Rust 1.97 or newer and a C compiler:

```sh
cargo build --locked --release --target-dir target
./target/release/monoprint example.rs -o example.png
./target/release/monoprint example.rs --theme nuxt --title example.rs -o example.svg
printf 'const answer = 42;\n' | ./target/release/monoprint -l ts --theme candy -o example.html
```

Use `--list-themes`, `--list-languages` and `--list-fonts` to see the available
choices. Themes have light/dark variants and themed, solid, patterned or
transparent backgrounds. See the [CLI reference](docs/cli.md) for all options.

## Rust and TypeScript

```rust
use monoprint::{Renderer, Scale, Snippet, Style};

let png = Renderer::new().png(
    &Snippet::new("fn main() {}\n").language("rust".parse()?).title("main.rs"),
    &Style::default(),
    Scale::X2,
)?;
```

The [Rust guide](docs/library.md) covers path dependencies, reusable renderers,
custom fonts and raw-pixel previews. The [TypeScript guide](bindings/node/README.md)
covers Node.js 22+, async/sync rendering and native packages for macOS, Linux
and Windows. Rust registry publishing is disabled until the vendored backend
fixes are available as published dependencies.

## Development

```sh
cargo fmt --package monoprint -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --release
cargo test --manifest-path vendor/takumi-raster/Cargo.toml --locked --release --lib
```

See [maintenance](docs/maintenance.md) for asset updates and
[validation](docs/validation.md) for checks and limitations.

## License

Original code is Apache-2.0. Bundled assets and dependencies keep their own
licenses. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
[grammar licensing](docs/grammar-licenses.md).
