import { loadBinding } from './loader.cjs';
import type { Buffer } from 'node:buffer';
import { normalize, snapshotStyle } from './validation.js';
import type { NativeBinding, NativeFont, NativeRenderer, NativeResult } from './native.js';
import type { ThemeId, LanguageId } from './catalog-types.js';
export type { ThemeId, LanguageId } from './catalog-types.js';

/** PNG output pixels per CSS pixel. Defaults to 4; SVG/HTML use CSS dimensions. */
export type Scale = 1 | 2 | 3 | 4 | 5 | 6;
export type Appearance = 'dark' | 'light';
/** A one-based line number or an inclusive [first, last] pair. */
export type Highlight = number | readonly [number, number];

/** Reusable appearance. Values omitted from a render inherit renderer defaults. */
export interface Style {
  /** Candy by default. IDs and capabilities are discoverable with `getThemes()`. */
  theme?: ThemeId;
  /** Dark by default. Stripe has no light variant. */
  appearance?: Appearance;
  /** Outer CSS pixels, 0–512. Defaults to 64. */
  padding?: number;
  /** Fixed outer CSS width, or 'auto' to fit with a 520px minimum. */
  width?: number | 'auto';
  /** Remove the decorative background, retaining the code window. */
  transparent?: boolean;
  /** Theme defaults and fixed partner-theme rules apply; plain text hides numbers. */
  lineNumbers?: boolean | 'auto';
  /** Reusable TTF/OTF code font. `null` restores the theme font. */
  font?: Font | null;
}

/** Content metadata for a render. The source string is always the first argument. */
export interface SnippetOptions {
  /** Language ID or alias, e.g. 'typescript', 'rs', 'py'. Defaults to plain text. */
  language?: LanguageId | (string & {});
  /** Frame label only; never used to infer the language. */
  title?: string;
  /** Auth0 metadata only; not displayed by its pinned frame. Other themes reject it. */
  subtitle?: string;
  /** One-based lines/ranges. Validated against normalized source lines. */
  highlight?: readonly Highlight[];
}

export interface RenderOptions extends Style, SnippetOptions {}
export interface PngOptions extends RenderOptions { scale?: Scale }

export interface ThemeInfo {
  readonly id: ThemeId;
  readonly name: string;
  readonly appearances: readonly Appearance[];
  readonly lineNumbers: { readonly toggleable: boolean; readonly default: boolean };
}
export interface LanguageInfo {
  readonly id: LanguageId;
  readonly name: string;
  readonly aliases: readonly string[];
}

export type ErrorCode = 'INVALID_ARGUMENT' | 'INVALID_OPTION' | 'UNKNOWN_THEME'
  | 'UNKNOWN_LANGUAGE' | 'INVALID_FONT' | 'UNSUPPORTED_GLYPH' | 'IMAGE_TOO_LARGE'
  | 'RENDER_FAILED' | 'NATIVE_BINDING_UNAVAILABLE';

/** Stable machine-readable fields; error messages remain human-facing. */
export interface ErrorDetails {
  readonly option?: string;
  readonly id?: string;
  readonly value?: number;
  readonly min?: number;
  readonly max?: number;
  readonly bytes?: number;
  readonly theme?: string;
  readonly appearance?: Appearance;
  readonly start?: number;
  readonly end?: number;
  readonly lineCount?: number;
  readonly text?: 'code' | 'title';
  readonly character?: string;
  readonly codeFont?: boolean;
  readonly width?: number;
  readonly height?: number;
  readonly supersampled?: boolean;
}

/** Catch this class and inspect `code` and `details`; never parse message text. */
export class MonoprintError extends Error {
  readonly code: ErrorCode;
  readonly details: Readonly<ErrorDetails>;
  constructor(code: ErrorCode, message: string, details: ErrorDetails = {}, options?: ErrorOptions) {
    super(message, options);
    this.name = 'MonoprintError';
    this.code = code;
    this.details = Object.freeze({ ...details });
  }
}

let binding: NativeBinding | undefined;
function native(): NativeBinding {
  try { return binding ??= loadBinding(); }
  catch (cause) {
    throw new MonoprintError('NATIVE_BINDING_UNAVAILABLE',
      'Cannot load the Monoprint native addon. Install optional dependencies for the runtime OS/CPU and keep monoprint external in server bundles.', {}, { cause });
  }
}
function failure(encoded: string): MonoprintError {
  const { code, message, details } = JSON.parse(encoded) as { code: ErrorCode; message: string; details: ErrorDetails };
  return new MonoprintError(code, message, details);
}
function unwrap(result: NativeResult): Buffer | string {
  if (result.failure) throw failure(result.failure);
  if (result.data === undefined || result.data === null) throw new MonoprintError('RENDER_FAILED', 'Native renderer returned no output');
  return result.data;
}

const fonts = new WeakMap<Font, NativeFont>();
/** An immutable, reusable custom code font. Its bytes are copied once on creation. */
export class Font {
  declare private readonly __fontBrand: void;
  private constructor() {}
  /** Accepts a Buffer or Uint8Array containing one TTF/OTF face, at most 64 MiB. */
  static fromBytes(bytes: Uint8Array): Font {
    if (!(bytes instanceof Uint8Array) || bytes.buffer instanceof SharedArrayBuffer) {
      throw new MonoprintError('INVALID_ARGUMENT', 'font bytes must be a non-shared Uint8Array or Buffer', { option: 'font' });
    }
    if (bytes.byteLength > limits.fontBytes) {
      throw new MonoprintError('INVALID_OPTION', 'font exceeds 64 MiB', { option: 'font', bytes: bytes.byteLength, max: limits.fontBytes });
    }
    const result = native().createFont(bytes);
    if (result.failure) throw failure(result.failure);
    if (!result.font) throw new MonoprintError('RENDER_FAILED', 'Native font creation returned no font');
    const font = new Font();
    fonts.set(font, result.font);
    Object.freeze(font);
    return font;
  }
}

/** Documented pre-normalization input limits. Native image limits also apply. */
export const limits = Object.freeze({ sourceBytes: 1_048_576, fontBytes: 64 * 1024 * 1024 });

/**
 * Reuses native grammar/font caches. Async methods run CPU work on libuv's pool;
 * sync methods block the calling thread. Inputs/defaults are snapshotted at call time.
 * Rendering cannot be interrupted once started; coalesce preview edits before dispatch.
 */
export class Renderer {
  #native: NativeRenderer | undefined;
  #style: Readonly<Style>;
  constructor(style: Style = {}) { this.#style = snapshotStyle(style, fonts); }

  #request(format: 'png' | 'svg' | 'html', source: string, options: PngOptions | RenderOptions) {
    const { json, font } = normalize(format, source, this.#style, options, fonts);
    const renderer = this.#native ??= new (native().NativeRenderer)();
    return { renderer, json, font };
  }
  async #render(format: 'png' | 'svg' | 'html', source: string, options: PngOptions | RenderOptions): Promise<Buffer | string> {
    const { renderer, json, font } = this.#request(format, source, options);
    return unwrap(await renderer.render(source, json, font));
  }
  #renderSync(format: 'png' | 'svg' | 'html', source: string, options: PngOptions | RenderOptions): Buffer | string {
    const { renderer, json, font } = this.#request(format, source, options);
    return unwrap(renderer.renderSync(source, json, font));
  }

  /** Lossless PNG. Defaults to 4×; returns an owned Node Buffer. */
  png(source: string, options: PngOptions = {}): Promise<Buffer> { return this.#render('png', source, options) as Promise<Buffer>; }
  /** Portable SVG with outlined text and embedded textures. */
  svg(source: string, options: RenderOptions = {}): Promise<string> { return this.#render('svg', source, options) as Promise<string>; }
  /** Standalone HTML with selectable text, embedded fonts/images, and no scripts. */
  html(source: string, options: RenderOptions = {}): Promise<string> { return this.#render('html', source, options) as Promise<string>; }
  /** Synchronous PNG; blocks the calling thread. */
  pngSync(source: string, options: PngOptions = {}): Buffer { return this.#renderSync('png', source, options) as Buffer; }
  /** Synchronous SVG; blocks the calling thread. */
  svgSync(source: string, options: RenderOptions = {}): string { return this.#renderSync('svg', source, options) as string; }
  /** Synchronous HTML; blocks the calling thread. */
  htmlSync(source: string, options: RenderOptions = {}): string { return this.#renderSync('html', source, options) as string; }
}

// Convenience calls share the same lazy renderer; constructing Renderer creates an independent cache.
const shared = new Renderer();
/** Render PNG using the shared renderer. */
export const png = (source: string, options?: PngOptions): Promise<Buffer> => shared.png(source, options);
/** Render SVG using the shared renderer. */
export const svg = (source: string, options?: RenderOptions): Promise<string> => shared.svg(source, options);
/** Render HTML using the shared renderer. */
export const html = (source: string, options?: RenderOptions): Promise<string> => shared.html(source, options);
/** Blocking counterpart of png. */
export const pngSync = (source: string, options?: PngOptions): Buffer => shared.pngSync(source, options);
/** Blocking counterpart of svg. */
export const svgSync = (source: string, options?: RenderOptions): string => shared.svgSync(source, options);
/** Blocking counterpart of html. */
export const htmlSync = (source: string, options?: RenderOptions): string => shared.htmlSync(source, options);

let cachedThemes: readonly ThemeInfo[] | undefined;
let cachedLanguages: readonly LanguageInfo[] | undefined;
function readCatalog(): void {
  if (cachedThemes) return;
  const catalog = JSON.parse(native().catalog()) as { themes: ThemeInfo[]; languages: LanguageInfo[] };
  cachedThemes = Object.freeze(catalog.themes.map(t => Object.freeze({ ...t,
    appearances: Object.freeze(t.appearances), lineNumbers: Object.freeze(t.lineNumbers) })));
  cachedLanguages = Object.freeze(catalog.languages.map(l => Object.freeze({ ...l, aliases: Object.freeze(l.aliases) })));
}
/** All 33 themes and their appearance/line-number capabilities, without initializing rendering. */
export function getThemes(): readonly ThemeInfo[] { readCatalog(); return cachedThemes!; }
/** All selectable highlighting languages plus plain text, with aliases. */
export function getLanguages(): readonly LanguageInfo[] { readCatalog(); return cachedLanguages!; }
/** Infer a canonical language ID from a filename without reading a file. */
export function languageForFileName(filename: string): LanguageId | undefined {
  if (typeof filename !== 'string') throw new MonoprintError('INVALID_ARGUMENT', 'filename must be a string');
  return native().languageForFileName(filename) ?? undefined;
}
