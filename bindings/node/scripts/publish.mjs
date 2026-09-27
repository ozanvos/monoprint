import fs from 'node:fs/promises';
import path from 'node:path';
import { directory, pkg, targets, npm, readJson, sha256, inputHash } from './shared.mjs';

const args = process.argv.slice(2);
const option = (flag, fallback) => args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--dry-run') continue;
  if (['--from', '--tag'].includes(args[i]) && args[i + 1] && !args[i + 1].startsWith('--')) { i++; continue; }
  throw new Error(`Unknown or incomplete publish option ${args[i]}`);
}
const source = path.resolve(option('--from', path.join(directory, '.packages')));
const manifest = await readJson(path.join(source, 'manifest.json'));
if (!manifest.release || manifest.name !== pkg.name || manifest.version !== pkg.version || manifest.inputHash !== await inputHash()
  || JSON.stringify(manifest.targets.map(t => t.id).sort()) !== JSON.stringify(targets.map(t => t.id).sort())) {
  throw new Error('Publication requires a complete release package set matching the current native inputs and version');
}
const expected = [...targets.map(t => `${pkg.name}-${t.id}`), pkg.name];
if (JSON.stringify(manifest.tarballs.map(t => t.name)) !== JSON.stringify(expected)) throw new Error('Publish platform packages first, then the main package; artifact set is incomplete');
for (const entry of manifest.tarballs) {
  if (entry.file !== `tarballs/${path.basename(entry.file)}` || sha256(await fs.readFile(path.join(source, entry.file))) !== entry.sha256) throw new Error(`Tarball changed: ${entry.name}`);
}
const tag = option('--tag', pkg.version.includes('-') ? 'next' : 'latest');
if (args.includes('--dry-run')) {
  console.log(`Validated ${manifest.tarballs.length} tarballs for tag ${tag}. No registry access or publication performed.`);
} else {
  // No lifecycle hooks: publishing never runs the NAPI CLI's implicit platform publisher.
  for (const entry of manifest.tarballs) npm(['publish', path.join(source, entry.file), '--access', 'public', '--tag', tag, '--ignore-scripts']);
}
