import fs from 'node:fs/promises';
import release from '../../../release/config.json' with { type: 'json' };

// CI supplies this only after verifying published GitHub assets. Local previews
// stay useful without networking or pretending an unreleased build is available.
const file = process.env.MONOPRINT_DOWNLOAD_MANIFEST;
export const manifest = file ? JSON.parse(await fs.readFile(file, 'utf8')) : null;
if (manifest && (manifest.distribution !== true || manifest.repository !== release.repository)) {
  throw new Error('Website downloads require a verified distribution manifest');
}
export const downloads = manifest ? [
  { ...manifest.downloads['macos-arm64'], label: 'Download for macOS', detail: 'Apple Silicon · ZIP' },
  { ...manifest.downloads['win-x64'], label: 'Download for Windows', detail: '64-bit · Installer' },
  { ...manifest.downloads['linux-x64'], label: 'Download for Linux', detail: '64-bit · AppImage · Ubuntu 24.04+' },
] : [];
