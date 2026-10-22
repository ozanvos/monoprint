//! Regenerate native token palettes using Giallo's own TextMate theme matcher.
//! cargo run --example compile_pierre_palettes -- [--check]
use anyhow::{Context, Result, ensure};
use giallo::{HighlightOptions, Registry, ThemeVariant};
use pierre_highlights_rs::TokenKind;
use serde_json::json;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scopes: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../assets/pierre-token-scopes.json"))?;
    ensure!(
        scopes.len() == TokenKind::ALL.len(),
        "token scope catalog changed"
    );
    let grammar = json!({"name":"monoprint-pierre-palette", "scopeName":"source.monoprint-pierre-palette", "patterns":TokenKind::ALL.iter().map(|kind| {
        let scope = scopes.get(kind.name()).expect("every token has a reviewed scope");
        json!({"match":format!("^K{:02}$",kind.id()),"name":scope})
    }).collect::<Vec<_>>()});
    let path = std::env::temp_dir().join(format!(
        "monoprint-pierre-palette-{}.json",
        std::process::id()
    ));
    fs::write(&path, serde_json::to_vec(&grammar)?)?;
    let mut registry = Registry::load(include_bytes!("../assets/registry.zst"))?;
    let added = registry.add_grammar_from_path(&path);
    fs::remove_file(&path)?;
    added?;
    registry.link_grammars();
    let sample = TokenKind::ALL
        .iter()
        .map(|kind| format!("K{:02}", kind.id()))
        .collect::<Vec<_>>()
        .join("\n");
    let mut palettes = BTreeMap::new();
    let mut paths = fs::read_dir(root.join("assets/themes"))?
        .map(|entry| Ok(entry?.path()))
        .collect::<Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        let theme: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
        let name = theme["name"].as_str().context("theme name")?;
        if name == "monoprint-comments" {
            continue;
        }
        let result = registry.highlight(
            &sample,
            &HighlightOptions::new("monoprint-pierre-palette", ThemeVariant::Single(name))
                .merge_whitespace(false),
        )?;
        let ThemeVariant::Single(theme) = result.theme else {
            unreachable!()
        };
        let styles = result
            .tokens
            .into_iter()
            .map(|line| {
                ensure!(
                    line.len() == 1,
                    "palette probe must produce one token per kind"
                );
                let ThemeVariant::Single(style) = line[0].style else {
                    unreachable!()
                };
                Ok(json!([
                    style.foreground.as_hex(),
                    style.background.as_hex(),
                    style.font_style
                ]))
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(styles.len() == 73, "palette must cover every token");
        palettes.insert(
            name.to_owned(),
            json!({"background":theme.default_style.background.as_hex(),"styles":styles}),
        );
    }
    let output = serde_json::to_string(&palettes)? + "\n";
    let path = root.join("assets/pierre-palettes.json");
    if std::env::args().any(|arg| arg == "--check") {
        ensure!(
            fs::read_to_string(path)? == output,
            "Pierre palettes are stale; regenerate them"
        );
    } else {
        fs::write(path, output)?;
    }
    println!(
        "Verified {} native palettes, 73 token styles each",
        palettes.len()
    );
    Ok(())
}
