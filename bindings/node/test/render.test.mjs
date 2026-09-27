import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { Font, Renderer, MonoprintError, png, pngSync, svg, html, htmlSync, getThemes, getLanguages, languageForFileName } from '../dist/index.mjs';

const source = 'const answer = 42;\nconsole.log(answer);\n';
const options = { language: 'typescript', theme: 'removed-frame', padding: 16, width: 400, highlight: [1, [2, 3]] };
const code = expected => error => error instanceof MonoprintError && error.code === expected;

test('async and sync exports return the same owned PNG / SVG / HTML data', async () => {
  const renderer = new Renderer();
  const [p, s, h] = await Promise.all([renderer.png(source, { ...options, scale: 2 }), renderer.svg(source, options), renderer.html(source, options)]);
  assert.ok(Buffer.isBuffer(p));
  assert.deepEqual([...p.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.deepEqual(p, renderer.pngSync(source, { ...options, scale: 2 }));
  assert.equal(s, renderer.svgSync(source, options));
  assert.equal(h, renderer.htmlSync(source, options));
  assert.match(s, /<svg/);
  assert.match(h, /^<!doctype html>/);
  assert.doesNotMatch(h, /<script[\s>]/i);
  p.fill(0);
  assert.equal((await renderer.png(source, { ...options, scale: 2 }))[0], 137);
});

test('ESM and CommonJS share class identity, errors, fonts and convenience methods', async () => {
  const require = createRequire(import.meta.url);
  const cjs = require('../dist/index.js');
  assert.equal(cjs.Renderer, Renderer);
  assert.equal(cjs.Font, Font);
  assert.equal(cjs.MonoprintError, MonoprintError);
  assert.equal(await cjs.html(source, options), htmlSync(source, options));
});

test('styles are reusable snapshots and explicit auto/null restore defaults', async () => {
  const style = { theme: 'removed-frame', width: 420, padding: 24 };
  const renderer = new Renderer(style);
  style.theme = 'removed-frame';
  const highlights = [1, [2, 2]];
  const request = { ...options, theme: 'removed-frame', highlight: highlights };
  const pending = renderer.html(source, request);
  request.theme = 'removed-frame';
  highlights[1][1] = 999;
  assert.equal(await pending, htmlSync(source, { ...options, theme: 'removed-frame', highlight: [1, [2, 2]] }));
  assert.equal(await renderer.html(source, { width: 'auto', padding: 64, font: null }),
    await html(source, { theme: 'removed-frame', width: 'auto', padding: 64 }));
});

test('catalog discovery is immutable, canonical, and follows reviewed grammar exclusions', () => {
  assert.equal(getThemes().length, 33);
  assert.equal(getLanguages().length, 230);
  assert.deepEqual(getThemes().find(t => t.id === 'removed-frame').appearances, ['dark']);
  assert.ok(getLanguages().some(l => l.id === 'tsx' && l.aliases.includes('typescript')));
  assert.ok(!getLanguages().some(l => l.id === 'nginx'));
  assert.equal(languageForFileName('src/main.rs'), 'rust');
  assert.equal(languageForFileName('example.mts'), 'tsx');
  assert.equal(languageForFileName('Dockerfile'), 'docker');
  assert.equal(languageForFileName('unknown.xyzzy'), undefined);
  assert.ok(Object.isFrozen(getThemes()[0].appearances));
  assert.ok(Object.isFrozen(getLanguages()[0].aliases));
});

test('all theme appearances work through the TypeScript interface', async () => {
  for (const theme of getThemes()) {
    for (const appearance of theme.appearances) {
      assert.match(await html('let x = 1;', { language: 'rust', theme: theme.id, appearance }), /^<!doctype html>/);
    }
  }
});

test('custom font bytes are copied and a font can be reused across renderers', async () => {
  const bytes = await readFile(new URL('../../../assets/fonts/jetbrains-mono.ttf', import.meta.url));
  const font = Font.fromBytes(bytes);
  const renderer = new Renderer({ font });
  const expected = await renderer.svg(source, options);
  bytes.fill(0);
  assert.equal(await new Renderer({ font }).svg(source, options), expected);
  assert.equal(await renderer.svg(source, { ...options, font: null }), await svg(source, options));
  assert.throws(() => Font.fromBytes(new Uint8Array(16)), code('INVALID_FONT'));
});

test('validation rejects invalid JS inputs without numeric coercion or ignored options', async () => {
  for (const bad of [NaN, Infinity, -1, 1.5, 2 ** 32, '32', null]) {
    await assert.rejects(png(source, { padding: bad }), code('INVALID_ARGUMENT'));
  }
  await assert.rejects(svg(source, { scale: 2 }), code('INVALID_ARGUMENT'));
  await assert.rejects(png(source, { them: 'removed-frame' }), code('INVALID_ARGUMENT'));
  await assert.rejects(html(source, null), code('INVALID_ARGUMENT'));
  await assert.rejects(html(123), code('INVALID_ARGUMENT'));
  await assert.rejects(html(source, { font: {} }), code('INVALID_ARGUMENT'));
  await assert.rejects(html(source, { highlight: [[1, 2, 3]] }), code('INVALID_ARGUMENT'));
  await assert.rejects(html(source, { highlight: new Array(1) }), code('INVALID_ARGUMENT'));
  await assert.rejects(html(source, { highlight: [new Array(2)] }), code('INVALID_ARGUMENT'));
  assert.throws(() => new Renderer({ scale: 2 }), code('INVALID_ARGUMENT'));
  assert.throws(() => pngSync(source, { padding: -1 }), code('INVALID_ARGUMENT'));
});

test('native failures expose stable codes and structured context, then renderer recovers', async () => {
  const renderer = new Renderer();
  await assert.rejects(renderer.html(source, { theme: 'missing' }), code('UNKNOWN_THEME'));
  await assert.rejects(renderer.html(source, { language: 'nginx' }), code('UNKNOWN_LANGUAGE'));
  await assert.rejects(renderer.html(source, { theme: 'removed-frame', appearance: 'light' }), error => code('INVALID_OPTION')(error) && error.details.option === 'appearance');
  await assert.rejects(renderer.html(source, { highlight: [0] }), error => code('INVALID_OPTION')(error) && error.details.lineCount === 3);
  await assert.rejects(renderer.png(source, { scale: 0 }), code('INVALID_OPTION'));
  await assert.rejects(renderer.png(source, { width: 8192, padding: 0, scale: 6 }), code('IMAGE_TOO_LARGE'));
  await assert.rejects(renderer.html('é'.repeat(524289)), error => code('INVALID_OPTION')(error) && error.details.option === 'source');
  assert.match(await renderer.html(source), /^<!doctype html>/);
});

test('async rendering leaves the event loop responsive and retains dropped renderers', async () => {
  let ticks = 0;
  const timer = setInterval(() => ticks++, 1);
  try {
    const result = await new Renderer().png(source.repeat(50), { language: 'ts', width: 600, scale: 2 });
    assert.equal(result[0], 137);
    assert.ok(ticks > 0, 'rendering should run off the calling JS thread');
  } finally { clearInterval(timer); }
});

test('TypeScript results are byte-identical to the Rust CLI', { skip: !process.env.MONOPRINT_CLI }, async () => {
  const renderer = new Renderer();
  for (const theme of ['removed-frame', 'removed-frame', 'removed-frame', 'removed-frame', 'removed-frame']) {
    for (const format of ['png', 'svg', 'html']) {
      const args = ['-l', 'tsx', '--theme', theme, '--padding', '16', '--width', '400', '--highlight', '1', '--scale', '2', '--format', format, '-o', '-'];
      const cli = spawnSync(process.env.MONOPRINT_CLI, args, { input: source, maxBuffer: 16 * 1024 * 1024 });
      assert.equal(cli.status, 0, cli.stderr?.toString());
      const options = { theme, language: 'tsx', padding: 16, width: 400, highlight: [1], ...(format === 'png' ? { scale: 2 } : {}) };
      assert.deepEqual(Buffer.from(await renderer[format](source, options)), cli.stdout, `${theme}/${format}`);
    }
  }
});
