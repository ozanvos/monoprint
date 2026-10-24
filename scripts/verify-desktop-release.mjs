// Resolve public website links only from a complete, published desktop release.
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import config from '../release/config.json' with { type: 'json' };

export function verifyRelease(release, manifest, repository, expectedTag) {
  const base = `https://github.com/${repository}/releases`;
  if (release.draft !== false || release.prerelease !== false
      || release.html_url !== `${base}/tag/${release.tag_name}`
      || (expectedTag && release.tag_name !== expectedTag)
      || manifest.repository !== repository || manifest.distribution !== true
      || !/^\d+\.\d+\.\d+$/.test(manifest.version)
      || release.tag_name !== `v${manifest.version}` || manifest.tag !== release.tag_name
      || !/^[a-f0-9]{40}$/.test(manifest.commit)) {
    throw new Error('Expected a published stable desktop release from the configured repository');
  }
  const assets = new Map(release.assets.map(asset => [asset.name, asset]));
  const files = new Map();
  for (const file of manifest.files ?? []) {
    const asset = assets.get(file.file);
    if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(file.file) || files.has(file.file)
        || !/^[a-f0-9]{64}$/.test(file.sha256) || !Number.isSafeInteger(file.bytes) || file.bytes < 1
        || !asset || asset.state !== 'uploaded' || asset.size !== file.bytes
        || asset.digest !== `sha256:${file.sha256}`
        || asset.browser_download_url !== `${base}/download/${manifest.tag}/${file.file}`) {
      throw new Error(`Published asset does not match release manifest: ${file.file}`);
    }
    files.set(file.file, file);
  }
  const formats = {
    'macos-arm64': ['-aarch64-apple-darwin.zip', 'appcast-macos-arm64.xml'],
    'win-x64': ['Setup.exe', 'releases.win-x64.json'],
    'linux-x64': ['-linux-x64.AppImage', 'releases.linux-x64.json'],
    'linux-arm64': ['-linux-arm64.AppImage', 'releases.linux-arm64.json'],
  };
  const platforms = Object.keys(manifest.downloads ?? {});
  if (!platforms.length || platforms.some(runtime => !formats[runtime])
      || config.desktopPlatforms.some(runtime => !platforms.includes(runtime))
      || (manifest.platforms && (new Set(manifest.platforms).size !== platforms.length
        || manifest.platforms.length !== platforms.length
        || manifest.platforms.some(runtime => !platforms.includes(runtime))))) {
    throw new Error('Missing or unsupported release platform');
  }
  if (!assets.get('provenance.sigstore.json')?.size || !assets.get('SHA256SUMS')?.size) {
    throw new Error('Missing release signatures or checksums');
  }
  for (const runtime of platforms) {
    const [suffix, feed] = formats[runtime];
    if (!files.has(feed)) throw new Error(`Missing update feed: ${feed}`);
    const download = manifest.downloads?.[runtime];
    const file = files.get(download?.file);
    if (!file || !download.file.endsWith(suffix) || file.sha256 !== download.sha256
        || file.bytes !== download.bytes || download.url !== assets.get(download.file).browser_download_url) {
      throw new Error(`Missing or inconsistent download: ${runtime}`);
    }
  }
  return manifest;
}

async function read(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(30_000), headers: { 'User-Agent': 'monoprint-release' } });
  if (!response.ok) throw new Error(`Release lookup failed: ${response.status} ${url}`);
  return response.json();
}

export async function fetchRelease(repository, expectedTag) {
  const release = await read(`https://api.github.com/repos/${repository}/releases/latest`);
  const asset = release.assets.find(asset => asset.name === 'release.json');
  const expected = `https://github.com/${repository}/releases/download/${release.tag_name}/release.json`;
  if (!asset || asset.browser_download_url !== expected) throw new Error('Latest release has no desktop manifest');
  return verifyRelease(release, await read(expected), repository, expectedTag);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (![2, 4].includes(args.length) || args[0] !== '--out' || (args.length === 4 && args[2] !== '--tag')) {
    throw new Error('Usage: verify-desktop-release.mjs --out FILE [--tag vVERSION]');
  }
  const manifest = await fetchRelease(config.repository, args[3]);
  await fs.mkdir(path.dirname(path.resolve(args[1])), { recursive: true });
  await fs.writeFile(args[1], JSON.stringify(manifest, null, 2) + '\n');
  console.log(`Verified public downloads for ${manifest.tag}`);
}
