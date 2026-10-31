// Sources and CLI settings for the three real exports shown on the homepage.
// Exports are rendered at `exportWidth` points and 2x scale.
export const exportWidth = 480;
export const showcases = [
  {
    id: 'typescript-monoprint', source: 'user.ts', theme: 'monoprint', appearance: 'dark', format: 'png',
    name: 'Pick your theme', detail: '69 themes',
    alt: 'A typed TypeScript API request in the dark Monoprint style, with warm syntax accents',
  },
  {
    id: 'python-highlights', source: 'total.py', theme: 'ayu', appearance: 'light', format: 'png', highlight: '7,8',
    name: 'Draw attention to a line', detail: 'Line highlights',
    alt: 'A Python shopping cart total in Ayu Light, with the discount calculation and return lines highlighted',
  },
  {
    id: 'rust-transparent', source: 'slug.rs', theme: 'github-default', appearance: 'light', format: 'svg', transparent: true,
    name: 'Fit it into your docs', detail: 'Transparent SVG',
    alt: 'A Rust function that turns a title into a URL slug, exported as SVG with a transparent backdrop',
  },
];
