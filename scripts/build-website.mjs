// Produce a local static website. No server, hosting API or deployment step.
import fs from 'node:fs/promises';
import path from 'node:path';
import { root, run, revision, sha256 } from '../bindings/node/scripts/shared.mjs';
import { checkRelease } from './release-config.mjs';

const args = process.argv.slice(2);
const option = (name, fallback) => args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
for (let i = 0; i < args.length; i++) {
  if (!['--out', '--cli'].includes(args[i]) || !args[++i] || args[i].startsWith('--')) throw new Error('Usage: node scripts/build-website.mjs --out FRESH_DIRECTORY [--cli BINARY]');
}
if (!args.includes('--out')) throw new Error('Choose a fresh --out directory');
const config = await checkRelease();
const out = path.resolve(option('--out'));
const cli = path.resolve(option('--cli', path.join(root, 'target/release/monoprint')));
await fs.mkdir(path.dirname(out), { recursive: true });
await fs.mkdir(out);
await fs.cp(path.join(root, 'website'), out, { recursive: true });
const assets = path.join(out, 'assets');
await fs.mkdir(assets, { recursive: true });
for (const name of ['inter.ttf', 'inter-OFL.txt', 'commit-mono.otf', 'commit-mono-OFL.txt']) await fs.copyFile(path.join(root, 'assets/fonts', name), path.join(assets, name));
await fs.copyFile(path.join(root, 'LICENSE'), path.join(out, 'LICENSE'));
let index = await fs.readFile(path.join(out, 'index.html'), 'utf8');
index = index.replaceAll('__REPOSITORY__', `https://github.com/${config.repository}`);
await fs.writeFile(path.join(out, 'index.html'), index);
const source = path.join(out, 'sample.rs');
await fs.writeFile(source, '// A little code. A lasting impression.\nfn main() {\n    let theme = "Monoprint";\n    println!("Hello, {theme}!");\n}\n');
const samples = [['gruvbox', 'gruvbox-medium', 'dark'], ['github', 'github-default', 'light'], ['removed-frame', 'removed-frame', 'dark'], ['nord', 'nord', 'dark']];
const images = [];
for (const [name, theme, appearance] of samples) {
  const file = path.join(assets, `${name}.png`);
  run(cli, [source, '--theme', theme, `--${appearance}`, '--width', '720', '--padding', '48', '--scale', '2', '--title', 'main.rs', '--png-compression', 'balanced', '-o', file]);
  images.push({ name, theme, appearance, sha256: sha256(await fs.readFile(file)) });
}
await fs.unlink(source);
await fs.writeFile(path.join(out, 'build.json'), JSON.stringify({ version: config.version, commit: revision(), privatePreview: true, images }, null, 2) + '\n');
console.log(`Built local website in ${out}; no deployment performed.`);
