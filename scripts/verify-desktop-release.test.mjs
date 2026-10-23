import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verifyRelease } from './verify-desktop-release.mjs';

function fixture() {
  const repository = 'ozanvos/monoprint';
  const base = `https://github.com/${repository}/releases`;
  const names = { 'macos-arm64': 'Monoprint-0.1.0-aarch64-apple-darwin.zip', 'win-x64': 'Monoprint-win-Setup.exe', 'linux-x64': 'Monoprint-linux.AppImage' };
  const files = [...Object.values(names), 'appcast-macos-arm64.xml', 'releases.win-x64.json', 'releases.linux-x64.json'].map(file => ({ file, bytes: 100, sha256: 'a'.repeat(64) }));
  const manifest = { version: '0.1.0', tag: 'v0.1.0', commit: 'b'.repeat(40), repository, distribution: true, files,
    downloads: Object.fromEntries(Object.entries(names).map(([runtime, file]) => [runtime, { ...files.find(entry => entry.file === file), url: `${base}/download/v0.1.0/${file}` }])) };
  const release = { tag_name: manifest.tag, draft: false, prerelease: false, html_url: `${base}/tag/v0.1.0`,
    assets: [...files, { file: 'SHA256SUMS', bytes: 10 }, { file: 'provenance.sigstore.json', bytes: 10 }].map(file => ({ name: file.file, size: file.bytes, digest: `sha256:${file.sha256}`, state: 'uploaded', browser_download_url: `${base}/download/v0.1.0/${file.file}` })) };
  return { manifest, release, repository };
}

test('published complete release resolves all three platform downloads', () => {
  const { release, manifest, repository } = fixture();
  assert.equal(verifyRelease(release, manifest, repository, 'v0.1.0'), manifest);
});

for (const [name, mutate] of [
  ['draft release', f => f.release.draft = true],
  ['prerelease', f => f.release.prerelease = true],
  ['unsigned candidate', f => f.manifest.distribution = false],
  ['different repository', f => f.manifest.repository = 'other/repo'],
  ['missing installer', f => f.release.assets.shift()],
  ['changed bytes', f => f.release.assets[0].digest = 'sha256:' + '0'.repeat(64)],
  ['partial upload', f => f.release.assets[0].state = 'new'],
  ['missing signatures', f => f.release.assets.pop()],
  ['unpublished URL', f => f.manifest.downloads['linux-x64'].url = 'https://example.com/app'],
  ['missing feed', f => f.manifest.files = f.manifest.files.filter(file => file.file !== 'releases.win-x64.json')],
  ['mismatched version', f => f.manifest.version = '0.2.0'],
]) {
  test(`website rejects ${name}`, () => {
    const f = fixture();
    mutate(f);
    assert.throws(() => verifyRelease(f.release, f.manifest, f.repository));
  });
}

test('website rejects an unexpected latest tag', () => {
  const f = fixture();
  assert.throws(() => verifyRelease(f.release, f.manifest, f.repository, 'v0.2.0'));
});
