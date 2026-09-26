use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "code-images-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn run(&self, args: &[&str], input: &str) -> Output {
        let mut process = Command::new(env!("CARGO_BIN_EXE_code-images"))
            .args(args)
            .current_dir(&self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        process.wait_with_output().unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn success(out: Output) -> Vec<u8> {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
fn failure(out: Output, message: &str) {
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(message),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn output_contract_and_overwrite_protection() {
    let w = Workspace::new();
    failure(w.run(&[], "hi"), "output is required");
    failure(w.run(&["-o", "-"], "hi"), "stdout requires --format");
    failure(
        w.run(&["-o", "test.png", "--format", "svg"], "hi"),
        "conflicts",
    );
    success(w.run(&["-o", "test.png", "--scale", "2"], "hi"));
    let first = fs::read(w.0.join("test.png")).unwrap();
    assert_eq!(&first[..8], b"\x89PNG\r\n\x1a\n");
    failure(w.run(&["-o", "test.png"], "changed"), "already exists");
    assert_eq!(fs::read(w.0.join("test.png")).unwrap(), first);
    success(w.run(&["-o", "test.png", "--force", "--scale", "2"], "changed"));
    assert_ne!(fs::read(w.0.join("test.png")).unwrap(), first);
}

#[test]
fn errors_are_clear_and_leave_no_output() {
    let w = Workspace::new();
    for (flags, message) in [
        (vec!["--theme", "missing"], "unknown theme"),
        (vec!["--theme", "removed-frame", "--light"], "no light variant"),
        (vec!["--language", "not-a-language"], "unknown language"),
        (vec!["--highlight", "0"], "outside"),
        (vec!["--highlight", "2-1"], "outside"),
        (vec!["--highlight", "1-999999999999"], "outside"),
        (vec!["--width", "100"], "leave room"),
        (vec!["--subtitle", "unsupported"], "Auth0"),
    ] {
        let mut args = vec!["-o", "out.png"];
        args.extend(flags);
        failure(w.run(&args, "hello"), message);
        assert!(!w.0.join("out.png").exists());
    }
    failure(w.run(&["-o", "out.png"], "Hello 🦀"), "U+1F980");
}

// The HTML text must preserve source exactly, including blank lines and strings
// resembling the internal template markers. No source text can inject markup.
#[test]
fn html_preserves_source_and_escapes_user_content() {
    let w = Workspace::new();
    let source = "<script>alert('x')</script>\n\n__TITLE__ asset:foo & \"text\"\n\tend\n";
    let html = String::from_utf8(success(w.run(
        &[
            "-o",
            "-",
            "--format",
            "html",
            "--title",
            "<b>__CODE_IMAGES_EDITOR__</b>",
        ],
        source,
    )))
    .unwrap();
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;b&gt;__CODE_IMAGES_EDITOR__&lt;/b&gt;"));
    let code = html
        .split("<code>")
        .nth(1)
        .unwrap()
        .split("</code>")
        .next()
        .unwrap();
    let mut text = String::new();
    let mut in_tag = false;
    for c in code.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(c);
        }
    }
    let text = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    assert_eq!(text, source);
    assert!(html.contains("data:font/ttf;base64,"));
    assert!(!html.contains("src=\"http"));
}

#[test]
fn inference_override_and_line_numbers() {
    let w = Workspace::new();
    fs::write(w.0.join("main.rs"), "fn main() {}\n").unwrap();
    let inferred = success(w.run(
        &["main.rs", "-o", "-", "--format", "html", "--line-numbers"],
        "",
    ));
    let explicit = success(w.run(
        &[
            "-l",
            "rust",
            "-o",
            "-",
            "--format",
            "html",
            "--line-numbers",
        ],
        "fn main() {}\n",
    ));
    assert_eq!(inferred, explicit);
    let plain = String::from_utf8(success(w.run(
        &[
            "main.rs",
            "-l",
            "plain",
            "-o",
            "-",
            "--format",
            "html",
            "--line-numbers",
        ],
        "",
    )))
    .unwrap();
    assert!(!plain.contains("class=\"Editor__editor Editor__showLineNumbers"));
    let forced = String::from_utf8(success(w.run(
        &[
            "main.rs",
            "--theme",
            "removed-frame",
            "--no-line-numbers",
            "-o",
            "-",
            "--format",
            "html",
        ],
        "",
    )))
    .unwrap();
    assert!(forced.contains("class=\"Editor__editor Editor__showLineNumbers"));
}

#[test]
fn png_scale_and_svg_dimensions_agree() {
    let w = Workspace::new();
    let png = success(w.run(
        &[
            "-o", "-", "--format", "png", "--scale", "2", "--width", "600",
        ],
        "one\n\nthree\n",
    ));
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!(width, 1200);
    let svg = String::from_utf8(success(w.run(
        &["-o", "-", "--format", "svg", "--width", "600"],
        "one\n\nthree\n",
    )))
    .unwrap();
    assert!(
        svg.contains(&format!("width=\"600\" height=\"{}\"", height / 2)),
        "{}",
        &svg[..150]
    );
    assert!(!svg.contains("<text"));
    assert!(!svg.contains("<foreignObject"));
}

#[test]
fn svg_shadows_do_not_fill_translucent_windows() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use takumi::prelude::*;
    let w = Workspace::new();
    let png = success(w.run(&["-o", "-", "--format", "png", "--scale", "2"], ""));
    let svg = success(w.run(&["-o", "-", "--format", "svg"], ""));
    let expected = image::load_from_memory(&png).unwrap().into_rgba8();
    let (width, height) = expected.dimensions();
    let html = format!(
        "<img style=\"width:{width}px;height:{height}px\" src=\"data:image/svg+xml;base64,{}\">",
        STANDARD.encode(svg)
    );
    let node = takumi::from_html(&html, FromHtmlOptions::default()).unwrap();
    let fonts = Fonts::default();
    let actual = takumi::render(
        RenderOptions::builder()
            .viewport(Viewport::new((width, height)))
            .node(node)
            .fonts(&fonts)
            .build(),
    )
    .unwrap();
    // Blank, translucent window interior. A filled outer shadow makes these
    // pixels nearly black; a CSS-correct shadow leaves the purple backdrop.
    for (x, y) in [(width / 2, height / 2), (width / 3, height / 2)] {
        let offset = ((y * width + x) * 4) as usize;
        for channel in 0..3 {
            assert!(
                actual.as_raw()[offset + channel].abs_diff(expected[(x, y)][channel]) <= 3,
                "SVG shadow changed interior at {x},{y}: {:?} versus {:?}",
                &actual.as_raw()[offset..offset + 4],
                expected[(x, y)]
            );
        }
    }
}

#[test]
fn all_variants_export_all_formats() {
    let w = Workspace::new();
    let themes: serde_json::Value =
        serde_json::from_str(include_str!("../assets/removed-frames/themes.json")).unwrap();
    let mut variants = 0;
    for (id, theme) in themes.as_object().unwrap() {
        for mode in theme["syntax"].as_object().unwrap().keys() {
            variants += 1;
            for format in ["png", "svg", "html"] {
                let output = success(w.run(
                    &[
                        "--theme",
                        id,
                        &format!("--{mode}"),
                        "-l",
                        "rust",
                        "--title",
                        "hello.rs",
                        "--scale",
                        "2",
                        "-o",
                        "-",
                        "--format",
                        format,
                    ],
                    "fn main() {\n    let x = 42;\n}\n",
                ));
                assert!(!output.is_empty(), "{id}/{mode}/{format}");
                if format == "html" {
                    let html = String::from_utf8(output).unwrap();
                    assert!(!html.contains("src=\"/"), "external image in {id}/{mode}");
                    assert!(
                        !html.contains("src=\"asset:"),
                        "unresolved image in {id}/{mode}"
                    );
                }
            }
        }
    }
    assert_eq!(variants, 65);
}
