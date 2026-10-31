// These are real CLI IDs and appearance flags, shared by the page and asset builder.
// `palette` holds each theme's published token colours for the picker thumbnails.
export const themes = [
  { id: 'gruvbox', name: 'Gruvbox Dark', cli: 'gruvbox-medium', appearance: 'dark', palette: { bg: '#282828', kw: '#fb4934', fn: '#b8bb26', type: '#fabd2f', num: '#d3869b', comment: '#928374' } },
  { id: 'ayu', name: 'Ayu Light', cli: 'ayu', appearance: 'light', palette: { bg: '#fcfcfc', kw: '#fa8d3e', fn: '#f2ae49', type: '#399ee6', num: '#a37acc', comment: '#adaeb1' } },
  { id: 'nightowl', name: 'Night Owl', cli: 'night-owl', appearance: 'dark', palette: { bg: '#011627', kw: '#c792ea', fn: '#82aaff', type: '#ffcb8b', num: '#f78c6c', comment: '#637777' } },
  { id: 'catppuccin', name: 'Catppuccin Mocha', cli: 'catppuccin', appearance: 'dark', palette: { bg: '#1e1e2e', kw: '#cba6f7', fn: '#89b4fa', type: '#f9e2af', num: '#fab387', comment: '#7f849c' } },
  { id: 'solarized', name: 'Solarized Light', cli: 'solarized', appearance: 'light', palette: { bg: '#fdf6e3', kw: '#859900', fn: '#268bd2', type: '#b58900', num: '#d33682', comment: '#93a1a1' } },
];
export const source = [
  'fn share(code: &str, theme: &Theme) -> Image {',
  '    let tokens = highlight(code, theme.syntax);',
  '    // Rendered offline. No network.',
  '    let canvas = Canvas::new(640, 64);',
  '    canvas.draw(tokens).export(Png { scale: 2 })',
  '}',
];
export const initialTheme = themes[2];
export function command(theme) {
  return `monoprint share.rs -o share.png --theme ${theme.cli}${theme.appearance === 'light' ? ' --light' : ''} --highlight 4`;
}
