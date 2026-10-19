# Editor themes and canvas backgrounds

The catalog contains 48 Monoprint Originals and 21 editor presets covering 31 palettes across 12 families.

| Family | Included palettes |
| --- | --- |
| Ayu | Dark, Mirage, Light |
| Catppuccin | Frappé, Latte, Macchiato, Mocha |
| Dracula | Original, Soft |
| GitHub | Dark Default, Dark Dimmed, Light Default |
| Gruvbox | Dark and Light, each in Hard, Medium and Soft |
| Night Owl | Night Owl, Light Owl |
| Nord | Nord |
| One Dark Pro | One Dark Pro |
| Rosé Pine | Original, Moon, Dawn |
| Solarized | Dark, Light |
| SynthWave '84 | Neon Dreams token glow |
| Tokyo Night | Night, Storm, Light |

Every preset supports Theme, Solid, Lattice, Halftone and None backgrounds in
the CLI, Rust library, TypeScript binding and Studio. Canvas colors follow the
chosen palette. None makes the surroundings transparent while retaining the
code window. Theme uses the selected original frame or editor palette's companion gradient. The old transparency option remains an alias.
Legacy light-palette IDs select their light appearance automatically.

## Curated light/dark pairs

The gallery, CLI listing and Rust/Node catalogs list each pair once. Appearance
switching selects the complete original palette, including its frame and backdrop.

| Preset ID | Dark palette | Light palette |
| --- | --- | --- |
| `ayu` | Ayu Dark | Ayu Light |
| `catppuccin` | Catppuccin Mocha | Catppuccin Latte |
| `github-default` | GitHub Dark Default | GitHub Light Default |
| `gruvbox-hard` | Gruvbox Dark Hard | Gruvbox Light Hard |
| `gruvbox-medium` | Gruvbox Dark Medium | Gruvbox Light Medium |
| `gruvbox-soft` | Gruvbox Dark Soft | Gruvbox Light Soft |
| `night-owl` | Night Owl | Light Owl |
| `rose-pine` | Rosé Pine | Rosé Pine Dawn |
| `solarized` | Solarized Dark | Solarized Light |
| `tokyo-night` | Tokyo Night | Tokyo Night Light |

All imported editor-palette IDs still parse, with their original default appearance. For example,
`--theme ayu-light` equals `--theme ayu --light`; `--theme ayu-light --dark` selects
Ayu Dark. Saved Studio drafts migrate to the paired preset while retaining their
appearance. Alias names remain searchable. No palette colors or token rules change.

Ayu Mirage, Catppuccin Frappé/Macchiato, GitHub Dimmed, Rosé Pine Moon and Tokyo
Night Storm remain distinct dark-only presets. The curation chooses one primary
dark counterpart for each light palette rather than duplicating light tiles or
inventing additional palette variants. Gruvbox pairs preserve contrast level.

Pairing is defined in [pairs.json](../assets/editor/pairs.json), separate from imported
palette files and their provenance receipts. Update this mapping when curating
pairs, then regenerate Studio thumbnails and Node type declarations. The original
31 palette files, licenses and compressed syntax registry remain unchanged.

## Source and license boundary

Imported TextMate rules and palette colors are pinned by source revision and
SHA-256 in [the input manifest](../assets/editor/sources.json). The normalized
theme files retain their upstream MIT licenses, recorded in
[the notice bundle](../licenses/editor-themes-NOTICE.txt) and
[the import receipt](../licenses/editor-themes-provenance.json).
Original Monoprint frame styling, gradients and patterns are Apache-2.0.
No upstream wallpapers or extension runtime code is bundled. Attribution stays
in project and distribution notices, not on exported images.

SynthWave uses the pinned upstream color-specific glow mapping, implemented with
CSS text shadows. Small fixes in the existing raster and SVG backends use each
token's resolved shadow style, preserving inline shaping and wrapping. The pixel
test checks both visible colored halos and explicit `text-shadow:none` overrides.

These are TextMate palette ports with Monoprint frames, not complete VS Code
screenshots. Language-server semantic highlighting is not reproduced. New
palettes have not been certified pixel-identical to VS Code.
