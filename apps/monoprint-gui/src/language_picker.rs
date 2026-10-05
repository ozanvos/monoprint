//! Studio language curation; the renderer's full catalog stays available.
use crate::studio::Choice;
use gpui_kit::{
    component::{
        IndexPath,
        select::{SelectDelegate, SelectItem},
    },
    *,
};

// Primary set plus the additional languages selected for Studio.
const PRIMARY: &[&str] = &[
    "ahk",
    "applescript",
    "asciidoc",
    "astro",
    "awk",
    "bat",
    "bibtex",
    "c",
    "clojure",
    "cmake",
    "common-lisp",
    "coq",
    "cpp",
    "crystal",
    "csharp",
    "css",
    "csv",
    "cue",
    "d",
    "dart",
    "diff",
    "docker",
    "dotenv",
    "elixir",
    "elm",
    "emacs-lisp",
    "erlang",
    "fennel",
    "fish",
    "fsharp",
    "gdshader",
    "gleam",
    "go",
    "graphql",
    "haskell",
    "haxe",
    "hcl",
    "hem",
    "hlsl",
    "html",
    "http",
    "ini",
    "java",
    "javascript",
    "json",
    "json5",
    "jsonc",
    "jsonl",
    "jsonnet",
    "jsx",
    "julia",
    "just",
    "kotlin",
    "latex",
    "lean",
    "lua",
    "make",
    "markdown",
    "mdx",
    "mermaid",
    "mojo",
    "nim",
    "nix",
    "nushell",
    "objective-c",
    "ocaml",
    "odin",
    "perl",
    "php",
    "plain",
    "plsql",
    "powershell",
    "removed-frame",
    "prolog",
    "proto",
    "python",
    "r",
    "raku",
    "regexp",
    "rst",
    "ruby",
    "rust",
    "scala",
    "scheme",
    "scss",
    "shaderlab",
    "shellscript",
    "shellsession",
    "solidity",
    "sql",
    "ssh-config",
    "svelte",
    "swift",
    "systemd",
    "terraform",
    "toml",
    "tsv",
    "tsx",
    "typescript",
    "typst",
    "vala",
    "vb",
    "viml",
    "vue",
    "wasm",
    "wgsl",
    "xml",
    "yaml",
    "zig",
];

pub struct LanguageList {
    all: Vec<Choice>,
    matched: Vec<Choice>,
    // Keep existing saved/file-inferred selections readable without offering
    // these languages as picker rows or search results.
    unlisted: Vec<Choice>,
}

impl LanguageList {
    pub fn new() -> Self {
        let (mut all, unlisted): (Vec<_>, Vec<_>) = monoprint::Language::all()
            .map(|language| Choice {
                id: language.id().into(),
                name: language.name().into(),
                preview: None,
            })
            .partition(|item| PRIMARY.contains(&item.id.as_str()));
        all.sort_by_key(|item| item.name.to_lowercase());
        Self {
            matched: all.clone(),
            all,
            unlisted,
        }
    }

    fn search(&mut self, query: &str) {
        let query = query.trim().to_lowercase();
        self.matched = self
            .all
            .iter()
            .filter(|item| item.matches(&query))
            .cloned()
            .collect();
        if !query.is_empty() {
            self.matched.sort_by_key(|item| {
                let name = item.name.to_lowercase();
                if name == query || item.id == query {
                    0
                } else if name.starts_with(&query) || item.id.starts_with(&query) {
                    1
                } else {
                    2
                }
            });
        }
    }
}

impl SelectDelegate for LanguageList {
    type Item = Choice;

    fn sections_count(&self, _: &App) -> usize {
        1
    }

    fn items_count(&self, section: usize) -> usize {
        if section == 0 { self.matched.len() } else { 0 }
    }

    fn item(&self, ix: IndexPath) -> Option<&Choice> {
        match ix.section {
            0 => self.matched.get(ix.row),
            1 => self.unlisted.get(ix.row),
            _ => None,
        }
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SelectItem<Value = V>,
        V: PartialEq,
    {
        [&self.matched, &self.unlisted]
            .into_iter()
            .enumerate()
            .find_map(|(section, items)| {
                items
                    .iter()
                    .position(|item| item.value() == value)
                    .map(|row| IndexPath::default().section(section).row(row))
            })
    }

    fn perform_search(&mut self, query: &str, _: &mut Window, _: &mut App) -> Task<()> {
        self.search(query);
        Task::ready(())
    }
}

#[cfg(test)]
mod tests {
    use super::{LanguageList, PRIMARY};
    use gpui_kit::component::{IndexPath, select::SelectDelegate};

    #[test]
    fn only_the_selected_languages_are_available_in_the_picker() {
        let mut list = LanguageList::new();
        assert_eq!(list.items_count(0), 109);
        assert_eq!(list.items_count(0), PRIMARY.len());
        assert_eq!(list.items_count(1), 0);
        for (query, expected) in [
            ("typescript", "typescript"),
            (".ts", "typescript"),
            ("protobuf", "proto"),
            ("hem", "hem"),
            ("elisp", "emacs-lisp"),
            ("fennel", "fennel"),
            ("haxe", "haxe"),
            ("lean", "lean"),
            ("mojo", "mojo"),
            ("pl/sql", "plsql"),
            ("prolog", "prolog"),
            ("raku", "raku"),
            ("rocq", "coq"),
            ("regexp", "regexp"),
            ("vimscript", "viml"),
            ("vala", "vala"),
        ] {
            list.search(query);
            assert_eq!(list.item(IndexPath::default()).unwrap().id, expected);
        }
        for language in monoprint::Language::all().filter(|l| !PRIMARY.contains(&l.id())) {
            list.search(language.id());
            assert!(list.matched.iter().all(|item| item.id != language.id()));
            assert_eq!(list.items_count(1), 0);
        }
        list.search("");
        // A previously saved excluded language keeps its label and rendering,
        // but its section is never included in the dropdown.
        let ix = list.position(&"angular-ts".to_owned()).unwrap();
        assert_eq!(ix.section, 1);
        assert_eq!(list.item(ix).unwrap().id, "angular-ts");
        assert_eq!(list.items_count(1), 0);
    }
}
