import fs from 'node:fs/promises';
import path from 'node:path';
import { pkg, targets, cliFile, run, sha256, inputHash, revision } from './shared.mjs';

export async function validatePackages(source, manifest) {
  if (!manifest.release || manifest.dirty || manifest.commit !== revision() || manifest.name !== pkg.name || manifest.version !== pkg.version || manifest.inputHash !== await inputHash()
    || JSON.stringify(manifest.targets.map(t => t.id).sort()) !== JSON.stringify(targets.map(t => t.id).sort())
    || manifest.targets.some(t => t.commit !== manifest.commit || t.dirty || t.inputHash !== manifest.inputHash || t.version !== pkg.version
      || t.target !== t.id || t.triple !== targets.find(target => target.id === t.id)?.triple)) {
    throw new Error('Publication requires a complete release package set matching the current clean source revision, native inputs and version');
  }
  const expected = [...targets.map(t => `${pkg.name}-${t.id}`), pkg.name];
  if (JSON.stringify(manifest.tarballs.map(t => t.name)) !== JSON.stringify(expected)) throw new Error('Publish platform packages first, then the main package; artifact set is incomplete');
  for (const entry of manifest.tarballs) {
    if (entry.version !== pkg.version || entry.file !== `tarballs/${path.basename(entry.file)}` || sha256(await fs.readFile(path.join(source, entry.file))) !== entry.sha256) throw new Error(`Tarball changed: ${entry.name}`);
    const packed = JSON.parse(run('tar', ['-xOf', path.join(source, entry.file), 'package/package.json'], { stdio: 'pipe' }));
    if (packed.name !== entry.name || packed.version !== pkg.version || packed.repository?.url !== pkg.repository?.url) throw new Error(`Packed metadata mismatch: ${entry.name}`);
    if (entry.name === pkg.name && JSON.stringify(packed.optionalDependencies) !== JSON.stringify(Object.fromEntries(targets.map(t => [`${pkg.name}-${t.id}`, pkg.version])))) throw new Error('Packed optional dependencies must match all native packages exactly');
    if (entry.name === pkg.name) {
      if (JSON.stringify(packed.bin) !== JSON.stringify(pkg.bin)) throw new Error('Packed CLI bin entry must match the package manifest');
      const launcher = run('tar', ['-xOf', path.join(source, entry.file), `package/${pkg.bin.monoprint}`], { stdio: 'pipe' });
      if (launcher.toString() !== await fs.readFile(new URL('../js/cli.cjs', import.meta.url), 'utf8')) throw new Error('Packed CLI launcher must match the source');
    }
    if (entry.name !== pkg.name) {
      const target = manifest.targets.find(t => entry.name === `${pkg.name}-${t.id}`);
      const native = run('tar', ['-xOf', path.join(source, entry.file), `package/monoprint.${target.id}.node`], { stdio: 'pipe', maxBuffer: 128 * 1024 * 1024 });
      if (sha256(native) !== target.sha256) throw new Error(`Packed native binary does not match its build receipt: ${entry.name}`);
      const cli = run('tar', ['-xOf', path.join(source, entry.file), `package/${cliFile(targets.find(t => t.id === target.id))}`], { stdio: 'pipe', maxBuffer: 128 * 1024 * 1024 });
      if (sha256(cli) !== target.cliSha256) throw new Error(`Packed CLI binary does not match its build receipt: ${entry.name}`);
    }
  }
}
