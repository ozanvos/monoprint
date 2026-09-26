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
    // Space Mono lacks Cyrillic while Inter has it. These titles follow CodeMono,
    // including a user override; Candy's title stays Inter when code uses Space Mono.
    for theme in ["removed-frame", "removed-frame"] {
        for format in ["png", "svg", "html"] {
            failure(
                w.run(
                    &[
                        "--theme",
                        theme,
                        "--font-file",
                        space.to_str().unwrap(),
                        "--title",
                        "Привет.rs",
                        "-o",
                        "-",
                        "--format",
                        format,
                    ],
                    "x",
                ),
                "title font cannot display 'П' (U+041F)",
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
            "--theme", "removed-frame", "--title", "🦀", "-o", "-", "--format", "html",
        ],
        "x",
    ));
}

#[test]
fn stripe_geometry_does_not_rewrite_source_text() {
    let w = Workspace::new();
    let source = format!("StripeFrame__isSmall\n{}", "\n".repeat(12));
    let html = String::from_utf8(success(w.run(
        &["--theme", "removed-frame", "-o", "-", "--format", "html"],
        &source,
    )))
    .unwrap();
    let code = html
        .split("<code>")
        .nth(1)
        .unwrap()
        .split("</code>")
        .next()
        .unwrap();
    assert!(code.contains(">StripeFrame__isSmall</span>"));
    // The measured large frame must still lose its small-layout class.
    let frame = html
        .split("<body>")
        .nth(1)
        .unwrap()
        .split("<code>")
        .next()
        .unwrap();
    assert!(!frame.contains("StripeFrame__isSmall"));
}

#[test]
fn typescript_module_extensions_use_tsx() {
    let w = Workspace::new();
    let source = "const view = <div />;";
    let expected = success(w.run(&["-l", "tsx", "-o", "-", "--format", "html"], source));
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
fn auth0_subtitle_is_metadata_not_visible_text() {
    let w = Workspace::new();
    let args = [
        "--theme", "removed-frame", "-l", "ts", "-o", "-", "--format", "png", "--scale", "2",
    ];
    let expected = success(w.run(&args, "const x = 1;"));
    let mut with_subtitle = args.to_vec();
    with_subtitle.extend(["--subtitle", "🦀"]);
    assert_eq!(success(w.run(&with_subtitle, "const x = 1;")), expected);
    let html = String::from_utf8(success(w.run(
        &[
            "--theme",
            "removed-frame",
            "-l",
            "ts",
            "-o",
            "-",
            "--format",
            "html",
            "--subtitle",
            "🦀",
        ],
        "const x = 1;",
    )))
    .unwrap();
    assert!(html.contains("data-value=\"🦀\""));
    assert!(html.contains(">TSX</span>"));
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
    let w = Workspace::new();
    let png = success(w.run(&["-o", "-", "--format", "png", "--scale", "2"], ""));
    let svg = success(w.run(&["-o", "-", "--format", "svg"], ""));
    let expected = image::load_from_memory(&png).unwrap().into_rgba8();
    let (width, height) = expected.dimensions();
    let actual = rasterize_svg(&svg, width, height);
    // Blank, translucent window interior. A filled outer shadow makes these
    // pixels nearly black; a CSS-correct shadow leaves the purple backdrop.
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
    let actual = takumi::render(
        RenderOptions::builder()
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
    let source = format!(
        "{}// {}\n\n// final line\n",
        include_str!("../fixtures/example.ts"),
        "wrap this long line ".repeat(6)
    );
    let args = [
        "--theme",
        "removed-frame",
        "-l",
        "ts",
        "--padding",
        "32",
        "--width",
        "460",
        "--line-numbers",
        "--title",
        "wrapped.ts",
        "--highlight",
        "2,9",
        "-o",
        "-",
        "--scale",
        "2",
    ];
    let mut png_args = args.to_vec();
    png_args.extend(["--format", "png"]);
    let expected = image::load_from_memory(&success(w.run(&png_args, &source)))
        .unwrap()
        .into_rgba8();
    let mut svg_args = args.to_vec();
    svg_args.extend(["--format", "svg"]);
    let svg = success(w.run(&svg_args, &source));
    let actual = rasterize_svg(&svg, expected.width(), expected.height());
    // The highlighted blank ninth row spans both the gutter and the code area.
    // A second inherited background brightens only the code area by ~20 levels.
    for x in [80, 160, 200, 400, 700, 840] {
        for channel in 0..3 {
            assert!(
                actual[(x, 860)][channel].abs_diff(expected[(x, 860)][channel]) <= 3,
                "highlight at {x}: {:?} versus {:?}",
                actual[(x, 860)],
                expected[(x, 860)]
            );
        }
    }
}

#[test]
fn native_images_match_independent_browser_references() {
    let w = Workspace::new();
    let source = include_str!("../fixtures/example.ts");
    let wrapped = format!("{source}// {}", "wrap this long line ".repeat(12));
    let numbered = format!(
        "{source}// {}\n\n// final line\n",
        "wrap this long line ".repeat(6)
    );
    struct ReferenceCase<'a> {
        name: &'a str,
        flags: &'a [&'a str],
        input: &'a str,
        reference: &'a [u8],
        tolerance: f64,
    }
    let cases = [
        ReferenceCase {
            name: "candy-wrap",
            flags: &[
                "--theme",
                "removed-frame",
                "--light",
                "--padding",
                "32",
                "--width",
                "460",
            ],
            input: &wrapped,
            reference: include_bytes!("visual/candy-wrap.png"),
            tolerance: 2.8,
        },
        ReferenceCase {
            name: "candy-wrap-numbers",
            flags: &[
                "--theme",
                "removed-frame",
                "--padding",
                "32",
                "--width",
                "460",
                "--line-numbers",
                "--title",
                "wrapped.ts",
                "--highlight",
                "2,9",
            ],
            input: &numbered,
            reference: include_bytes!("visual/candy-wrap-numbers.png"),
            tolerance: 2.2,
        },
        ReferenceCase {
            name: "removed-frame",
            flags: &["--theme", "removed-frame"],
            input: source,
            reference: include_bytes!("visual/nuxt-dark.png"),
            tolerance: 1.6,
        },
        ReferenceCase {
            name: "removed-frame",
            flags: &["--theme", "removed-frame"],
            input: source,
            reference: include_bytes!("visual/firecrawl-dark.png"),
            tolerance: 2.0,
        },
    ];
    for ReferenceCase {
        name,
        flags,
        input,
        reference,
        tolerance,
    } in cases
    {
        let mut args = vec!["-o", "-", "--format", "png", "--scale", "2", "-l", "ts"];
        args.extend_from_slice(flags);
        let actual = image::load_from_memory(&success(w.run(&args, input)))
            .unwrap()
            .into_rgb8();
        let expected = image::load_from_memory(reference).unwrap().into_rgb8();
        assert_eq!(actual.dimensions(), expected.dimensions(), "{name} layout");
        let mae = actual
            .as_raw()
            .iter()
            .zip(expected.as_raw())
            .map(|(a, b)| f64::from(a.abs_diff(*b)))
            .sum::<f64>()
            / actual.as_raw().len() as f64;
        assert!(
            mae < tolerance,
            "{name}: mean RGB difference {mae} exceeds {tolerance}"
        );
        if name == "candy-wrap-numbers" {
            // The first listed CSS shadow is the thin light border, above the dark shadow.
            for x in [200, 460, 700] {
                for channel in 0..3 {
                    assert!(
                        actual[(x, 62)][channel].abs_diff(expected[(x, 62)][channel]) <= 5,
                        "window border at {x}"
                    );
                }
            }
        }
        if name == "removed-frame" {
            let bounds = |im: &image::RgbImage| {
                im.enumerate_pixels()
                    .filter(|(_, y, p)| {
                        (470..550).contains(y)
                            && p[0] > 30
                            && p[0].saturating_sub(p[1]) > 20
                            && p[1].saturating_sub(p[2]) > 5
                    })
                    .fold((u32::MAX, 0, u32::MAX), |(left, right, top), (x, y, _)| {
                        (left.min(x), right.max(x), top.min(y))
                    })
            };
            let a = bounds(&actual);
            let e = bounds(&expected);
            assert!(
                a.0.abs_diff(e.0) <= 2 && a.1.abs_diff(e.1) <= 2 && a.2.abs_diff(e.2) <= 2,
                "ASCII bounds: {a:?} versus {e:?}"
            );
        }
    }
}

#[test]
fn nuxt_glow_survives_viewport_clipping_at_export_scales() {
    let w = Workspace::new();
    let reference = image::load_from_memory(include_bytes!("visual/nuxt-dark.png"))
        .unwrap()
        .into_rgb8();
    for scale in [1, 2, 4] {
        let png = success(w.run(
            &[
                "-o",
                "-",
                "--format",
                "png",
                "--theme",
                "removed-frame",
                "-l",
                "ts",
                "--scale",
                &scale.to_string(),
            ],
            include_str!("../fixtures/example.ts"),
        ));
        let actual = image::load_from_memory(&png).unwrap().into_rgb8();
        assert_eq!(actual.dimensions(), (613 * scale, 342 * scale));
        for (x, y) in [(0, 0), (10, 10), (32, 32), (50, 0), (0, 100)] {
            for channel in 0..3 {
                assert!(
                    actual[(x * scale, y * scale)][channel]
                        .abs_diff(reference[(x * 2, y * 2)][channel])
                        <= 3,
                    "Nuxt glow: scale {scale} at {x},{y}"
                );
            }
        }
    }
}

#[test]
fn partner_title_fonts_and_alignment_match_browser_references() {
    let w = Workspace::new();
    let cases: [(&str, bool, &[u8]); 3] = [
        (
            "removed-frame",
            false,
            include_bytes!("visual/triggerdev-title.png"),
        ),
        (
            "removed-frame",
            true,
            include_bytes!("visual/removed_frame-title.png"),
        ),
        ("removed-frame", false, include_bytes!("visual/resend-title.png")),
    ];
    for (theme, light, reference) in cases {
        let expected = image::load_from_memory(reference).unwrap().into_rgba8();
        let bounds = |image: &image::RgbaImage| {
            image
                .enumerate_pixels()
                .filter(|(x, y, pixel)| {
                    (145..400).contains(x)
                        && (140..200).contains(y)
                        && if light {
                            pixel[0] < 100
                        } else {
                            pixel[0] > 100
                        }
                })
                .fold(
                    (u32::MAX, u32::MAX, 0, 0),
                    |(left, top, right, bottom), (x, y, _)| {
                        (left.min(x), top.min(y), right.max(x), bottom.max(y))
                    },
                )
        };
        let expected_bounds = bounds(&expected);
        assert_ne!(expected_bounds.0, u32::MAX, "reference title exists");
        for format in ["png", "svg"] {
            let output = success(w.run(
                &[
                    "--theme",
                    theme,
                    if light { "--light" } else { "--dark" },
                    "-l",
                    "ts",
                    "--title",
                    "example.ts",
                    "--scale",
                    "2",
                    "-o",
                    "-",
                    "--format",
                    format,
                ],
                include_str!("../fixtures/example.ts"),
            ));
            let actual = if format == "svg" {
                rasterize_svg(&output, expected.width(), expected.height())
            } else {
                image::load_from_memory(&output).unwrap().into_rgba8()
            };
            assert_eq!(
                actual.dimensions(),
                expected.dimensions(),
                "{theme}/{format}"
            );
            let actual_bounds = bounds(&actual);
            for (a, b) in [
                (actual_bounds.0, expected_bounds.0),
                (actual_bounds.1, expected_bounds.1),
                (actual_bounds.2, expected_bounds.2),
                (actual_bounds.3, expected_bounds.3),
            ] {
                assert!(
                    a.abs_diff(b) <= 3,
                    "{theme}/{format} title bounds: {actual_bounds:?} vs {expected_bounds:?}"
                );
            }
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
