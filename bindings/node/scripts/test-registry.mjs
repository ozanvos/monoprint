// Public-release follow-up only; no publishing. Not run during private prep.
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import { pkg, run } from './shared.mjs';

const folder = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-registry-'));
try {
  await fs.writeFile(path.join(folder, 'package.json'), JSON.stringify({ name: 'monoprint-registry-check', private: true, type: 'module' }));
  const npmCli = process.env.npm_execpath;
  const command = npmCli ? process.execPath : process.platform === 'win32' ? 'npm.cmd' : 'npm';
  run(command, [...(npmCli ? [npmCli] : []), 'install', `${pkg.name}@${pkg.version}`, '--ignore-scripts', '--no-audit', '--no-fund', '--registry', 'https://registry.npmjs.org'], { cwd: folder });
  const version = run(command, [...(npmCli ? [npmCli] : []), 'exec', '--offline', '--', `${pkg.name}@${pkg.version}`, '--version'], { cwd: folder, stdio: 'pipe' }).toString().trim();
  assert.equal(version, `monoprint ${pkg.version}`);
  const help = run(command, [...(npmCli ? [npmCli] : []), 'exec', '--offline', '--', `${pkg.name}@${pkg.version}`], { cwd: folder, stdio: 'pipe' }).toString();
  assert.match(help, /Usage: monoprint/);
  const launcher = path.join(folder, 'node_modules', pkg.name, pkg.bin.monoprint);
  const image = run(process.execPath, [launcher, '-', '--language', 'ts', '--scale', '1', '--format', 'png', '-o', '-'],
    { cwd: folder, env: { ...process.env, PATH: '' }, input: 'const n = 42;\n', stdio: ['pipe', 'pipe', 'pipe'] });
  assert.equal(image.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  await fs.writeFile(path.join(folder, 'check.mjs'), `import assert from 'node:assert/strict';
import { png, svg, html } from ${JSON.stringify(pkg.name)};
assert.equal((await png('const n = 42;', {language:'ts',scale:1}))[0],137);
assert.match(await svg('你好 👩🏽‍💻'), /<svg/);
assert.match(await html('hello'), /<!doctype html>/);
`);
  run(process.execPath, ['check.mjs'], { cwd: folder, env: { ...process.env, PATH: '' } });
} finally {
  await fs.rm(folder, { recursive: true, force: true });
}
