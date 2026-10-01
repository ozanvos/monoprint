import fs from 'node:fs/promises';
import path from 'node:path';
import { directory, root, pkg, targets, run, npm, readJson, filesUnder, sha256, inputHash, revision, isDirty } from './shared.mjs';
import { prepareNotices } from './notices.mjs';

const args = process.argv.slice(2);
const option = (flag, fallback) => args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback;
const source = path.resolve(option('--artifacts', path.join(directory, 'native')));
const destination = path.resolve(option('--out', path.join(directory, '.packages')));
const local = args.includes('--local');
const commit = revision();
if (!local && isDirty()) throw new Error('Release npm packages require clean source; use --local for verification');
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--local') continue;
  if (['--artifacts', '--out'].includes(args[i]) && args[i + 1] && !args[i + 1].startsWith('--')) { i++; continue; }
  throw new Error(`Unknown or incomplete packaging option ${args[i]}`);
}
// Never delete a caller-selected output path. Refuse stale packages instead.
if (await fs.stat(destination).then(() => true, () => false)) throw new Error(`${destination} already exists; choose a fresh --out directory`);
run(process.execPath, [path.join(root, 'scripts/import-grammar-metadata.mjs'), '--check']);
run(process.execPath, [path.join(root, 'scripts/import-editor-themes.mjs'), '--check']);
run(process.execPath, [path.join(directory, 'scripts/build.mjs'), '--js-only']);
const hash = await inputHash();
const artifacts = (await filesUnder(source)).filter(file => file.endsWith('.node'));
const selected = [];
for (const target of targets) {
  const matches = artifacts.filter(file => path.basename(file) === `monoprint.${target.id}.node`);
  if (matches.length === 0 && local) continue;
  if (matches.length !== 1) throw new Error(`Expected one native artifact for ${target.id}, found ${matches.length}`);
  const bytes = await fs.readFile(matches[0]);
  const receipt = await readJson(matches[0].replace(/\.node$/, '.json'));
  if (receipt.target !== target.id || receipt.triple !== target.triple || receipt.version !== pkg.version || receipt.sha256 !== sha256(bytes) || receipt.inputHash !== hash || receipt.commit !== commit || (!local && receipt.dirty)) {
    throw new Error(`Stale or mismatched native artifact ${target.id}; rebuild against the current source/version`);
  }
  selected.push({ ...target, file: matches[0], receipt });
}
if (!selected.length || artifacts.length !== selected.length) throw new Error('No supported artifacts, or unexpected/duplicate native artifacts');
await fs.mkdir(destination, { recursive: true });
const legal = path.join(destination, 'legal');
await prepareNotices(legal, selected);
const tarballs = [];
const packages = path.join(destination, 'packages');
const packed = path.join(destination, 'tarballs');
await fs.mkdir(packed);
async function pack(name, manifest, prepare) {
  const folder = path.join(packages, name);
  await fs.cp(legal, folder, { recursive: true });
  await fs.writeFile(path.join(folder, 'package.json'), JSON.stringify(manifest, null, 2) + '\n');
  await prepare(folder);
  const result = JSON.parse(npm(['pack', '--ignore-scripts', '--json', '--pack-destination', packed], { cwd: folder, stdio: 'pipe', maxBuffer: 4 * 1024 * 1024 }))[0];
  const bytes = await fs.readFile(path.join(packed, result.filename));
  tarballs.push({ name: manifest.name, version: manifest.version, file: `tarballs/${result.filename}`, sha256: sha256(bytes), bytes: bytes.length, files: result.files.map(f => f.path) });
}
for (const target of selected) {
  await pack(target.id, { name: `${pkg.name}-${target.id}`, version: pkg.version,
    description: `Monoprint native binding for ${target.id}`, license: pkg.license,
    main: `monoprint.${target.id}.node`, os: [target.os], cpu: [target.cpu],
    ...(target.libc ? { libc: [target.libc] } : {}), engines: pkg.engines, publishConfig: pkg.publishConfig,
    ...(pkg.repository ? { repository: pkg.repository } : {}),
    ...(pkg.homepage ? { homepage: pkg.homepage } : {}),
    ...(pkg.bugs ? { bugs: pkg.bugs } : {}),
  }, async folder => {
    await fs.copyFile(target.file, path.join(folder, `monoprint.${target.id}.node`));
    await fs.writeFile(path.join(folder, 'README.md'), `# ${pkg.name}-${target.id}\n\nNative runtime dependency of \`${pkg.name}\`. Install the main package for its TypeScript interface. Third-party notices and corresponding Giallo/MPL dependency and grammar sources are included.\n`);
  });
}
const { scripts, devDependencies, ...publicPackage } = pkg;
await pack('main', { ...publicPackage, optionalDependencies: Object.fromEntries(selected.map(t => [`${pkg.name}-${t.id}`, pkg.version])) }, async folder => {
  await fs.cp(path.join(directory, 'dist'), path.join(folder, 'dist'), { recursive: true });
  await fs.copyFile(path.join(directory, 'README.md'), path.join(folder, 'README.md'));
});
if (hash !== await inputHash() || commit !== revision() || (!local && isDirty())) throw new Error('Source changed during packaging; discard and rebuild');
await fs.writeFile(path.join(destination, 'manifest.json'), JSON.stringify({
  name: pkg.name, version: pkg.version, release: !local, inputHash: hash,
  commit, dirty: isDirty(),
  targets: selected.map(({ id, receipt }) => ({ id, ...receipt })), tarballs,
}, null, 2) + '\n');
await fs.rm(legal, { recursive: true });
console.log(`Packed ${tarballs.length} npm tarballs in ${packed}${local ? ' (local verification only; publishing disabled)' : ''}`);
