import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

export const directory = fileURLToPath(new URL('..', import.meta.url));
export const root = path.resolve(directory, '../..');
export const readJson = async file => JSON.parse(await fs.readFile(file, 'utf8'));
export const pkg = await readJson(path.join(directory, 'package.json'));
export const targets = await readJson(path.join(directory, 'targets.json'));
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export const revision = () => run('git', ['rev-parse', 'HEAD'], { stdio: 'pipe' }).toString().trim();
export const isDirty = () => run('git', ['status', '--porcelain'], { stdio: 'pipe' }).toString().trim() !== '';
export function run(command, args, options = {}) {
  return execFileSync(command, args, { cwd: root, stdio: 'inherit', ...options });
}
export function npm(args, options = {}) {
  if (!process.env.npm_execpath) throw new Error('Run this command through npm run so its npm CLI path is known.');
  return run(process.execPath, [process.env.npm_execpath, ...args], options);
}
export async function filesUnder(directory) {
  const files = [];
  for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await filesUnder(file));
    else if (entry.isFile()) files.push(file);
  }
  return files.sort();
}
/** Bind a prebuilt artifact to its native inputs, including vendored fixes and embedded assets. */
export async function inputHash() {
  const paths = ['Cargo.toml', 'Cargo.lock', 'bindings/node/Cargo.toml', 'bindings/node/build.rs'];
  for (const dir of ['src', 'assets', 'bindings/node/src', 'vendor/takumi-svg/src', 'vendor/takumi-raster/src']) {
    paths.push(...(await filesUnder(path.join(root, dir))).map(file => path.relative(root, file).replaceAll('\\', '/')));
  }
  paths.push('vendor/takumi-svg/Cargo.toml');
  paths.push('vendor/takumi-raster/Cargo.toml');
  const hash = createHash('sha256');
  for (const file of paths.sort()) {
    let bytes = await fs.readFile(path.join(root, file));
    if (/\.(?:toml|lock|rs|json|css|txt)$/.test(file)) bytes = Buffer.from(bytes.toString('utf8').replaceAll('\r\n', '\n'));
    hash.update(file).update('\0').update(bytes).update('\0');
  }
  return hash.digest('hex');
}
