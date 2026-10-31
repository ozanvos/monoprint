// Token colours for the live sample, one per theme in data/demo.mjs, taken from each theme's published palette.
export const palettes = {
  gruvbox: { bg: '#282828', fg: '#ebdbb2', punct: '#a89984', kw: '#fb4934', fn: '#b8bb26', type: '#fabd2f', num: '#d3869b', comment: '#928374', gutter: '#7c6f64', line: '#3c3836', chrome: '#504945' },
  ayu: { bg: '#fcfcfc', fg: '#5c6166', punct: '#8a9199', kw: '#fa8d3e', fn: '#f2ae49', type: '#399ee6', num: '#a37acc', comment: '#adaeb1', gutter: '#c5c7ca', line: '#f0f1f2', chrome: '#e7e8e9' },
  nightowl: { bg: '#011627', fg: '#d6deeb', punct: '#7fdbca', kw: '#c792ea', fn: '#82aaff', type: '#ffcb8b', num: '#f78c6c', comment: '#637777', gutter: '#4b6479', line: '#0b2942', chrome: '#1d3b53' },
  catppuccin: { bg: '#1e1e2e', fg: '#cdd6f4', punct: '#9399b2', kw: '#cba6f7', fn: '#89b4fa', type: '#f9e2af', num: '#fab387', comment: '#7f849c', gutter: '#585b70', line: '#313244', chrome: '#45475a' },
  solarized: { bg: '#fdf6e3', fg: '#657b83', punct: '#839496', kw: '#859900', fn: '#268bd2', type: '#b58900', num: '#d33682', comment: '#93a1a1', gutter: '#93a1a1', line: '#eee8d5', chrome: '#e4ddc8' },
};

// share.rs from data/demo.mjs, split into [class, text] tokens.
export const tokens = [
  [['kw', 'fn'], ['', ' '], ['fn', 'share'], ['punct', '('], ['', 'code'], ['punct', ': &'], ['type', 'str'], ['punct', ', '], ['', 'theme'], ['punct', ': &'], ['type', 'Theme'], ['punct', ') -> '], ['type', 'Image'], ['punct', ' {']],
  [['', '    '], ['kw', 'let'], ['', ' tokens '], ['punct', '= '], ['fn', 'highlight'], ['punct', '('], ['', 'code'], ['punct', ', '], ['', 'theme'], ['punct', '.'], ['', 'syntax'], ['punct', ');']],
  [['', '    '], ['comment', '// Rendered on this Mac. No network.']],
  [['', '    '], ['kw', 'let'], ['', ' canvas '], ['punct', '= '], ['type', 'Canvas'], ['punct', '::'], ['fn', 'new'], ['punct', '('], ['num', '640'], ['punct', ', '], ['num', '64'], ['punct', ');']],
  [['', '    canvas'], ['punct', '.'], ['fn', 'draw'], ['punct', '('], ['', 'tokens'], ['punct', ').'], ['fn', 'export'], ['punct', '('], ['type', 'Png'], ['punct', ' { '], ['', 'scale'], ['punct', ': '], ['num', '2'], ['punct', ' })']],
  [['punct', '}']],
];
