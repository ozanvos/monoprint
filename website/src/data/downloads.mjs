import fs from 'node:fs/promises';
import release from '../../../release/config.json' with { type: 'json' };

// CI supplies this only after verifying published GitHub assets. Local previews
// stay useful without networking or pretending an unreleased build is available.
const file = process.env.MONOPRINT_DOWNLOAD_MANIFEST;
export const manifest = file ? JSON.parse(await fs.readFile(file, 'utf8')) : null;
if (manifest && (manifest.distribution !== true || manifest.repository !== release.repository)) {
  throw new Error('Website downloads require a verified distribution manifest');
}
// One terminal command per platform that installs the published app; shown with the downloads.
export const installs = manifest ? [
  { platform: 'macOS', command: 'brew install --cask ozanvos/tap/monoprint', copy: true },
  // Download-and-run commands written to the clipboard by a script look like ClickFix
  // malware to content blockers (uBlock Origin blocks the copy), so this one is select-only.
  ...[['linux-x64', 'Linux x64'], ['linux-arm64', 'Linux ARM64']].flatMap(([runtime, platform]) => {
    const download = manifest.downloads[runtime];
    return download ? [{ platform, copy: false, command: `curl -fLO ${download.url} && chmod +x ${download.file} && ./${download.file}` }] : [];
  }),
] : [];
export const downloads = manifest ? [
  { ...manifest.downloads['macos-arm64'], label: 'Download for macOS', detail: 'Apple Silicon · ZIP' },
  ...(manifest.downloads['win-x64'] ? [{ ...manifest.downloads['win-x64'], label: 'Download for Windows', detail: '64-bit · Installer' }] : []),
  { ...manifest.downloads['linux-x64'], label: 'Download for Linux x64', detail: 'Intel / AMD · AppImage · Ubuntu 24.04+' },
  ...(manifest.downloads['linux-arm64'] ? [{ ...manifest.downloads['linux-arm64'], label: 'Download for Linux ARM64', detail: 'ARM64 · AppImage · Ubuntu 24.04+' }] : []),
] : [];
