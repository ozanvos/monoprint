import fs from 'node:fs/promises';
import path from 'node:path';
import { root, pkg, run, readJson } from '../bindings/node/scripts/shared.mjs';

export async function checkRelease({ publicPublish = false, tag } = {}) {
  const config = await readJson(path.join(root, 'release/config.json'));
  if (!/^[\w.-]+\/[\w.-]+$/.test(config.repository ?? '')) throw new Error('Set release/config.json repository to OWNER/REPO');
  if (!Array.isArray(config.desktopPlatforms) || !config.desktopPlatforms.length
      || new Set(config.desktopPlatforms).size !== config.desktopPlatforms.length
      || config.desktopPlatforms.some(runtime => !['macos-arm64', 'linux-x64', 'win-x64'].includes(runtime))) {
    throw new Error('Configure the desktop release platforms explicitly');
  }
  const url = `https://github.com/${config.repository}`;
  for (const manifest of ['Cargo.toml', 'bindings/node/Cargo.toml', 'apps/monoprint-gui/Cargo.toml']) {
    const text = await fs.readFile(path.join(root, manifest), 'utf8');
    if (!text.includes(`version = "${pkg.version}"`) || !text.includes(`repository = "${url}"`)) throw new Error(`Version/repository mismatch in ${manifest}`);
  }
  if (pkg.repository?.url !== `git+${url}.git`) throw new Error('npm repository metadata must match the release repository');
  const lock = await readJson(path.join(root, 'bindings/node/package-lock.json'));
  if (lock.version !== pkg.version || lock.packages[''].version !== pkg.version) throw new Error('npm lock version mismatch');
  await fs.access(path.join(root, `release/notes/${pkg.version}.md`));
  if (tag) {
    if (tag !== `v${pkg.version}`) throw new Error(`Expected tag v${pkg.version}`);
    const tagged = run('git', ['rev-parse', `${tag}^{commit}`], { stdio: 'pipe' }).toString().trim();
    const head = run('git', ['rev-parse', 'HEAD'], { stdio: 'pipe' }).toString().trim();
    if (tagged !== head) throw new Error('Release tag does not point to the checked-out source');
  }
  if (publicPublish) assertPublicAllowed(config, process.env);
  return { ...config, version: pkg.version };
}

export function assertPublicAllowed(config, environment) {
  if (config.publicPublishing !== true || environment.MONOPRINT_PUBLIC_PUBLISH !== 'approved') {
    throw new Error('Public publishing is disabled. Enable release/config.json and explicitly authorize this invocation.');
  }
  if (environment.GITHUB_REPOSITORY && environment.GITHUB_REPOSITORY !== config.repository) throw new Error('Publishing repository mismatch');
  if (environment.GITHUB_REPOSITORY_VISIBILITY === 'private') throw new Error('Public npm provenance requires an approved public repository');
}
