//! Maintenance tool; no compilation or filesystem registry loading at runtime.
use anyhow::{Context, Result, ensure};
use giallo::Registry;
use std::{env, fs, path::Path, process::Command};

fn main() -> Result<()> {
    let root = env::args()
        .nth(1)
        .context("usage: cargo run --example compile_registry -- GRAMMARS_ROOT")?;
    let root = Path::new(&root);
    ensure!(
        Command::new("node")
            .args(["scripts/import-grammar-metadata.mjs", "--check-inputs"])
            .arg(root)
            .status()
            .context("Node.js is required for the maintenance-only grammar license check")?
            .success(),
        "grammar license/input validation failed"
    );
    let policy: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("licenses/grammar-policy.json")?)?;
    let mut registry = Registry::default();
    // Never sweep the collection directory: it also contains excluded grammars.
    let mut paths: Vec<_> = policy["entries"]
        .as_array()
        .context("grammar policy entries")?
        .iter()
        .filter(|entry| entry["status"] == "included")
        .map(|entry| {
            Ok(root.join("packages/tm-grammars/grammars").join(format!(
                "{}.json",
                entry["name"].as_str().context("grammar name")?
            )))
        })
        .collect::<Result<_>>()?;
    paths.sort();
    for path in paths {
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
    ensure!(
        Command::new("node")
            .args(["scripts/import-grammar-metadata.mjs", "--record-registry"])
            .status()?
            .success(),
        "failed to record registry provenance"
    );
    println!("Wrote embedded registry");
    Ok(())
}
