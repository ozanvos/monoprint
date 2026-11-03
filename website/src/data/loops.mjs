// Content for the looping demos on the interface cards.
// The terminal recalls its last command with each of these theme IDs in turn.
export const loopThemes = ['night-owl', 'solarized', 'gruvbox-medium', 'ayu', 'catppuccin'];
// PNG sizes in bytes of share.rs rendered with `png(source, { theme, scale })` at scale 2 and 4.
export const renders = [
  { theme: 'night-owl', bytes: { 2: 109161, 4: 231835 } },
  { theme: 'gruvbox-medium', bytes: { 2: 94528, 4: 201189 } },
  { theme: 'catppuccin', bytes: { 2: 112823, 4: 241092 } },
];
// What the Studio demo retypes, alternating, as the `name` in its snippet.
export const names = ['Monoprint', 'Proof sheet'];
// Fig. 1 plays the Studio with this file open, in GitHub Default. The CLI renders its preview in
// each appearance and backdrop the demo switches between, at the Studio's frame settings.
export const palette = [
  '// Small details, clear results.',
  'struct Palette {',
  '    name: String,',
  '    dark: bool,',
  '}',
  '',
  'fn main() {',
  '    let palette = Palette {',
  '        name: "Monoprint".into(),',
  '        dark: false,',
  '    };',
  '    println!("{}", palette.name);',
  '}',
];
export const previews = [['light', 'solid'], ['dark', 'solid'], ['light', 'lattice'], ['dark', 'lattice']];
