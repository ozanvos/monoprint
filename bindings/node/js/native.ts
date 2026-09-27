import type { LanguageId } from './catalog-types.js';
export interface NativeFont { readonly __nativeFont?: never }
export interface NativeResult { data?: Buffer | string | null; failure?: string | null }
export interface NativeRenderer {
  render(source: string, options: string, font?: NativeFont): Promise<NativeResult>;
  renderSync(source: string, options: string, font?: NativeFont): NativeResult;
}
export interface NativeBinding {
  NativeRenderer: new () => NativeRenderer;
  createFont(bytes: Uint8Array): { font?: NativeFont | null; failure?: string | null };
  catalog(): string;
  languageForFileName(filename: string): LanguageId | null;
}
