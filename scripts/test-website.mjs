// Static-site checks using the repository's existing Playwright tooling.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { chromium } from 'playwright';

const args = process.argv.slice(2);
const option = flag => args[args.indexOf(flag) + 1];
if (args.length !== 4 || !args.includes('--from') || !args.includes('--out')) throw new Error('Usage: node scripts/test-website.mjs --from WEBSITE --out SCREENSHOTS');
const source = path.resolve(option('--from'));
const out = path.resolve(option('--out'));
await fs.mkdir(out, { recursive: true });
const types = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.png': 'image/png', '.ttf': 'font/ttf', '.otf': 'font/otf' };
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
  const page = await browser.newPage();
  const errors = [];
  const network = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (new URL(route.request().url()).origin !== url) { network.push(route.request().url()); return route.abort(); }
    return route.continue();
  });
  await page.goto(url);
  await page.evaluate(() => document.fonts.ready);
  assert.equal(await page.locator('meta[name=robots]').getAttribute('content'), 'noindex,nofollow');
  assert.equal(await page.getByRole('link', { name: /Download/i }).count(), 0);
  for (const [width, height] of [[1440, 1100], [390, 844]]) {
    await page.setViewportSize({ width, height });
    for (const label of ['Gruvbox', 'GitHub light', 'Candy', 'Nord']) {
      await page.getByRole('button', { name: label, exact: true }).click();
      assert.equal(await page.getByRole('button', { name: label, exact: true }).getAttribute('aria-pressed'), 'true');
      await page.waitForFunction(() => [...document.images].every(img => img.complete && img.naturalWidth > 0));
    }
    for (const label of ['Rust', 'TypeScript', 'CLI']) {
      await page.getByRole('button', { name: label, exact: true }).click();
      assert.equal(await page.getByRole('button', { name: label, exact: true }).getAttribute('aria-pressed'), 'true');
      assert.ok((await page.locator('#example').textContent()).length > 40);
    }
    assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), `Overflow at ${width}px`);
    await page.getByRole('button', { name: 'Gruvbox', exact: true }).click();
    await page.evaluate(() => scrollTo(0, 0));
    await page.screenshot({ path: path.join(out, `website-${width}.png`), fullPage: true });
  }
  assert.deepEqual(errors, []);
  assert.deepEqual(network, []);
  console.log('Website controls, local assets, mobile/desktop layout and private-preview state passed.');
} finally {
  if (browser) await browser.close();
  server.close();
}
