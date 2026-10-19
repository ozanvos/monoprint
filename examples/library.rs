//! Run with `cargo run --release --example library --no-default-features`.
use monoprint::{Appearance, Font, Language, Renderer, Scale, Snippet, Style};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::path::Path::new("output/library");
    fs::create_dir_all(output)?;
    let renderer = Renderer::new();
    let snippet = Snippet::new("fn main() {\n    println!(\"Hello, Monoprint!\");\n}\n")
        .language("rust".parse()?)
        .title("hello.rs")
        .highlight(2);
    let style = Style::default().padding(32).width(640);
    fs::write(
        output.join("hello.png"),
        renderer.png(&snippet, &style, Scale::X2)?,
    )?;
    fs::write(output.join("hello.svg"), renderer.svg(&snippet, &style)?)?;

    let light = style
        .clone()
        .theme("monoprint".parse()?)
        .appearance(Appearance::Light);
    for (name, source) in [("one.rs", "let answer = 42;"), ("two.py", "answer = 42")] {
        let snippet = Snippet::new(source)
            .title(name)
            .language(Language::for_file_name(name).unwrap_or_default());
        fs::write(
            output.join(format!("{name}.html")),
            renderer.html(&snippet, &light)?,
        )?;
    }

    let font = Font::from_bytes(&include_bytes!("../assets/fonts/space-mono.ttf")[..])?;
    let custom = style.clone().font(font);
    fs::write(
        output.join("custom.html"),
        renderer.html(&snippet, &custom)?,
    )?;

    let preview_renderer = renderer.clone();
    let preview = std::thread::spawn(move || preview_renderer.html(&snippet, &style));
    fs::write(
        output.join("preview.html"),
        preview.join().expect("preview worker panicked")?,
    )?;

    println!("Wrote examples to {}", output.display());
    Ok(())
}
