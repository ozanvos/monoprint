import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { setTimeout } from 'node:timers/promises';

const directory = path.resolve(process.argv[2]);
const manifest = JSON.parse(await fs.readFile(path.join(directory, 'manifest.json')));
const pending = new Map(await Promise.all(manifest.tarballs.map(async entry => [entry.name, {
  version: entry.version,
  integrity: `sha512-${createHash('sha512').update(await fs.readFile(path.join(directory, entry.file))).digest('base64')}`,
}])));
const deadline = Date.now() + 30 * 60_000;
while (pending.size) {
  for (const [name, expected] of pending) {
    const response = await fetch(`https://registry.npmjs.org/${name}/${expected.version}`, { signal: AbortSignal.timeout(30_000) });
    if (response.status === 404 || response.status === 429 || response.status >= 500) continue;
    if (!response.ok) throw new Error(`Registry lookup failed for ${name}: ${response.status}`);
    const actual = await response.json();
    if (actual.dist?.integrity !== expected.integrity) throw new Error(`Published bytes differ from the CI artifact: ${name}`);
    pending.delete(name);
    console.log(`Verified published artifact: ${name}@${expected.version}`);
  }
  if (!pending.size) break;
  if (Date.now() >= deadline) throw new Error(`npm availability timed out: ${[...pending.keys()].join(', ')}`);
  console.log(`Waiting for npm scanning: ${[...pending.keys()].join(', ')}`);
  await setTimeout(30_000);
}
