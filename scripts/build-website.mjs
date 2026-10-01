// Produce a local static Astro build. No hosting API or deployment step.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { root, run } from '../bindings/node/scripts/shared.mjs';
import { prepareAssets, website } from '../website/scripts/prepare-assets.mjs';
const args = process.argv.slice(2);
const option = (name, fallback) => args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
for (let i = 0; i < args.length; i++) {
  if (!['--out', '--cli'].includes(args[i]) || !args[++i] || args[i].startsWith('--')) throw new Error('Usage: node scripts/build-website.mjs --out FRESH_DIRECTORY [--cli BINARY]');
}
if (!args.includes('--out')) throw new Error('Choose a fresh --out directory');
const out = path.resolve(option('--out'));
const cli = path.resolve(option('--cli', path.join(root, 'target/release/monoprint')));
await fs.mkdir(path.dirname(out), { recursive: true });
await fs.mkdir(out);
const stage = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-site-'));
try {
  await prepareAssets(stage, cli);
  run(process.execPath, [path.join(website, 'node_modules/astro/bin/astro.mjs'), 'build', '--outDir', out], {
    cwd: website, env: { ...process.env, MONOPRINT_SITE_PUBLIC_DIR: stage },
  });
} finally { await fs.rm(stage, { recursive: true, force: true }); }
console.log(`Built local website in ${out}; no deployment performed.`);
