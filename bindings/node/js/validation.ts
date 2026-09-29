import { MonoprintError, limits } from './index.js';
import type { Font, PngOptions, RenderOptions, Style } from './index.js';
import type { NativeFont } from './native.js';

const styleKeys = ['theme', 'appearance', 'padding', 'fontSize', 'width', 'transparent', 'background', 'lineNumbers', 'font'];
const contentKeys = ['language', 'title', 'subtitle', 'highlight'];
function invalid(option: string, message: string): never {
  throw new MonoprintError('INVALID_ARGUMENT', `${option}: ${message}`, { option });
}
function object(value: unknown, allowed: readonly string[]): Record<string, unknown> {
  if (!value || typeof value !== 'object' || ![null, Object.prototype].includes(Object.getPrototypeOf(value))) invalid('options', 'must be a plain object');
  const copy: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    if (!allowed.includes(key)) invalid(key, 'unknown option');
    if (item !== undefined) copy[key] = item;
  }
  return copy;
}
function integer(value: unknown, option: string): void {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 0 || value > 0xffffffff) invalid(option, 'must be a nonnegative 32-bit integer');
}
function validate(value: Record<string, unknown>, fonts: WeakMap<Font, NativeFont>): void {
  for (const key of ['language', 'title', 'subtitle', 'theme']) {
    if (key in value && typeof value[key] !== 'string') invalid(key, 'must be a string');
  }
  if ('compression' in value && !['fast', 'balanced'].includes(value.compression as string)) invalid('compression', "must be 'fast' or 'balanced'");
  if ('appearance' in value && !['light', 'dark'].includes(value.appearance as string)) invalid('appearance', "must be 'light' or 'dark'");
  for (const key of ['padding', 'fontSize', 'scale']) if (key in value) integer(value[key], key);
  if ('width' in value && value.width !== 'auto') integer(value.width, 'width');
  if ('transparent' in value && typeof value.transparent !== 'boolean') invalid('transparent', 'must be boolean');
  if ('background' in value && !['theme', 'solid', 'lattice', 'halftone', 'none'].includes(value.background as string)) invalid('background', 'must be theme, solid, lattice, halftone, or none');
  if ('background' in value && 'transparent' in value) invalid('background', 'cannot be combined with transparent');
  if ('lineNumbers' in value && value.lineNumbers !== 'auto' && typeof value.lineNumbers !== 'boolean') invalid('lineNumbers', "must be true, false, or 'auto'");
  if ('font' in value && value.font !== null && (typeof value.font !== 'object' || !fonts.has(value.font as Font))) invalid('font', 'must be created with Font.fromBytes(), or null');
  if ('highlight' in value) {
    if (!Array.isArray(value.highlight)) invalid('highlight', 'must be an array of lines or [first, last] ranges');
    value.highlight = Array.from(value.highlight, selection => {
      if (Array.isArray(selection)) {
        if (selection.length !== 2) invalid('highlight', 'ranges require exactly two line numbers');
        for (let i = 0; i < 2; i++) integer(selection[i], 'highlight');
        return [...selection];
      }
      integer(selection, 'highlight');
      return selection;
    });
  }
}
export function snapshotStyle(style: Style, fonts: WeakMap<Font, NativeFont>): Readonly<Style> {
  const copy = object(style, styleKeys);
  validate(copy, fonts);
  return Object.freeze(copy) as Readonly<Style>;
}
export function normalize(format: 'png' | 'svg' | 'html', source: string, style: Readonly<Style>, options: PngOptions | RenderOptions, fonts: WeakMap<Font, NativeFont>) {
  if (typeof source !== 'string') invalid('source', 'must be a string');
  const bytes = Buffer.byteLength(source, 'utf8');
  if (bytes > limits.sourceBytes) throw new MonoprintError('INVALID_OPTION', 'source exceeds 1 MiB', { option: 'source', bytes, max: limits.sourceBytes });
  const overrides = object(options, [...styleKeys, ...contentKeys, ...(format === 'png' ? ['scale', 'compression'] : [])]);
  validate(overrides, fonts);
  const value = { ...style, ...overrides };
  // The modern option and its compatibility alias describe one setting.
  if ('background' in overrides) delete value.transparent;
  if ('transparent' in overrides) delete value.background;
  validate(value, fonts);
  const font = value.font ? fonts.get(value.font as Font) : undefined;
  delete value.font;
  if (value.width === 'auto') delete value.width;
  return { json: JSON.stringify({ ...value, format }), font };
}
