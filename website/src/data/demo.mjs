// These are real CLI IDs and appearance flags, shared by the page and asset builder.
export const themes = [
  { id: 'gruvbox', name: 'Gruvbox Dark', cli: 'gruvbox-medium', appearance: 'dark', bg: '#282828', accent: '#fabd2f' },
  { id: 'ayu', name: 'Ayu Light', cli: 'ayu', appearance: 'light', bg: '#fcfcfc', accent: '#fa8d3e' },
  { id: 'nightowl', name: 'Night Owl', cli: 'night-owl', appearance: 'dark', bg: '#011627', accent: '#82aaff' },
  { id: 'catppuccin', name: 'Catppuccin Mocha', cli: 'catppuccin', appearance: 'dark', bg: '#1e1e2e', accent: '#cba6f7' },
  { id: 'solarized', name: 'Solarized Light', cli: 'solarized', appearance: 'light', bg: '#fdf6e3', accent: '#268bd2' },
];
export const source = [
  'fn share(code: &str, theme: &Theme) -> Image {',
  '    let tokens = highlight(code, theme.syntax);',
  '    // Rendered on this Mac. No network.',
  '    let canvas = Canvas::new(640, 64);',
  '    canvas.draw(tokens).export(Png { scale: 2 })',
  '}',
];
export const initialTheme = themes[2];
export function command(theme) {
  return `monoprint share.rs --theme ${theme.cli}${theme.appearance === 'light' ? ' --light' : ''} --highlight 4`;
}
