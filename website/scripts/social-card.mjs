// Renders the link-preview card to website/assets/social-card.png. The code sample is the CLI's
// own renders; the layout around them is HTML captured by Chromium, so this runs on demand rather
// than in every site build. Run it after changing the copy, the brand mark or the shipped platforms:
//   node website/scripts/social-card.mjs   (CHROME_PATH selects an installed Chrome)
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { chromium } from 'playwright';
import { root, run } from '../../bindings/node/scripts/shared.mjs';
import release from '../../release/config.json' with { type: 'json' };
import { socialCard } from '../src/data/demo.mjs';

const cli = path.resolve(process.env.MONOPRINT_CLI ?? path.join(root, 'target/release/monoprint'));
const website = path.join(root, 'website');
const desktop = [['macos-', 'macOS'], ['win-', 'Windows'], ['linux-', 'Linux']]
  .filter(([prefix]) => release.desktopPlatforms.some(runtime => runtime.startsWith(prefix)))
  .map(([, name]) => name);
// The house Monoprint theme in front, with a familiar classic behind it. The sample is a shortened
// share.rs, set large enough to read in a feed thumbnail.
const source = [
  'fn share(code: &str) -> Image {',
  '    let tokens = highlight(code);',
  '    // Rendered offline.',
  '    render(tokens).png()',
  '}',
];
const windows = [
  { file: 'gruvbox.png', args: ['--theme', 'gruvbox-medium', '--dark'], style: 'left:648px;top:52px' },
  { file: 'monoprint.png', args: ['--theme', 'monoprint', '--light', '--highlight', '3'], style: 'left:580px;top:236px' },
];
// The header's brand mark, drawn larger.
const mark = '<svg viewBox="196 249 633 498" aria-hidden="true"><path fill="#5c5c58" d="M 510 381 L 620 522 L 499 598 L 465 548 Z"/><path fill="#2a2a28" d="M 220 448 L 493 263 Q 513 252 513 275 L 512 505 Q 512 515 502 522 L 220 720 Q 199 732 201 708 L 208 468 Q 208 457 220 448 Z"/><path fill="#8d8d88" d="M 499 598 L 781 407 Q 798 395 801 416 L 823 598 Q 826 613 813 621 L 625 735 Q 613 744 605 731 Z"/></svg>';

const work = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-card-'));
try {
  for (const name of ['inter.ttf', 'jetbrains-mono.ttf']) await fs.copyFile(path.join(root, 'assets/fonts', name), path.join(work, name));
  const input = path.join(work, 'share.rs');
  await fs.writeFile(input, source.join('\n'));
  for (const window of windows) {
    run(cli, [input, ...window.args, '--background', 'none', '--width', '560', '--padding', '0', '--scale', '2', '--font', 'jetbrains-mono', '--font-size', '20', '--ligatures', '--line-numbers', '--title', 'share.rs', '-o', path.join(work, window.file), '--force']);
  }
  await fs.writeFile(path.join(work, 'card.html'), `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face{font-family:JetBrains Mono;src:url(jetbrains-mono.ttf)}
@font-face{font-family:Inter;src:url(inter.ttf)}
*{box-sizing:border-box;margin:0}
body{width:${socialCard.width}px;height:${socialCard.height}px;overflow:hidden;position:relative;color:#2a2a28;font-family:JetBrains Mono;
  background:#e8e8e5 linear-gradient(rgb(255 255 255/42%) 1px,transparent 1px) 0 0/14px 14px;}
body::before{content:'';position:absolute;inset:0;background:linear-gradient(90deg,rgb(255 255 255/42%) 1px,transparent 1px) 0 0/14px 14px}
.copy{position:absolute;left:64px;top:60px;bottom:56px;width:500px;display:flex;flex-direction:column}
.brand{display:flex;align-items:center;gap:14px;font-size:30px;font-weight:650;letter-spacing:-.02em}
.brand svg{width:40px;height:32px}
h1{margin-top:auto;font-size:100px;line-height:.95;font-weight:500;letter-spacing:-.055em}
p{margin-top:26px;font-family:Inter;font-size:27px;line-height:1.4;color:#4a4a47;text-wrap:balance}
ul{margin-top:auto;padding:0;list-style:none;display:flex;gap:10px}
li{font-size:16px;letter-spacing:.03em;color:#4a4a47;background:#f7f7f5;border:1px solid #d0d0cc;border-radius:4px;padding:7px 11px}
img{position:absolute;width:560px;filter:drop-shadow(0 22px 40px rgb(30 30 30/24%))}
</style></head><body>
<div class="copy">
  <div class="brand">${mark}Monoprint</div>
  <h1>Code images.</h1>
  <p>PNG, SVG or HTML, rendered on your machine.</p>
  <ul><li>Free</li><li>Open source</li><li>${[...desktop, 'CLI'].join(' · ')}</li></ul>
</div>
${windows.map(window => `<img src="${window.file}" style="${window.style}" alt="">`).join('\n')}
</body></html>`);
  const browser = await chromium.launch(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {});
  try {
    const page = await browser.newPage({ viewport: { width: socialCard.width, height: socialCard.height } });
    await page.goto('file://' + path.join(work, 'card.html'));
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: path.join(website, 'assets', socialCard.file) });
  } finally { await browser.close(); }
} finally { await fs.rm(work, { recursive: true, force: true }); }
console.log(`Wrote website/assets/${socialCard.file}`);
