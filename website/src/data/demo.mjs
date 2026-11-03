// These are real CLI IDs and appearance flags, shared by the page and asset builder.
// `palette` holds each theme's published token colours for the picker thumbnails. `height` is the
// sample's height at 1280 px wide: an Original draws its frame into the image, so it runs taller.
export const themes = [
  { id: 'cyanotype', name: 'Cyanotype', cli: 'cyanotype', appearance: 'dark', original: true, height: 1036, palette: { bg: '#143a62', kw: '#ffffff', fn: '#ffffff', type: '#a8d1ff', num: '#ffb4a2', comment: '#87a9d1' } },
  { id: 'phosphor', name: 'Phosphor', cli: 'phosphor', appearance: 'dark', original: true, height: 964, palette: { bg: '#17130d', kw: '#ffd08a', fn: '#fff1d6', type: '#ffc266', num: '#ffd98f', comment: '#8f5f2a' } },
  { id: 'gruvbox', name: 'Gruvbox Dark', cli: 'gruvbox-medium', appearance: 'dark', height: 666, palette: { bg: '#282828', kw: '#fb4934', fn: '#b8bb26', type: '#fabd2f', num: '#d3869b', comment: '#928374' } },
  { id: 'ayu', name: 'Ayu Light', cli: 'ayu', appearance: 'light', height: 666, palette: { bg: '#fcfcfc', kw: '#fa8d3e', fn: '#f2ae49', type: '#399ee6', num: '#a37acc', comment: '#adaeb1' } },
  { id: 'nightowl', name: 'Night Owl', cli: 'night-owl', appearance: 'dark', height: 666, palette: { bg: '#011627', kw: '#c792ea', fn: '#82aaff', type: '#ffcb8b', num: '#f78c6c', comment: '#637777' } },
  { id: 'catppuccin', name: 'Catppuccin Mocha', cli: 'catppuccin', appearance: 'dark', height: 666, palette: { bg: '#1e1e2e', kw: '#cba6f7', fn: '#89b4fa', type: '#f9e2af', num: '#fab387', comment: '#7f849c' } },
  { id: 'solarized', name: 'Solarized Light', cli: 'solarized', appearance: 'light', height: 666, palette: { bg: '#fdf6e3', kw: '#859900', fn: '#268bd2', type: '#b58900', num: '#d33682', comment: '#93a1a1' } },
];
export const source = [
  'fn share(code: &str, theme: &Theme) -> Image {',
  '    let tokens = highlight(code, theme.syntax);',
  '    // Rendered offline. No network.',
  '    let canvas = Canvas::new(640, 64);',
  '    canvas.draw(tokens).export(Png { scale: 2 })',
  '}',
];
export const initialTheme = themes[themes.findIndex(theme => theme.id === 'nightowl')];
export function command(theme) {
  return `monoprint share.rs -o share.png --theme ${theme.cli}${theme.appearance === 'light' ? ' --light' : ''} --highlight 4`;
}
