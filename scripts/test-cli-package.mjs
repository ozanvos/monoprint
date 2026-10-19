import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { run, sha256, readJson } from '../bindings/node/scripts/shared.mjs';

const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== '--from') throw new Error('Usage: node scripts/test-cli-package.mjs --from DIRECTORY');
const source = path.resolve(args[1]);
const manifest = await readJson(path.join(source, 'manifest.json'));
assert.equal(path.basename(manifest.archive), manifest.archive);
const archive = path.join(source, manifest.archive);
assert.equal(sha256(await fs.readFile(archive)), manifest.archiveSha256);
const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-cli-consumer-'));
try {
  const names = run('tar', ['-tzf', archive], { stdio: 'pipe' }).toString().trim().split('\n');
  const prefix = names[0].split('/')[0];
  assert.ok(names.every(n => n.startsWith(`${prefix}/`) && !n.split('/').includes('..')));
  run('tar', ['-xzf', archive, '-C', temporary]);
  const folder = path.join(temporary, prefix);
  const receipt = await readJson(path.join(folder, 'build.json'));
  const binary = path.join(folder, 'monoprint');
  assert.equal(receipt.commit, manifest.commit);
  assert.equal(receipt.inputHash, manifest.inputHash);
  assert.equal(sha256(await fs.readFile(binary)), manifest.binarySha256);
  for (const file of ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'licenses/rust-dependencies.json',
    'licenses/rust-dependency-notices.txt', 'licenses/sources/giallo-0.5.2.crate',
    'licenses/grammar-sources/hcl.json', 'assets/fonts/fallback/NOTICE.txt']) await fs.access(path.join(folder, file));
  const relocated = path.join(temporary, 'relocated');
  await fs.rename(folder, relocated);
  const executable = path.join(relocated, 'monoprint');
  const snippet = path.join(temporary, 'example.ts');
  await fs.writeFile(snippet, 'const message = "你好 👩🏽‍💻 🇳🇱";\n');
  for (const format of ['png', 'svg', 'html']) {
    const file = path.join(temporary, `example.${format}`);
    run(executable, [snippet, '--theme', 'monoprint', '--light', '--background', 'halftone', '--scale', '1', '-o', file], { env: { ...process.env, PATH: '' } });
    const bytes = await fs.readFile(file);
    if (format === 'png') assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
    if (format === 'svg') { assert.match(bytes.toString(), /<svg/); assert.doesNotMatch(bytes.toString(), /<foreignObject|<text[ >]/); }
    if (format === 'html') assert.match(bytes.toString(), /你好/);
  }
  console.log('CLI archive checksums, notices and relocated offline PNG/SVG/HTML exports passed.');
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
