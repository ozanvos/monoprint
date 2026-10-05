// Static-site checks using the repository's existing Playwright tooling.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { chromium } from 'playwright';
import release from '../release/config.json' with { type: 'json' };

const args = process.argv.slice(2);
const option = flag => args[args.indexOf(flag) + 1];
if (args.length !== 4 || !args.includes('--from') || !args.includes('--out')) throw new Error('Usage: node scripts/test-website.mjs --from WEBSITE --out SCREENSHOTS');
const source = path.resolve(option('--from'));
const out = path.resolve(option('--out'));
await fs.mkdir(out, { recursive: true });
const types = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.svg': 'image/svg+xml', '.ttf': 'font/ttf', '.otf': 'font/otf' };
const server = http.createServer(async (request, response) => {
  const file = path.resolve(source, '.' + new URL(request.url, 'http://localhost').pathname.replace(/\/$/, '/index.html'));
  if (!file.startsWith(source + path.sep)) { response.writeHead(403).end(); return; }
  try { response.setHeader('Content-Type', types[path.extname(file)] ?? 'application/octet-stream'); response.end(await fs.readFile(file)); }
  catch { response.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
  const url = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}) });
  const context = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] });
  const page = await context.newPage();
  const errors = [];
  const network = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin !== url) { network.push(route.request().url()); return route.abort(); }
    return route.continue();
  });
  await page.goto(url);
  await page.evaluate(async () => {
    await document.fonts.ready;
    document.querySelectorAll('img').forEach(image => image.loading = 'eager');
  });
  await page.waitForFunction(() => [...document.images].every(img => img.complete && img.naturalWidth > 0));
  if (release.websitePublishing === true) {
    assert.equal(await page.locator('meta[name=robots]').count(), 0);
    assert.equal(await page.locator('link[rel=canonical]').getAttribute('href'), release.homepage + '/');
  } else {
    assert.equal(await page.locator('meta[name=robots]').getAttribute('content'), 'noindex,nofollow');
  }
  assert.equal(await page.getByRole('link', { name: /Download/i }).count(), 0);
  assert.equal(await page.getByRole('button', { name: 'Download', exact: true }).count(), 2);
  assert.equal(await page.locator('button:disabled').count(), 2);
  const samples = [
    ['Gruvbox Dark', 'gruvbox', 'gruvbox-medium'],
    ['Ayu Light', 'ayu', 'ayu --light'],
    ['Night Owl', 'nightowl', 'night-owl'],
    ['Catppuccin Mocha', 'catppuccin', 'catppuccin'],
    ['Solarized Light', 'solarized', 'solarized --light'],
  ];
  for (const [width, height] of [[1440, 1100], [1024, 900], [768, 1000], [390, 844], [320, 740]]) {
    await page.setViewportSize({ width, height });
    for (const [label, id, cli] of samples) {
      const button = page.getByRole('button', { name: label, exact: true });
      await button.click();
      assert.equal(await button.getAttribute('aria-pressed'), 'true');
      assert.equal(await page.locator('.preview-image:visible').count(), 1);
      assert.equal(await page.locator('.preview-image:visible').getAttribute('id'), `preview-${id}`);
      assert.equal(await page.locator('#command-text').textContent(), `monoprint share.rs --theme ${cli} --highlight 4`);
      assert.equal(await page.locator('#export-image').getAttribute('download'), `monoprint-${id}.png`);
    }
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), `Overflow at ${width}px`);
    await page.getByRole('button', { name: 'Night Owl', exact: true }).click();
    await page.evaluate(() => scrollTo(0, 0));
    await page.screenshot({ path: path.join(out, `website-${width}.png`), fullPage: true });
  }
  await page.setViewportSize({ width: 1440, height: 1100 });
  await page.getByRole('searchbox').fill('AYU');
  assert.equal(await page.locator('[data-theme]:visible').count(), 1);
  await page.getByRole('button', { name: 'Ayu Light', exact: true }).click();
  await page.getByRole('searchbox').fill('missing theme');
  assert.equal(await page.locator('[data-theme]:visible').count(), 0);
  assert.equal(await page.locator('#no-themes').isVisible(), true);
  await page.getByRole('searchbox').fill('');
  const copy = page.getByRole('button', { name: 'Copy CLI command', exact: true });
  await copy.click();
  await page.waitForFunction(() => document.querySelector('#copy-command').title === 'Copied');
  assert.equal(await page.evaluate(() => navigator.clipboard.readText()), 'monoprint share.rs --theme ayu --light --highlight 4');
  await page.waitForFunction(() => !document.querySelector('#copy-command').disabled);
  const exportBefore = await page.locator('#export-image').boundingBox();
  await page.getByRole('button', { name: 'Copy preview image', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('#copy-image').title === 'Copied');
  assert.deepEqual(await page.locator('#export-image').boundingBox(), exportBefore, 'Copy feedback must not move Export');
  assert.ok(await page.evaluate(async () => (await navigator.clipboard.read()).some(item => item.types.includes('image/png'))));
  const downloading = page.waitForEvent('download');
  await page.getByRole('link', { name: 'Export…', exact: true }).click();
  const download = await downloading;
  assert.equal(download.suggestedFilename(), 'monoprint-ayu.png');
  assert.deepEqual(await fs.readFile(await download.path()), await fs.readFile(path.join(source, 'assets/ayu.png')));
  for (const name of ['Support', 'Privacy']) {
    await page.goto(`${url}/${name.toLowerCase()}/`);
    assert.equal(await page.getByRole('heading', { level: 1 }).textContent(), `Monoprint ${name === 'Support' ? 'support' : 'privacy policy'}`);
    assert.ok(await page.locator('a[href="mailto:contact@7ea.st"]').count() > 0);
    assert.ok(await page.locator('a[href="/"]').count() > 0);
    await page.setViewportSize({ width: 320, height: 740 });
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), `${name} overflows at 320px`);
    await page.screenshot({ path: path.join(out, `${name.toLowerCase()}-320.png`), fullPage: true });
  }
  await context.close();
  assert.deepEqual(errors, []);
  assert.deepEqual(network, []);
  console.log('Five themes, search, CLI/image clipboard, PNG download, five viewport sizes, local assets, publication state and support/privacy pages passed.');
} finally {
  if (browser) await browser.close();
  server.close();
}
