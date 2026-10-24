import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { pkg, targets, cliFile, run, revision, inputHash, sha256 } from '../bindings/node/scripts/shared.mjs';
import { validatePackages } from '../bindings/node/scripts/release-validation.mjs';

async function fixture(check) {
  const source = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-receipts-'));
  try {
    await fs.mkdir(path.join(source, 'tarballs'));
    const hash = await inputHash();
    const commit = revision();
    const manifest = { name: pkg.name, version: pkg.version, release: true, dirty: false, commit, inputHash: hash, targets: [], tarballs: [] };
    for (const target of [...targets, null]) {
      const id = target?.id ?? 'main';
      const name = target ? `${pkg.name}-${id}` : pkg.name;
      const folder = path.join(source, id);
      const content = path.join(folder, 'package');
      await fs.mkdir(content, { recursive: true });
      const metadata = { name, version: pkg.version, repository: pkg.repository,
        ...(!target ? { bin: pkg.bin, optionalDependencies: Object.fromEntries(targets.map(t => [`${pkg.name}-${t.id}`, pkg.version])) } : {}) };
      await fs.writeFile(path.join(content, 'package.json'), JSON.stringify(metadata));
      if (target) {
        const native = Buffer.from(`test fixture ${id}`);
        await fs.writeFile(path.join(content, `monoprint.${id}.node`), native);
        const cli = Buffer.from(`test CLI fixture ${id}`);
        await fs.writeFile(path.join(content, cliFile(target)), cli, { mode: 0o755 });
        manifest.targets.push({ id, target: id, triple: target.triple, version: pkg.version, commit, inputHash: hash, dirty: false, sha256: sha256(native), cliSha256: sha256(cli) });
      } else {
        await fs.mkdir(path.join(content, 'dist'));
        await fs.copyFile(new URL('../bindings/node/js/cli.cjs', import.meta.url), path.join(content, pkg.bin.monoprint));
      }
      const file = `tarballs/${id}.tgz`;
      run('tar', ['-czf', path.join(source, file), '-C', folder, 'package']);
      manifest.tarballs.push({ name, version: pkg.version, file, sha256: sha256(await fs.readFile(path.join(source, file))) });
    }
    await check(source, manifest);
  } finally { await fs.rm(source, { recursive: true, force: true }); }
}

test('complete source-bound tarball set validates offline', () => fixture(async (source, manifest) => {
  await validatePackages(source, manifest);
}));
test('mixed native revisions and local subsets are rejected', () => fixture(async (source, manifest) => {
  manifest.targets[0].commit = '0'.repeat(40);
  await assert.rejects(validatePackages(source, manifest), /complete release package set/);
  manifest.targets[0].commit = manifest.commit;
  manifest.release = false;
  await assert.rejects(validatePackages(source, manifest), /complete release package set/);
}));
test('altered archive bytes are rejected', () => fixture(async (source, manifest) => {
  await fs.appendFile(path.join(source, manifest.tarballs[0].file), 'changed');
  await assert.rejects(validatePackages(source, manifest), /Tarball changed/);
}));
test('self-consistent archive hash cannot hide changed packed metadata', () => fixture(async (source, manifest) => {
  const entry = manifest.tarballs[0];
  const folder = path.join(source, targets[0].id);
  await fs.writeFile(path.join(folder, 'package/package.json'), JSON.stringify({ name: 'unexpected', version: pkg.version }));
  run('tar', ['-czf', path.join(source, entry.file), '-C', folder, 'package']);
  entry.sha256 = sha256(await fs.readFile(path.join(source, entry.file)));
  await assert.rejects(validatePackages(source, manifest), /Packed metadata mismatch/);
}));
test('changed native binary must agree with the independent build receipt', () => fixture(async (source, manifest) => {
  const entry = manifest.tarballs[0];
  const folder = path.join(source, targets[0].id);
  await fs.writeFile(path.join(folder, `package/monoprint.${targets[0].id}.node`), 'changed binary');
  run('tar', ['-czf', path.join(source, entry.file), '-C', folder, 'package']);
  entry.sha256 = sha256(await fs.readFile(path.join(source, entry.file)));
  await assert.rejects(validatePackages(source, manifest), /Packed native binary/);
}));
test('missing npm executable metadata is rejected', () => fixture(async (source, manifest) => {
  const entry = manifest.tarballs.at(-1);
  const folder = path.join(source, 'main');
  const file = path.join(folder, 'package/package.json');
  const metadata = JSON.parse(await fs.readFile(file, 'utf8'));
  delete metadata.bin;
  await fs.writeFile(file, JSON.stringify(metadata));
  run('tar', ['-czf', path.join(source, entry.file), '-C', folder, 'package']);
  entry.sha256 = sha256(await fs.readFile(path.join(source, entry.file)));
  await assert.rejects(validatePackages(source, manifest), /Packed CLI bin entry/);
}));
test('changed CLI binary must agree with the independent build receipt', () => fixture(async (source, manifest) => {
  const entry = manifest.tarballs[0];
  const folder = path.join(source, targets[0].id);
  await fs.writeFile(path.join(folder, 'package', cliFile(targets[0])), 'changed CLI');
  run('tar', ['-czf', path.join(source, entry.file), '-C', folder, 'package']);
  entry.sha256 = sha256(await fs.readFile(path.join(source, entry.file)));
  await assert.rejects(validatePackages(source, manifest), /Packed CLI binary/);
}));
