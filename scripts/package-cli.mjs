// Development-only release packaging. Builds the host CLI and never publishes.
import fs from 'node:fs/promises';
import path from 'node:path';
import { root, run, sha256, inputHash } from '../bindings/node/scripts/shared.mjs';
import { prepareNotices } from '../bindings/node/scripts/notices.mjs';

const args = process.argv.slice(2);
let destination;
let local = false;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--local') local = true;
  else if (args[i] === '--out' && args[i + 1] && !args[i + 1].startsWith('--')) destination = path.resolve(args[++i]);
  else throw new Error('Usage: node scripts/package-cli.mjs --out FRESH_DIRECTORY [--local]');
}
if (!destination) throw new Error('Choose a fresh --out directory');
if (await fs.stat(destination).then(() => true, () => false)) throw new Error(`${destination} already exists; choose a fresh --out directory`);
const dirty = run('git', ['status', '--porcelain'], { stdio: 'pipe' }).toString().trim() !== '';
if (dirty && !local) throw new Error('Release packaging requires a clean checkout; --local creates verification-only artifacts');
const commit = run('git', ['rev-parse', 'HEAD'], { stdio: 'pipe' }).toString().trim();
const compiler = run('rustc', ['-vV'], { stdio: 'pipe' }).toString();
const triple = compiler.match(/^host: (.+)$/m)[1];
if (!['aarch64-apple-darwin', 'x86_64-unknown-linux-gnu'].includes(triple)) {
  throw new Error(`CLI release target ${triple} is not in the initial supported matrix`);
}
run(process.execPath, [path.join(root, 'scripts/import-grammar-metadata.mjs'), '--check']);
run(process.execPath, [path.join(root, 'scripts/import-editor-themes.mjs'), '--check']);
run('python3', [path.join(root, 'scripts/fetch-fallback-fonts.py'), '--check']);
const before = await inputHash();
run('cargo', ['build', '--locked', '--release', '--target', triple, '--package', 'monoprint', '--bin', 'monoprint']);
const metadata = JSON.parse(run('cargo', ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'], { stdio: 'pipe' }));
const pkg = metadata.packages.find(p => p.name === 'monoprint');
const name = `monoprint-${pkg.version}-${triple}`;
const folder = path.join(destination, name);
await fs.mkdir(folder, { recursive: true });
await prepareNotices(folder, [{ triple }], 'monoprint');
await fs.cp(path.join(root, 'docs'), path.join(folder, 'docs'), { recursive: true });
await fs.copyFile(path.join(root, 'README.md'), path.join(folder, 'README.md'));
await fs.copyFile(path.join(root, 'Cargo.lock'), path.join(folder, 'Cargo.lock'));
const binary = await fs.readFile(path.join(metadata.target_directory, triple, 'release/monoprint'));
await fs.writeFile(path.join(folder, 'monoprint'), binary, { mode: 0o755 });
if (before !== await inputHash()) throw new Error('Native inputs changed while building; discard these artifacts and rebuild');
if (commit !== run('git', ['rev-parse', 'HEAD'], { stdio: 'pipe' }).toString().trim()) throw new Error('Checkout revision changed while building; discard these artifacts and rebuild');
const finalDirty = run('git', ['status', '--porcelain'], { stdio: 'pipe' }).toString().trim() !== '';
if (finalDirty && !local) throw new Error('Checkout changed while packaging; discard these artifacts and rebuild');
const receipt = {
  version: pkg.version, target: triple, release: !local,
  commit, dirty: finalDirty, inputHash: before, binarySha256: sha256(binary), binaryBytes: binary.length,
  compiler: compiler.trim(),
};
await fs.writeFile(path.join(folder, 'build.json'), JSON.stringify(receipt, null, 2) + '\n');
const archiveName = `${name}.tar.gz`;
run('tar', ['-czf', path.join(destination, archiveName), '-C', destination, name]);
const archive = await fs.readFile(path.join(destination, archiveName));
const archiveSha256 = sha256(archive);
await fs.writeFile(path.join(destination, `${archiveName}.sha256`), `${archiveSha256}  ${archiveName}\n`);
await fs.writeFile(path.join(destination, 'manifest.json'), JSON.stringify({ ...receipt, archive: archiveName, archiveSha256, archiveBytes: archive.length }, null, 2) + '\n');
console.log(`Packaged ${archiveName}${local ? ' (local verification only)' : ''}`);
