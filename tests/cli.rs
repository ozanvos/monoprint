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
            "monoprint-test-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn run(&self, args: &[&str], input: &str) -> Output {
        let mut process = Command::new(env!("CARGO_BIN_EXE_monoprint"))
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
fn monoprint_is_the_cli_default_with_a_light_counterpart() {
    let w = Workspace::new();
    for appearance in ["--dark", "--light"] {
        let args = ["-o", "-", "--format", "html", "-l", "ts", appearance];
        let default = success(w.run(&args, "const answer = 42;"));
        let mut explicit = args.to_vec();
        explicit.extend(["--theme", "monoprint"]);
        assert_eq!(default, success(w.run(&explicit, "const answer = 42;")));
        assert!(
            String::from_utf8(default)
                .unwrap()
                .contains("Monoprint__page")
        );
    }
}

#[test]
fn png_compression_is_tunable_without_changing_defaults() {
    let w = Workspace::new();
    let source = "// Hello 👋\nlet answer = 42;";
    let args = ["-o", "-", "--format", "png", "--scale", "2", "-l", "rust"];
    let default = success(w.run(&args, source));
    let renderer = monoprint::Renderer::new();
    let snippet = monoprint::Snippet::new(source).language("rust".parse().unwrap());
    for mode in ["fast", "balanced"] {
        let mut args = args.to_vec();
        args.extend(["--png-compression", mode]);
        let actual = success(w.run(&args, source));
        assert_eq!(
            actual,
            renderer
                .png(
                    &snippet,
                    &monoprint::Style::default(),
                    monoprint::PngOptions::new(monoprint::Scale::X2)
                        .compression(mode.parse().unwrap()),
                )
                .unwrap(),
        );
        if mode == "fast" {
            assert_eq!(actual, default);
        }
    }
    failure(
        w.run(&["-o", "out.png", "--png-compression", "smallest"], ""),
        "use fast or balanced",
    );
    assert!(!w.0.join("out.png").exists());
    for format in ["svg", "html"] {
        let path = format!("out.{format}");
        failure(
            w.run(&["-o", &path, "--png-compression", "fast"], ""),
            "only valid for PNG",
        );
        assert!(!w.0.join(path).exists());
    }
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
        (vec!["--theme", "dusk", "--light"], "no light variant"),
        (vec!["--language", "not-a-language"], "unknown language"),
        (vec!["--highlight", "0"], "outside"),
        (vec!["--highlight", "2-1"], "outside"),
        (vec!["--highlight", "1-999999999999"], "outside"),
        (vec!["--width", "100"], "leave room"),
        (
            vec!["--subtitle", "unsupported"],
            "subtitles are no longer supported",
        ),
    ] {
        let mut args = vec!["-o", "out.png"];
        args.extend(flags);
        failure(w.run(&args, "hello"), message);
        assert!(!w.0.join("out.png").exists());
    }
    failure(w.run(&["-o", "out.png"], "Hello \u{10ffff}"), "U+10FFFF");
}

#[test]
fn source_limit_is_enforced_before_end_of_stream() {
    let w = Workspace::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_monoprint"))
        .args(["-o", "out.html"])
        .current_dir(&w.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Keep the writer open: rejection must not depend on EOF from the producer.
    let mut writer = child.stdin.take().unwrap();
    writer.write_all(&vec![b'x'; 1_048_577]).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("oversized source waited for EOF");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    failure(child.wait_with_output().unwrap(), "source exceeds 1 MiB");
    drop(writer);
    assert!(!w.0.join("out.html").exists());

    fs::write(w.0.join("too-large.txt"), vec![b'x'; 1_048_577]).unwrap();
    failure(
        w.run(&["too-large.txt", "-o", "out.html"], ""),
        "source exceeds 1 MiB",
    );
    fs::write(w.0.join("invalid.txt"), [0xff]).unwrap();
    failure(w.run(&["invalid.txt", "-o", "out.html"], ""), "UTF-8");
    assert!(!w.0.join("out.html").exists());
}

#[test]
fn custom_fonts_are_bounded_and_titles_use_their_actual_font() {
    let w = Workspace::new();
    let font = w.0.join("too-large.ttf");
    fs::File::create(&font)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    failure(
        w.run(&["--font-file", "too-large.ttf", "-o", "out.html"], "x"),
        "font exceeds 64 MiB",
    );
    assert!(!w.0.join("out.html").exists());

    let space = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/space-mono.ttf");
    let inter = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/inter.ttf");
    // Missing characters fail only after fallback; Cyrillic title text is supported.
    for theme in ["monoprint", "thermal-receipt"] {
        for format in ["png", "svg", "html"] {
            failure(
                w.run(
                    &[
                        "--theme",
                        theme,
                        "--font-file",
                        space.to_str().unwrap(),
                        "--title",
                        "\u{10ffff}.rs",
                        "-o",
                        "-",
                        "--format",
                        format,
                    ],
                    "x",
                ),
                "U+10FFFF",
            );
        }
        success(w.run(
            &[
                "--theme",
                theme,
                "--font-file",
                inter.to_str().unwrap(),
                "--title",
                "Привет.rs",
                "-o",
                "-",
                "--format",
                "html",
            ],
            "x",
        ));
    }
    success(w.run(
        &[
            "--font-file",
            space.to_str().unwrap(),
            "--title",
            "Привет.rs",
            "-o",
            "-",
            "--format",
            "html",
        ],
        "x",
    ));
    // Frames without a title field must not reject unused label characters.
    success(w.run(
        &[
            "--theme", "ground", "--title", "🦀", "-o", "-", "--format", "html",
        ],
        "x",
    ));
}

#[test]
fn advanced_typography_and_text_highlights_reach_every_export() {
    let w = Workspace::new();
    let fonts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    let code = fonts.join("fira-code.ttf");
    let comments = fonts.join("space-mono.ttf");
    let listed = String::from_utf8(success(w.run(&["--list-fonts"], ""))).unwrap();
    for font in monoprint::BundledFont::all() {
        assert!(listed.contains(font.id()));
        assert!(listed.contains(font.name()));
    }
    for format in ["png", "svg", "html"] {
        assert_eq!(
            success(w.run(
                &[
                    "--font",
                    "fira-code",
                    "--comment-font",
                    "space-mono",
                    "-l",
                    "rust",
                    "-o",
                    "-",
                    "--format",
                    format
                ],
                "let x = 1; // comment"
            )),
            success(w.run(
                &[
                    "--font-file",
                    code.to_str().unwrap(),
                    "--comment-font-file",
                    comments.to_str().unwrap(),
                    "-l",
                    "rust",
                    "-o",
                    "-",
                    "--format",
                    format
                ],
                "let x = 1; // comment"
            ))
        );
    }
    for format in ["png", "svg", "html"] {
        let bytes = success(w.run(
            &[
                "-l",
                "rust",
                "--font-file",
                code.to_str().unwrap(),
                "--comment-font-file",
                comments.to_str().unwrap(),
                "--ligatures",
                "--highlight",
                "2",
                "--highlight-text",
                "1:5-1:9",
                "--highlight-text",
                "2:4-2:11",
                "-o",
                "-",
                "--format",
                format,
            ],
            "let café = 1;\n// comment\n",
        ));
        assert!(!bytes.is_empty());
        if format == "html" {
            let html = String::from_utf8(bytes).unwrap();
            assert!(html.contains("font-family:var(--monoprint-comment-font,CommentFont);"));
            assert!(html.contains("class=\"highlighted-text\""));
        }
    }
    assert_eq!(
        success(w.run(&["-o", "-", "--format", "svg"], "a => b")),
        success(w.run(&["--no-ligatures", "-o", "-", "--format", "svg"], "a => b"))
    );
    for (args, message) in [
        (vec!["--font", "missing"], "unknown bundled font"),
        (
            vec!["--font", "fira-code", "--font-file", "unused.ttf"],
            "cannot be used with",
        ),
        (
            vec![
                "--comment-font",
                "inter",
                "--comment-font-file",
                "unused.ttf",
            ],
            "cannot be used with",
        ),
        (vec!["--ligatures", "--no-ligatures"], "cannot be used with"),
        (vec!["--highlight-text", "1-2"], "invalid text highlight"),
        (vec!["--highlight-text", "1:1-2:1"], "outside the source"),
    ] {
        let mut command = vec!["-o", "out.png"];
        command.extend(args);
        failure(w.run(&command, "x"), message);
        assert!(!w.0.join("out.png").exists());
    }
    fs::File::create(w.0.join("large.ttf"))
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    failure(
        w.run(&["--comment-font-file", "large.ttf", "-o", "out.png"], "x"),
        "font exceeds 64 MiB",
    );
    assert!(!w.0.join("out.png").exists());
}

#[test]
fn hem_files_infer_the_bundled_grammar_and_export_all_formats() {
    let w = Workspace::new();
    fs::write(
        w.0.join("example.hem"),
        "// Hem\neffect greet(): string { raise \"hello\" }\n",
    )
    .unwrap();
    for format in ["html", "svg", "png"] {
        let inferred = success(w.run(&["example.hem", "-o", "-", "--format", format], ""));
        let explicit = success(w.run(
            &["example.hem", "-l", "hem", "-o", "-", "--format", format],
            "",
        ));
        assert_eq!(inferred, explicit, "{format}");
        assert!(!inferred.is_empty());
    }
}

#[test]
fn typescript_and_tsx_use_distinct_grammars() {
    let w = Workspace::new();
    let source = "const value = <number>42;";
    let expected = success(w.run(&["-l", "typescript", "-o", "-", "--format", "html"], source));
    for extension in ["ts", "mts", "cts"] {
        let name = format!("example.{extension}");
        fs::write(w.0.join(&name), source).unwrap();
        assert_eq!(
            success(w.run(&[&name, "-o", "-", "--format", "html"], "")),
            expected
        );
        assert_eq!(
            success(w.run(&["-l", extension, "-o", "-", "--format", "html"], source)),
            expected
        );
    }
    let jsx = "const view = <div />;";
    fs::write(w.0.join("component.tsx"), jsx).unwrap();
    assert_eq!(
        success(w.run(&["component.tsx", "-o", "-", "--format", "html"], "")),
        success(w.run(&["-l", "tsx", "-o", "-", "--format", "html"], jsx)),
    );
    assert_ne!(
        expected,
        success(w.run(&["-l", "tsx", "-o", "-", "--format", "html"], source)),
        "TypeScript angle-bracket assertions must not use the JSX grammar",
    );
}

#[test]
fn render_limits_explain_odd_scale_supersampling() {
    let w = Workspace::new();
    failure(
        w.run(&["-o", "out.png", "--scale", "5", "--width", "4000"], "x"),
        "internal render pixels; odd PNG scales render at 2x the requested scale",
    );
    assert!(!w.0.join("out.png").exists());
}

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
    let hidden = String::from_utf8(success(w.run(
        &[
            "main.rs",
            "--theme",
            "ordinal",
            "--no-line-numbers",
            "-o",
            "-",
            "--format",
            "html",
        ],
        "",
    )))
    .unwrap();
    assert!(!hidden.contains("class=\"Editor__editor Editor__showLineNumbers"));
    let default = String::from_utf8(success(w.run(
        &[
            "main.rs", "--theme", "ordinal", "-o", "-", "--format", "html",
        ],
        "",
    )))
    .unwrap();
    assert!(default.contains("class=\"Editor__editor Editor__showLineNumbers"));
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
    let w = Workspace::new();
    let png = success(w.run(
        &[
            "--theme", "ayu", "-o", "-", "--format", "png", "--scale", "2",
        ],
        "",
    ));
    let svg = success(w.run(&["--theme", "ayu", "-o", "-", "--format", "svg"], ""));
    let expected = image::load_from_memory(&png).unwrap().into_rgba8();
    let (width, height) = expected.dimensions();
    let actual = rasterize_svg(&svg, width, height);
    // Blank, translucent window interior. A filled outer shadow makes these
    // pixels nearly black; a CSS-correct shadow leaves the companion backdrop.
    for (x, y) in [(width / 2, height / 2), (width / 3, height / 2)] {
        for channel in 0..3 {
            assert!(
                actual[(x, y)][channel].abs_diff(expected[(x, y)][channel]) <= 3,
                "SVG shadow changed interior at {x},{y}: {:?} versus {:?}",
                actual[(x, y)],
                expected[(x, y)]
            );
        }
    }
}

fn rasterize_svg(svg: &[u8], width: u32, height: u32) -> image::RgbaImage {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use takumi::prelude::*;
    let html = format!(
        "<img style=\"width:{width}px;height:{height}px\" src=\"data:image/svg+xml;base64,{}\">",
        STANDARD.encode(svg)
    );
    let node = takumi::from_html(&html, FromHtmlOptions::default()).unwrap();
    let fonts = Fonts::default();
    let actual = takumi_raster::render(
        takumi_raster::RenderOptions::builder()
            .viewport(Viewport::new((width, height)))
            .node(node)
            .fonts(&fonts)
            .build(),
    )
    .unwrap();
    image::RgbaImage::from_raw(width, height, actual.into_raw()).unwrap()
}

#[test]
fn svg_line_highlights_are_not_painted_twice() {
    let w = Workspace::new();
    let source = "\n\n";
    let args = [
        "--theme",
        "ayu",
        "-l",
        "ts",
        "--padding",
        "32",
        "--width",
        "460",
        "--line-numbers",
        "--highlight",
        "2",
        "-o",
        "-",
        "--scale",
        "2",
    ];
    let mut png_args = args.to_vec();
    png_args.extend(["--format", "png"]);
    let expected = image::load_from_memory(&success(w.run(&png_args, source)))
        .unwrap()
        .into_rgba8();
    let mut svg_args = args.to_vec();
    svg_args.extend(["--format", "svg"]);
    let svg = success(w.run(&svg_args, source));
    let actual = rasterize_svg(&svg, expected.width(), expected.height());
    // The middle blank row is highlighted. Away from glyphs and the frame,
    // duplicate background painting would brighten its solid interior.
    let mut checked = 0;
    for y in 100..expected.height().saturating_sub(70) {
        for x in [300, 400, 500, 600] {
            if expected[(x, y)] != expected[(x, y - 1)] {
                continue;
            }
            for channel in 0..3 {
                assert!(
                    actual[(x, y)][channel].abs_diff(expected[(x, y)][channel]) <= 3,
                    "highlight at {x},{y}: {:?} versus {:?}",
                    actual[(x, y)],
                    expected[(x, y)]
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 100);
}

#[test]
fn all_variants_export_all_formats() {
    let w = Workspace::new();
    let themes = monoprint::Theme::all();
    let mut variants = 0;
    for theme in themes {
        let id = theme.id();
        for appearance in [monoprint::Appearance::Dark, monoprint::Appearance::Light] {
            if !theme.supports(appearance) {
                continue;
            }
            let mode = if appearance == monoprint::Appearance::Dark {
                "dark"
            } else {
                "light"
            };
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
    assert_eq!(variants, 80);
}
