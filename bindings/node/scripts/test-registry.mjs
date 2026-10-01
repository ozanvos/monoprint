// Public-release follow-up only; no publishing. Not run during private prep.
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { pkg, run } from './shared.mjs';

const folder = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-registry-'));
try {
  await fs.writeFile(path.join(folder, 'package.json'), JSON.stringify({ name: 'monoprint-registry-check', private: true, type: 'module' }));
  const npmCli = process.env.npm_execpath;
  const command = npmCli ? process.execPath : process.platform === 'win32' ? 'npm.cmd' : 'npm';
  run(command, [...(npmCli ? [npmCli] : []), 'install', `${pkg.name}@${pkg.version}`, '--ignore-scripts', '--no-audit', '--no-fund', '--registry', 'https://registry.npmjs.org'], { cwd: folder });
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
