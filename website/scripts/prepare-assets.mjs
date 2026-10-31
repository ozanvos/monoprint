import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { root, run, revision, isDirty, sha256 } from '../../bindings/node/scripts/shared.mjs';
import { checkRelease } from '../../scripts/release-config.mjs';
import { showcases } from '../src/data/showcase.mjs';
import { themes, source } from '../src/data/demo.mjs';

export const website = path.join(root, 'website');
export async function prepareAssets(directory = path.join(website, '.generated/public'), cli = process.env.MONOPRINT_CLI ?? path.join(root, 'target/release/monoprint')) {
  const config = await checkRelease();
  await fs.mkdir(path.join(directory, 'assets'), { recursive: true });
  const assets = path.join(directory, 'assets');
  for (const name of ['inter.ttf', 'inter-OFL.txt', 'jetbrains-mono.ttf', 'jetbrains-mono-OFL.txt']) await fs.copyFile(path.join(root, 'assets/fonts', name), path.join(assets, name));
  await fs.copyFile(path.join(website, 'assets/icon.png'), path.join(assets, 'icon.png'));
  await fs.copyFile(path.join(root, 'LICENSE'), path.join(directory, 'LICENSE'));
  const images = [];
  for (const sample of showcases) {
    const input = path.join(website, 'assets/examples', sample.source);
    const file = path.join(assets, `${sample.id}.${sample.format}`);
    const flags = [input, '--theme', sample.theme, `--${sample.appearance}`, '--width', '480', '--padding', '16', '--scale', '2', '--font', 'jetbrains-mono', '--font-size', '16', '--ligatures', '--line-numbers', '--title', sample.source];
    if (sample.format === 'png') flags.push('--png-compression', 'balanced');
    if (sample.highlight) flags.push('--highlight', sample.highlight);
    if (sample.transparent) flags.push('--transparent');
    run(path.resolve(cli), [...flags, '-o', file, '--force']);
    images.push({ name: sample.id, source: sample.source, theme: sample.theme, appearance: sample.appearance, format: sample.format, highlight: sample.highlight ?? null, transparent: sample.transparent ?? false, sha256: sha256(await fs.readFile(file)) });
  }
  // Theme picker samples share one snippet and match the command shown beside them.
  const pickerInput = path.join(directory, 'share.rs');
  await fs.writeFile(pickerInput, source.join('\n'));
  try {
    for (const theme of themes) {
      const file = path.join(assets, `${theme.id}.png`);
      run(path.resolve(cli), [pickerInput, '--theme', theme.cli, `--${theme.appearance}`, '--width', '640', '--padding', '64', '--scale', '2', '--font', 'jetbrains-mono', '--font-size', '15', '--ligatures', '--line-numbers', '--title', 'share.rs', '--png-compression', 'balanced', '--highlight', '4', '-o', file, '--force']);
      images.push({ name: theme.id, source: 'share.rs', theme: theme.cli, appearance: theme.appearance, format: 'png', highlight: '4', transparent: false, sha256: sha256(await fs.readFile(file)) });
    }
  } finally { await fs.unlink(pickerInput); }
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
