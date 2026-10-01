// Local preparation only. This script never creates tags, releases or uploads.
import fs from 'node:fs/promises';
import path from 'node:path';
import { root, run, revision, isDirty, readJson, sha256, inputHash } from '../bindings/node/scripts/shared.mjs';
import { checkRelease } from './release-config.mjs';

const [command, ...args] = process.argv.slice(2);
const option = name => args.includes(name) ? args[args.indexOf(name) + 1] : undefined;
const allowed = command === 'check' ? ['--public', '--tag'] : command === 'snapshot' ? ['--out'] : command === 'assemble' ? ['--from', '--out', '--distribution'] : [];
for (let i = 0; i < args.length; i++) {
  if (!allowed.includes(args[i])) throw new Error(`Unknown release option ${args[i]}`);
  if (['--from', '--out', '--tag'].includes(args[i]) && (!args[++i] || args[i].startsWith('--'))) throw new Error('Missing release option value');
}
const config = await checkRelease({ publicPublish: args.includes('--public'), tag: option('--tag') });
if (command === 'check') {
  console.log(`Monoprint ${config.version}; repository ${config.repository}; public publishing ${config.publicPublishing ? 'enabled' : 'disabled'}`);
} else if (command === 'snapshot') {
  if (!option('--out')) throw new Error('Usage: release.mjs snapshot --out FRESH_DIRECTORY');
  if (isDirty()) throw new Error('Commit the intended source before exporting a source snapshot');
  const out = path.resolve(option('--out'));
  await fs.mkdir(path.dirname(out), { recursive: true });
  await fs.mkdir(out); // refuse overwriting any existing output
  const name = `monoprint-${config.version}-source.tar.gz`;
  run('git', ['archive', '--format=tar.gz', `--prefix=monoprint-${config.version}/`, '-o', path.join(out, name), 'HEAD']);
  const bytes = await fs.readFile(path.join(out, name));
  await fs.writeFile(path.join(out, `${name}.sha256`), `${sha256(bytes)}  ${name}\n`);
  await fs.writeFile(path.join(out, 'source.json'), JSON.stringify({ version: config.version, commit: revision(), archive: name, sha256: sha256(bytes) }, null, 2) + '\n');
  console.log('Exported current source without development Git history. History policy still requires review.');
} else if (command === 'assemble') {
  if (!option('--from') || !option('--out')) throw new Error('Usage: release.mjs assemble --from ARTIFACTS --out FRESH_DIRECTORY [--distribution]');
  const from = path.resolve(option('--from'));
  const out = path.resolve(option('--out'));
  const commit = revision();
  const files = [];
  const expected = [
    ['cli-aarch64-apple-darwin', 'aarch64-apple-darwin'],
    ['cli-x86_64-unknown-linux-gnu', 'x86_64-unknown-linux-gnu'],
    ['macos-app', 'aarch64-apple-darwin'], ['npm-release', null],
  ];
  for (const [folder, target] of expected) {
    const receipt = await readJson(path.join(from, folder, 'manifest.json'));
    if (receipt.version !== config.version || receipt.commit !== commit || !receipt.release || receipt.dirty || (target && receipt.target !== target)) throw new Error(`Artifact source/version mismatch: ${folder}`);
    if (folder.startsWith('cli-') && receipt.inputHash !== await inputHash()) throw new Error(`Native inputs mismatch: ${folder}`);
    if (folder === 'macos-app') {
      const hash = run('python3', ['-c', 'import sys; sys.path.insert(0,"scripts"); from macos_release import source_digest; print(source_digest("."))'], { stdio: 'pipe' }).toString().trim();
      if (receipt.inputHash !== hash) throw new Error('App packaging inputs mismatch');
    }
    if (args.includes('--distribution') && folder !== 'npm-release' && target?.endsWith('apple-darwin') && (!receipt.signed || receipt.notarization?.status !== 'Accepted' || (folder === 'macos-app' && !receipt.stapled))) throw new Error(`${folder} must be signed/notarized (and app stapled) before public distribution`);
    const entries = folder === 'npm-release' ? receipt.tarballs : [{ file: receipt.archive, sha256: receipt.archiveSha256 }];
    if (folder === 'npm-release') {
      const { validatePackages } = await import('../bindings/node/scripts/release-validation.mjs');
      await validatePackages(path.join(from, folder), receipt);
    }
    for (const entry of entries) {
      if (path.isAbsolute(entry.file) || entry.file.split(/[\\/]/).includes('..')) throw new Error('Unsafe artifact filename');
      const file = path.join(from, folder, entry.file);
      const bytes = await fs.readFile(file);
      if (sha256(bytes) !== entry.sha256) throw new Error(`Archive checksum mismatch: ${file}`);
      if (files.some(f => f.name === path.basename(file))) throw new Error('Duplicate release artifact name');
      files.push({ source: file, name: path.basename(file), sha256: entry.sha256, bytes: bytes.length });
    }
  }
  await fs.mkdir(path.dirname(out), { recursive: true });
  await fs.mkdir(out);
  for (const entry of files) await fs.copyFile(entry.source, path.join(out, entry.name));
  await fs.writeFile(path.join(out, 'SHA256SUMS'), files.map(f => `${f.sha256}  ${f.name}\n`).join(''));
  await fs.writeFile(path.join(out, 'release.json'), JSON.stringify({ version: config.version, commit, distribution: args.includes('--distribution'), files: files.map(({ source, ...entry }) => entry) }, null, 2) + '\n');
  console.log(`Verified ${files.length} archives from ${commit}`);
} else {
  throw new Error('Usage: node scripts/release.mjs check|snapshot|assemble [OPTIONS]');
}
