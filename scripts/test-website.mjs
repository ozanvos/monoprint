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
const types = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.webp': 'image/webp', '.svg': 'image/svg+xml', '.ttf': 'font/ttf', '.otf': 'font/otf' };
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
  const screenshot = page.locator('.app-screenshot img');
  assert.equal(await screenshot.count(), 1);
  assert.match(await screenshot.getAttribute('alt'), /Monoprint for macOS/);
  assert.equal(await screenshot.getAttribute('width'), '2880');
  assert.equal(await screenshot.getAttribute('height'), '1800');
  assert.ok(await screenshot.getAttribute('srcset'), 'Hero needs responsive image sizes');
  for (const [width, height] of [[1440, 1100], [1024, 900], [768, 1000], [700, 1000], [390, 844], [320, 740]]) {
    await page.setViewportSize({ width, height });
    const bounds = await screenshot.boundingBox();
    assert.ok(Math.abs(bounds.width / bounds.height - 1.6) < 0.01, `Screenshot distorted at ${width}px`);
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), `Overflow at ${width}px`);
    await page.evaluate(() => scrollTo(0, 0));
    await page.screenshot({ path: path.join(out, `website-${width}.png`), fullPage: true });
    await page.locator('.showcase').screenshot({ path: path.join(out, `showcase-${width}.png`) });
  }
  await page.setViewportSize({ width: 1440, height: 1100 });
  const copy = page.getByRole('button', { name: 'Copy CLI command', exact: true });
  await copy.click();
  await page.waitForFunction(() => document.querySelector('#copy-command').title === 'Copied');
  assert.equal(await page.evaluate(() => navigator.clipboard.readText()), 'monoprint share.rs --theme night-owl --highlight 4');
  await page.waitForFunction(() => !document.querySelector('#copy-command').disabled);
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
  console.log('Native hero screenshot, CLI clipboard, six viewport sizes, local assets, publication state and support/privacy pages passed.');
} finally {
  if (browser) await browser.close();
  server.close();
}
