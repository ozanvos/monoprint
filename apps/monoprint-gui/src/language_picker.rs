//! Studio language curation; the renderer's full catalog stays available.
use crate::studio::Choice;
use crate::ui::picker::PickerSource;

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
    "prisma",
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

impl PickerSource for LanguageList {
    fn options(&self) -> &[Choice] {
        &self.matched
    }
    fn selected(&self, id: &str) -> Option<&Choice> {
        self.all
            .iter()
            .chain(&self.unlisted)
            .find(|item| item.id == id)
    }
    fn search(&mut self, query: &str) {
        self.search(query);
    }
}

#[cfg(test)]
mod tests {
    use super::{LanguageList, PRIMARY};
    use crate::ui::picker::PickerSource;

    #[test]
    fn only_the_selected_languages_are_available_in_the_picker() {
        let mut list = LanguageList::new();
        assert_eq!(list.options().len(), 109);
        assert_eq!(list.options().len(), PRIMARY.len());
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
            assert_eq!(list.options().first().unwrap().id, expected);
        }
        for language in monoprint::Language::all().filter(|l| !PRIMARY.contains(&l.id())) {
            list.search(language.id());
            assert!(list.matched.iter().all(|item| item.id != language.id()));
        }
        list.search("");
        // A previously saved excluded language keeps its label and rendering,
        // but its section is never included in the dropdown.
        assert_eq!(list.selected("angular-ts").unwrap().id, "angular-ts");
        assert!(list.options().iter().all(|item| item.id != "angular-ts"));
    }
}
