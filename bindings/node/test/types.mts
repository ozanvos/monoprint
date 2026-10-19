import { Font, Renderer, png, svg, html, getThemes, languageForFileName, MonoprintError } from 'monoprint';
import type { Background, PngOptions, Style, ThemeId, LanguageId } from 'monoprint';
const theme: ThemeId = 'catppuccin-latte';
const background: Background = 'lattice';
const style = { theme, background, width: 'auto', lineNumbers: true, fontSize: 18 } satisfies Style;
const renderer = new Renderer(style);
const options = { language: 'typescript', scale: 2, compression: 'balanced', highlight: [1, [2, 3]] } as const satisfies PngOptions;
const p: Promise<Buffer> = renderer.png('code', options);
const s: Promise<string> = svg('code', { ...style, highlight: [1] });
const h: string = renderer.htmlSync('code');
const inferred: LanguageId | undefined = languageForFileName('main.ts');
const id: ThemeId = getThemes()[0]!.id;
const font: Font = Font.fromBytes(new Uint8Array());
html('code', { font });
// @ts-expect-error PNG scale is not an SVG option.
svg('code', { scale: 2 });
// @ts-expect-error Scale cannot exceed six.
png('code', { scale: 7 });
// @ts-expect-error Only the documented compression presets are accepted.
png('code', { compression: 'smallest' });
// @ts-expect-error Compression only applies to PNG output.
svg('code', { compression: 'fast' });
// @ts-expect-error Compression is an export option, not a visual style.
new Renderer({ compression: 'balanced' });
// @ts-expect-error Font is an opaque validated handle.
html('code', { font: {} });
// @ts-expect-error Unknown theme typo.
new Renderer({ theme: 'nuxtt' });
// @ts-expect-error Unsupported theme IDs are not part of the public catalog.
new Renderer({ theme: 'candy' });
// @ts-expect-error Unknown background treatment.
new Renderer({ background: 'sparkles' });
// @ts-expect-error Font size is numeric CSS pixels, not a CSS string.
new Renderer({ fontSize: '18px' });
// @ts-expect-error Content options do not belong to a reusable style.
new Renderer({ language: 'rust' });
if (new Error() instanceof MonoprintError) { const e = new MonoprintError('INVALID_FONT', 'bad font'); console.log(e.code, e.details.option); }
void [p, s, h, inferred, id];
