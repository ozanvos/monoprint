import fs from 'node:fs/promises';
import path from 'node:path';
import { directory, root, run, readJson, filesUnder, sha256 } from './shared.mjs';

export async function prepareNotices(destination, targets) {
  await fs.mkdir(destination, { recursive: true });
  for (const name of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) await fs.copyFile(path.join(root, name), path.join(destination, name));
  await fs.cp(path.join(root, 'licenses'), path.join(destination, 'licenses'), { recursive: true });
  await fs.cp(path.join(directory, 'licenses'), path.join(destination, 'licenses/node'), { recursive: true });
  for (const file of ['docs/grammar-licenses.md', 'docs/grammar-license-validation.json', 'assets/fonts/manifest.json',
    'assets/removed-frames/provenance.json', 'assets/editor/sources.json', 'assets/grammar-provenance.json', 'vendor/takumi-svg/PATCHES.md',
    'vendor/takumi-svg/LICENSE-MIT', 'vendor/takumi-svg/LICENSE-APACHE',
    'vendor/takumi-raster/PATCHES.md', 'vendor/takumi-raster/LICENSE-MIT', 'vendor/takumi-raster/LICENSE-APACHE',
    ...(await fs.readdir(path.join(root, 'assets/fonts'))).filter(n => n.endsWith('-OFL.txt')).map(n => `assets/fonts/${n}`)]) {
    await fs.mkdir(path.dirname(path.join(destination, file)), { recursive: true });
    await fs.copyFile(path.join(root, file), path.join(destination, file));
  }
  const packages = new Map();
  const selected = new Set();
  for (const target of targets) {
    const metadata = JSON.parse(run('cargo', ['metadata', '--offline', '--locked', '--no-default-features', '--filter-platform', target.triple, '--format-version', '1'], { stdio: 'pipe', maxBuffer: 32 * 1024 * 1024 }));
    for (const pkg of metadata.packages) packages.set(pkg.id, pkg);
    const nodes = new Map(metadata.resolve.nodes.map(n => [n.id, n]));
    const pending = [metadata.packages.find(p => p.name === 'monoprint-node').id];
    const visited = new Set();
    while (pending.length) {
      const id = pending.pop();
      if (visited.has(id)) continue;
      visited.add(id);
      selected.add(id);
      pending.push(...nodes.get(id).dependencies);
    }
  }
  const upstream = await readJson(path.join(root, 'licenses/upstream/manifest.json'));
  const nodeUpstream = await readJson(path.join(directory, 'licenses/upstream.json'));
  const inventory = [];
  const lock = await fs.readFile(path.join(root, 'Cargo.lock'), 'utf8');
  let notices = `MONOPRINT NODE BINDING — LOCKED RUST DEPENDENCY NOTICES\nIncludes build dependencies for these resolved targets: ${targets.map(t => t.triple).join(', ')}.\n`;
  for (const id of [...selected].sort()) {
    const pkg = packages.get(id);
    if (!pkg.source) continue;
    const sourceRoot = path.dirname(pkg.manifest_path);
    const files = (await filesUnder(sourceRoot)).filter(file => /^(?:licen[cs]e|copying|notice)/i.test(path.basename(file)));
    const fallback = upstream.filter(record => record.packages.includes(pkg.name));
    const nodeFallback = nodeUpstream.filter(record => record.name === pkg.name && record.version === pkg.version);
    if (!files.length && !fallback.length && !nodeFallback.length) throw new Error(`Missing packaged license text for ${pkg.name}@${pkg.version}; review before packaging`);
    const record = { name: pkg.name, version: pkg.version, license: pkg.license, repository: pkg.repository,
      source: `https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download`, noticeFiles: files.map(file => path.relative(sourceRoot, file)) };
    inventory.push(record);
    for (const file of files) notices += `\n${'='.repeat(72)}\n${pkg.name} ${pkg.version} — ${path.relative(sourceRoot, file)}\n${'='.repeat(72)}\n${await fs.readFile(file, 'utf8')}`;
    for (const entry of fallback) {
      const bytes = await fs.readFile(path.join(root, entry.file));
      if (sha256(bytes) !== entry.sha256) throw new Error(`Upstream notice changed: ${entry.file}`);
      notices += `\n${pkg.name} ${pkg.version} — ${entry.source}\n${bytes.toString('utf8')}\n`;
    }
    for (const entry of nodeFallback) {
      const bytes = await fs.readFile(path.join(directory, 'licenses', entry.file));
      if (sha256(bytes) !== entry.sha256) throw new Error(`Binding dependency notice changed: ${entry.file}`);
      notices += `\n${pkg.name} ${pkg.version} — ${entry.source}\n${bytes.toString('utf8')}\n`;
    }
    if (pkg.license === 'EUPL-1.2' || pkg.license === 'MPL-2.0') {
      const archive = path.resolve(sourceRoot, '../../../cache', path.basename(path.dirname(sourceRoot)), `${pkg.name}-${pkg.version}.crate`);
      const bytes = await fs.readFile(archive);
      const entry = lock.replaceAll('\r\n', '\n').split('[[package]]').find(entry => entry.includes(`\nname = "${pkg.name}"\n`) && entry.includes(`\nversion = "${pkg.version}"\n`));
      const checksum = entry?.match(/\nchecksum = "([a-f0-9]{64})"/)?.[1];
      if (sha256(bytes) !== checksum) throw new Error(`${pkg.name} source archive checksum mismatch`);
      const source = `licenses/sources/${pkg.name}-${pkg.version}.crate`;
      await fs.mkdir(path.join(destination, 'licenses/sources'), { recursive: true });
      await fs.writeFile(path.join(destination, source), bytes);
      record.bundledSource = source;
      record.sha256 = sha256(bytes);
    }
  }
  await fs.writeFile(path.join(destination, 'licenses/rust-dependencies.json'), JSON.stringify(inventory, null, 2) + '\n');
  await fs.writeFile(path.join(destination, 'licenses/rust-dependency-notices.txt'), notices);
  await fs.writeFile(path.join(destination, 'licenses/sources/README.md'),
    '# Corresponding dependency source\n\nUnchanged source archives for Giallo (EUPL-1.2) and MPL-2.0 Rust dependencies accompany the native binaries. Extract each `.crate` as a gzip-compressed tar archive. Versions, licenses and checksums are recorded in `../rust-dependencies.json`; license texts are in `../rust-dependency-notices.txt`. MPL-covered grammar sources are in `../grammar-sources/`. Other exact dependency source archives are linked in the inventory. Original third-party licenses remain applicable.\n');
}
