import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { root, run, revision, isDirty, sha256 } from '../../bindings/node/scripts/shared.mjs';
import { checkRelease } from '../../scripts/release-config.mjs';
import { themes, source, socialCard } from '../src/data/demo.mjs';
import { loopThemes, palette, previews } from '../src/data/loops.mjs';

export const website = path.join(root, 'website');
export async function prepareAssets(directory = path.join(website, '.generated/public'), cli = process.env.MONOPRINT_CLI ?? path.join(root, 'target/release/monoprint')) {
  const config = await checkRelease();
  await fs.mkdir(path.join(directory, 'assets'), { recursive: true });
  const assets = path.join(directory, 'assets');
  for (const name of ['inter.ttf', 'inter-OFL.txt', 'jetbrains-mono.ttf', 'jetbrains-mono-OFL.txt']) await fs.copyFile(path.join(root, 'assets/fonts', name), path.join(assets, name));
  for (const name of ['icon.png', 'favicon-32.png']) await fs.copyFile(path.join(website, 'assets', name), path.join(assets, name));
  await fs.copyFile(path.join(website, 'assets/apple-touch-icon.png'), path.join(directory, 'apple-touch-icon.png'));
  await fs.copyFile(path.join(root, 'LICENSE'), path.join(directory, 'LICENSE'));
  const images = [];
  // Theme picker samples share one snippet and match the command shown beside them.
  const pickerInput = path.join(directory, 'share.rs');
  await fs.writeFile(pickerInput, source.join('\n'));
  try {
    for (const theme of themes) {
      const file = path.join(assets, `${theme.id}.png`);
      run(path.resolve(cli), [pickerInput, '--theme', theme.cli, `--${theme.appearance}`, '--width', '640', '--padding', '64', '--scale', '2', '--font', 'jetbrains-mono', '--font-size', '15', '--ligatures', '--line-numbers', '--title', 'share.rs', '--png-compression', 'balanced', '--highlight', '4', '-o', file, '--force']);
      images.push({ name: theme.id, source: 'share.rs', theme: theme.cli, appearance: theme.appearance, format: 'png', highlight: '4', transparent: false, sha256: sha256(await fs.readFile(file)) });
      // The CLI card's loop shows what its commands write, at 1x since it is drawn small.
      if (!loopThemes.includes(theme.cli)) continue;
      const output = path.join(assets, `output-${theme.id}.png`);
      run(path.resolve(cli), [pickerInput, '--theme', theme.cli, `--${theme.appearance}`, '--scale', '1', '--png-compression', 'balanced', '-o', output, '--force']);
      images.push({ name: `output-${theme.id}`, source: 'share.rs', theme: theme.cli, appearance: theme.appearance, format: 'png', highlight: null, transparent: false, sha256: sha256(await fs.readFile(output)) });
    }
  } finally { await fs.unlink(pickerInput); }
  const card = await fs.readFile(path.join(website, 'assets', socialCard.file));
  if (card.readUInt32BE(16) !== socialCard.width || card.readUInt32BE(20) !== socialCard.height) throw new Error('Social card size does not match socialCard in src/data/demo.mjs; run scripts/social-card.mjs');
  await fs.writeFile(path.join(assets, socialCard.file), card);
  const paletteInput = path.join(directory, 'palette.rs');
  await fs.writeFile(paletteInput, palette.join('\n'));
  try {
    for (const [appearance, backdrop] of previews) {
      const file = path.join(assets, `studio-${appearance}-${backdrop}.png`);
      run(path.resolve(cli), [paletteInput, '--theme', 'github-default', `--${appearance}`, '--width', '640', '--padding', '48', '--background', backdrop, '--line-numbers', '--title', 'palette.rs', '--scale', '2', '--png-compression', 'balanced', '-o', file, '--force']);
      images.push({ name: `studio-${appearance}-${backdrop}`, source: 'palette.rs', theme: 'github-default', appearance, format: 'png', highlight: null, transparent: false, sha256: sha256(await fs.readFile(file)) });
    }
  } finally { await fs.unlink(paletteInput); }
  const publicSite = config.websitePublishing === true;
  await fs.writeFile(path.join(directory, '_headers'), '/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: strict-origin-when-cross-origin\n' + (publicSite ? '' : '  X-Robots-Tag: noindex, nofollow\n'));
  await fs.writeFile(path.join(directory, 'robots.txt'), `User-agent: *\n${publicSite ? 'Allow' : 'Disallow'}: /\n`);
  // Keep the hosted pages and the App Store handoff sourced from the same copy.
  const legal = path.join(root, 'release/app-store/pages');
  await fs.copyFile(path.join(legal, 'style.css'), path.join(directory, 'legal.css'));
  for (const page of ['support', 'privacy']) {
    await fs.mkdir(path.join(directory, page), { recursive: true });
    const html = (await fs.readFile(path.join(legal, `${page}.html`), 'utf8'))
      .replace('href="style.css"', 'href="/legal.css"')
      .replaceAll('href="support.html"', 'href="/support/"')
      .replaceAll('href="privacy.html"', 'href="/privacy/"')
      .replace('<nav>', '<nav><a href="/">Monoprint</a>');
    await fs.writeFile(path.join(directory, page, 'index.html'), html);
  }
  await fs.writeFile(path.join(directory, 'build.json'), JSON.stringify({ version: config.version, commit: revision(), dirty: isDirty(), privatePreview: !publicSite, images }, null, 2) + '\n');
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await prepareAssets();
