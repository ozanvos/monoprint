//! Maintenance tool; no compilation or filesystem registry loading at runtime.
use anyhow::{Context, Result};
use giallo::Registry;
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    let root = env::args()
        .nth(1)
        .context("usage: cargo run --example compile_registry -- GRAMMARS_ROOT")?;
    let root = Path::new(&root);
    let mut registry = Registry::default();
    let mut paths: Vec<_> = fs::read_dir(root.join("packages/tm-grammars/grammars"))?
        .map(|p| p.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    paths.sort();
    for path in paths
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
    {
        registry
            .add_grammar_from_path(&path)
            .with_context(|| format!("grammar {}", path.display()))?;
    }
    let aliases: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("assets/grammar-metadata.json")?)?;
    for grammar in aliases.as_array().context("grammar metadata is an array")? {
        let name = grammar["name"].as_str().context("grammar name")?;
        if let Some(aliases) = grammar["aliases"].as_array() {
            for alias in aliases {
                registry.add_alias(name, alias.as_str().context("grammar alias")?);
            }
        }
    }
    registry.add_plain_grammar(&["text", "txt", "plaintext"])?;
    let mut themes: Vec<_> = fs::read_dir("assets/themes")?
        .map(|p| p.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    themes.sort();
    for path in themes {
        registry
            .add_theme_from_path(&path)
            .with_context(|| format!("theme {}", path.display()))?;
    }
    fs::write("assets/registry.zst", registry.dump()?)?;
    println!("Wrote embedded registry");
    Ok(())
}
