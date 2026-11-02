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
