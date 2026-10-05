import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { root, run, revision, isDirty, sha256 } from '../../bindings/node/scripts/shared.mjs';
import { checkRelease } from '../../scripts/release-config.mjs';
import { themes, source } from '../src/data/demo.mjs';

export const website = path.join(root, 'website');
export async function prepareAssets(directory = path.join(website, '.generated/public'), cli = process.env.MONOPRINT_CLI ?? path.join(root, 'target/release/monoprint')) {
  const config = await checkRelease();
  await fs.mkdir(path.join(directory, 'assets'), { recursive: true });
  const assets = path.join(directory, 'assets');
  for (const name of ['inter.ttf', 'inter-OFL.txt', 'jetbrains-mono.ttf', 'jetbrains-mono-OFL.txt']) await fs.copyFile(path.join(root, 'assets/fonts', name), path.join(assets, name));
  await fs.copyFile(path.join(website, 'assets/icon.png'), path.join(assets, 'icon.png'));
  await fs.copyFile(path.join(root, 'LICENSE'), path.join(directory, 'LICENSE'));
  const input = path.join(directory, 'share.rs');
  await fs.writeFile(input, source.join('\n'));
  const images = [];
  try {
    const samples = [
      ...themes.map(theme => ({ ...theme, highlight: true, format: 'png' })),
      { ...themes[3], id: 'catppuccin-plain', highlight: false, format: 'png' },
      { ...themes[0], id: 'gruvbox-transparent', highlight: false, transparent: true, format: 'svg' },
    ];
    for (const sample of samples) {
      const file = path.join(assets, `${sample.id}.${sample.format}`);
      const flags = [input, '--theme', sample.cli, `--${sample.appearance}`, '--width', '640', '--padding', '64', '--scale', '2', '--font', 'jetbrains-mono', '--font-size', '15', '--ligatures', '--line-numbers', '--title', 'share.rs'];
      if (sample.format === 'png') flags.push('--png-compression', 'balanced');
      if (sample.highlight) flags.push('--highlight', '4');
      if (sample.transparent) flags.push('--transparent');
      run(path.resolve(cli), [...flags, '-o', file, '--force']);
      images.push({ name: sample.id, theme: sample.cli, appearance: sample.appearance, format: sample.format, sha256: sha256(await fs.readFile(file)) });
    }
  } finally { await fs.unlink(input); }
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
