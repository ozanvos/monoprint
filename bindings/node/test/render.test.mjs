import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { Font, Renderer, MonoprintError, png, pngSync, svg, html, htmlSync, getThemes, getLanguages, languageForFileName } from '../dist/index.mjs';

const source = 'const answer = 42;\nconsole.log(answer);\n';
const options = { language: 'typescript', theme: 'removed-frame', padding: 16, width: 400, highlight: [1, [2, 3]] };
const code = expected => error => error instanceof MonoprintError && error.code === expected;

test('line numbers inherit theme defaults and can be overridden for every theme', () => {
  assert.ok(getThemes().every(theme => theme.lineNumbers.toggleable));
  const renderer = new Renderer();
  const visible = options => renderer.htmlSync(source, { language: 'typescript', ...options }).includes('class="Editor__editor Editor__showLineNumbers');
  for (const [theme, defaultVisible] of [['removed-frame', true], ['removed-frame', true], ['removed-frame', false], ['removed-frame', false]]) {
    assert.equal(visible({ theme }), defaultVisible);
    assert.equal(visible({ theme, lineNumbers: 'auto' }), defaultVisible);
    assert.equal(visible({ theme, lineNumbers: false }), false);
    assert.equal(visible({ theme, lineNumbers: true }), true);
  }
});

test('font size changes layout independently of PNG scale and preserves the default', async () => {
  const source = 'const answer = 42;\n'.repeat(6);
  const renderer = new Renderer({ fontSize: 24 });
  const base = { language: 'typescript', scale: 2 };
  const defaultImage = pngSync(source, base);
  assert.deepEqual(defaultImage, renderer.pngSync(source, { ...base, fontSize: 15 }));
  for (const format of ['svg', 'html']) {
    assert.equal(await renderer[format](source, { fontSize: 15 }), await new Renderer()[format](source));
  }
  const small = await renderer.png(source, { ...base, fontSize: 8 });
  const large = await renderer.png(source, base);
  const largest = await renderer.png(source, { ...base, fontSize: 72 });
  const height = image => image.readUInt32BE(20);
  assert.ok(height(small) < height(defaultImage));
  assert.ok(height(defaultImage) < height(large));
  assert.ok(height(large) < height(largest));
  assert.deepEqual(large, renderer.pngSync(source, base), 'per-render size overrides must not change defaults');
  assert.equal(height(await renderer.png(source, { ...base, scale: 4 })), height(large) * 2);
  for (const fontSize of [0, 7, 73, 0xffffffff]) {
    await assert.rejects(renderer.html(source, { fontSize }), error => {
      assert.ok(code('INVALID_OPTION')(error));
      assert.deepEqual(error.details, { option: 'fontSize', value: fontSize, min: 8, max: 72 });
      return true;
    });
  }
  for (const fontSize of ['18', 14.5, NaN, Infinity, -1, null]) {
    assert.throws(() => new Renderer({ fontSize }), code('INVALID_ARGUMENT'));
  }
});

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
  assert.equal(getThemes().length, 54);
  assert.equal(getLanguages().length, 230);
  assert.deepEqual(getThemes().find(t => t.id === 'removed-frame').appearances, ['dark']);
  assert.ok(getLanguages().some(l => l.id === 'tsx' && l.aliases.includes('typescript')));
  assert.ok(!getLanguages().some(l => l.id === 'nginx'));
  assert.equal(languageForFileName('src/main.rs'), 'rust');
  assert.equal(languageForFileName('example.mts'), 'tsx');
  assert.equal(languageForFileName('Dockerfile'), 'docker');
  assert.equal(languageForFileName('unknown.xyzzy'), undefined);
  assert.ok(Object.isFrozen(getThemes()[0].appearances));
  assert.ok(Object.isFrozen(getThemes()[0].aliases));
  assert.ok(Object.isFrozen(getLanguages()[0].aliases));
});

test('all theme appearances work through the TypeScript interface', async () => {
  for (const theme of getThemes()) {
    for (const appearance of theme.appearances) {
      assert.match(await html('let x = 1;', { language: 'rust', theme: theme.id, appearance }), /^<!doctype html>/);
    }
  }
});

test('editor palettes select their appearance and background overrides replace the legacy alias', () => {
  const catppuccin = getThemes().find(t => t.id === 'catppuccin');
  assert.equal(catppuccin.family, 'Catppuccin');
  assert.deepEqual(catppuccin.appearances, ['dark', 'light']);
  assert.ok(catppuccin.aliases.includes('catppuccin-latte'));
  const theme = 'catppuccin-latte';
  assert.match(htmlSync(source, { theme }), /data-theme="light"/);
  assert.equal(htmlSync(source, { theme }), htmlSync(source, { theme: 'catppuccin', appearance: 'light' }));
  assert.equal(htmlSync(source, { theme, appearance: 'dark' }), htmlSync(source, { theme: 'catppuccin-mocha' }));
  const renderer = new Renderer({ transparent: true, theme });
  for (const background of ['theme', 'solid', 'lattice', 'halftone', 'none']) {
    assert.equal(renderer.htmlSync(source, { background }), htmlSync(source, { theme, background }));
  }
  assert.equal(new Renderer({ background: 'lattice' }).htmlSync(source, { transparent: true }),
    htmlSync(source, { background: 'none' }));
  assert.throws(() => htmlSync(source, { background: 'wallpaper' }), code('INVALID_ARGUMENT'));
  assert.throws(() => htmlSync(source, { background: 'solid', transparent: true }), code('INVALID_ARGUMENT'));
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

test('PNG compression is opt-in, with async/sync parity', async () => {
  const opts = { ...options, scale: 2 };
  const fast = await png(source, { ...opts, compression: 'fast' });
  assert.deepEqual(fast, pngSync(source, opts));
  const balanced = await png(source, { ...opts, compression: 'balanced' });
  assert.deepEqual(balanced, pngSync(source, { ...opts, compression: 'balanced' }));
  assert.ok(balanced.length < fast.length);
  for (const bad of ['smallest', 6, null, true]) {
    await assert.rejects(png(source, { compression: bad }), code('INVALID_ARGUMENT'));
    assert.throws(() => pngSync(source, { compression: bad }), code('INVALID_ARGUMENT'));
  }
  await assert.rejects(svg(source, { compression: 'fast' }), code('INVALID_ARGUMENT'));
  await assert.rejects(html(source, { compression: 'balanced' }), code('INVALID_ARGUMENT'));
  assert.throws(() => new Renderer({ compression: 'fast' }), code('INVALID_ARGUMENT'));
});

test('both PNG compression modes match the CLI', { skip: !process.env.MONOPRINT_CLI }, async () => {
  for (const compression of ['fast', 'balanced']) {
    const cli = spawnSync(process.env.MONOPRINT_CLI,
      ['-l', 'tsx', '--png-compression', compression, '--scale', '2', '--format', 'png', '-o', '-'],
      { input: source, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(cli.status, 0, cli.stderr?.toString());
    assert.deepEqual(await png(source, { language: 'tsx', compression, scale: 2 }), cli.stdout);
  }
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
  const wrapped = Array.from({ length: 30 }, (_, i) =>
    `let value_${String(i).padStart(3, '0')} = "hello ${i}"; // benchmark code image `.padEnd(80, '.')
  ).join('\n');
  await assert.rejects(renderer.png(wrapped, { language: 'rust', theme: 'removed-frame', width: 896, scale: 4, lineNumbers: true }),
    error => code('IMAGE_TOO_LARGE')(error) && error.details.width === 3584 && error.details.height === 6184);
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
  for (const theme of ['removed-frame', 'removed-frame', 'removed-frame', 'removed-frame', 'removed-frame', 'ayu', 'ayu-light', 'solarized', 'solarized-light']) {
    for (const format of ['png', 'svg', 'html']) {
      const args = ['-l', 'tsx', '--theme', theme, '--padding', '16', '--width', '400', '--highlight', '1', '--scale', '2', '--format', format, '-o', '-'];
      const cli = spawnSync(process.env.MONOPRINT_CLI, args, { input: source, maxBuffer: 16 * 1024 * 1024 });
      assert.equal(cli.status, 0, cli.stderr?.toString());
      const options = { theme, language: 'tsx', padding: 16, width: 400, highlight: [1], ...(format === 'png' ? { scale: 2 } : {}) };
      assert.deepEqual(Buffer.from(await renderer[format](source, options)), cli.stdout, `${theme}/${format}`);
    }
  }
});

test('custom font size matches the CLI in all formats', { skip: !process.env.MONOPRINT_CLI }, async () => {
  const renderer = new Renderer({ fontSize: 24 });
  for (const format of ['png', 'svg', 'html']) {
    const cli = spawnSync(process.env.MONOPRINT_CLI,
      ['-l', 'tsx', '--font-size', '24', '--scale', '2', '--format', format, '-o', '-'],
      { input: source, maxBuffer: 16 * 1024 * 1024 });
    assert.equal(cli.status, 0, cli.stderr?.toString());
    const options = { language: 'tsx', ...(format === 'png' ? { scale: 2 } : {}) };
    assert.deepEqual(Buffer.from(await renderer[format](source, options)), cli.stdout, format);
  }
});


test('emoji sequences and glyph fallback are automatic across Node and CLI', async () => {
  const source = '// 中文 日本語 한국어 مرحبا नमस्ते שלום สวัสดี\nconst emoji = "👩🏽‍💻 🇳🇱 ❤️ 1️⃣";';
  const renderer = new Renderer();
  const options = { language: 'tsx', width: 900, title: '你好 🦀.tsx', fontSize: 20 };
  const before = renderer.htmlSync('plain ASCII');
  for (const format of ['png', 'svg', 'html']) {
    const opts = { ...options, ...(format === 'png' ? { scale: 2 } : {}) };
    const value = await renderer[format](source, opts);
    assert.deepEqual(value, renderer[`${format}Sync`](source, opts));
    if (format === 'svg') assert.ok((value.match(/<image/g) ?? []).length >= 4);
    if (format === 'html') {
      for (const family of ['Emoji', 'Cjk', 'Arabic', 'Devanagari', 'Hebrew', 'Thai']) {
        assert.ok(value.includes(`font-family:'Monoprint${family}'`));
      }
    }
    if (process.env.MONOPRINT_CLI) {
      const cli = spawnSync(process.env.MONOPRINT_CLI,
        ['-l', 'tsx', '--width', '900', '--title', options.title, '--font-size', '20', '--scale', '2', '--format', format, '-o', '-'],
        { input: source, maxBuffer: 64 * 1024 * 1024 });
      assert.equal(cli.status, 0, cli.stderr?.toString());
      assert.deepEqual(Buffer.from(value), cli.stdout, `fallback ${format}`);
    }
  }
  assert.equal(renderer.htmlSync('plain ASCII'), before);
  await assert.rejects(renderer.html('\u{10ffff}'), error =>
    code('UNSUPPORTED_GLYPH')(error) && error.details.character === '\u{10ffff}');
});
