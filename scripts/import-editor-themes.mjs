// Maintenance only. Fetch exact reviewed inputs, retain licenses, and compile
// inert palette JSON. The renderer never executes an extension or fetches data.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';

const configPath = 'assets/editor/sources.json';
const receiptPath = 'licenses/editor-themes-provenance.json';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
if (process.argv.includes('--check')) {
  const receipt = JSON.parse(await fs.readFile(receiptPath, 'utf8'));
  for (const [file, expected] of Object.entries(receipt.checksums))
    assert.equal(digest(await fs.readFile(file)), expected, `Changed theme input: ${file}; reimport and rebuild the registry`);
  const manifest = JSON.parse(await fs.readFile('assets/editor/themes.json', 'utf8'));
  assert.equal(Object.keys(manifest).length, receipt.entries.length);
  for (const entry of receipt.entries) assert.ok(manifest[entry.id]);
  console.log(`${receipt.entries.length} editor palettes: source/license checksums verified`);
} else {
  const { parse } = await import('jsonc-parser');
  const { parse: color, formatHex8 } = await import('culori');
  const config = JSON.parse(await fs.readFile(configPath, 'utf8'));
  const cache = process.argv[2] || '/tmp/monoprint-editor-themes';
  await fs.mkdir(cache, { recursive: true });
  async function input(spec) {
    assert.match(spec.url, /^https:\/\/raw\.githubusercontent\.com\/[^/]+\/[^/]+\/[a-f0-9]{40}\//);
    assert.match(spec.sha256, /^[a-f0-9]{64}$/);
    const file = path.join(cache, spec.sha256);
    let bytes = await fs.readFile(file).catch(() => null);
    if (!bytes) {
      const response = await fetch(spec.url);
      assert.ok(response.ok, `${response.status}: ${spec.url}`);
      bytes = Buffer.from(await response.arrayBuffer());
      assert.equal(digest(bytes), spec.sha256, `Upstream bytes changed: ${spec.url}`);
      await fs.writeFile(file, bytes);
    }
    assert.equal(digest(bytes), spec.sha256, `Input changed: ${spec.url}`);
    return bytes.toString('utf8');
  }
  const hex = value => {
    const parsed = color(value);
    assert.ok(parsed, `Invalid theme color ${value}`);
    return formatHex8(parsed);
  };
  const mix = (a, b, amount) => {
    const rgb = h => [1, 3, 5].map(i => parseInt(hex(h).slice(i, i + 2), 16));
    const aa = rgb(a), bb = rgb(b);
    return '#' + aa.map((c, i) => Math.round(c + (bb[i] - c) * amount).toString(16).padStart(2, '0')).join('');
  };
  const manifest = {}, checksums = {}, entries = [], notices = new Map();
  async function write(file, bytes) {
    await fs.writeFile(file, bytes);
    checksums[file] = digest(bytes);
  }
  for (const entry of config.entries) {
    const raw = await input(entry.input), errors = [];
    const theme = parse(raw, errors, { allowTrailingComma: true });
    assert.deepEqual(errors, [], `Invalid JSONC: ${entry.id}`);
    assert.ok(!theme.include, `Resolve included theme first: ${entry.id}`);
    assert.ok(Array.isArray(theme.tokenColors), `Missing token rules: ${entry.id}`);
    const global = theme.tokenColors.find(t => !t.scope)?.settings || {};
    const colors = theme.colors || {};
    const surface = hex(colors['editor.background'] || global.background || theme.bg);
    const foreground = hex(colors['editor.foreground'] || global.foreground || theme.fg || colors.foreground);
    const mode = entry.appearance || theme.type;
    assert.ok(['light', 'dark'].includes(mode), `Missing appearance: ${entry.id}`);
    const light = mode === 'light';
    const gutter = hex(colors['editorLineNumber.foreground'] || mix(surface, foreground, .48));
    const selection = hex(colors['editor.selectionBackground'] || mix(surface, foreground, .15));
    const rules = theme.tokenColors.map(rule => {
      const settings = { ...rule.settings };
      for (const key of ['foreground', 'background'])
        if (settings[key] && settings[key] !== 'inherit') settings[key] = hex(settings[key]);
      return { ...(rule.scope !== undefined ? { scope: rule.scope } : {}), settings };
    });
    const normalized = { name: `editor-${entry.id}`, type: mode,
      colors: { 'editor.foreground': foreground, 'editor.background': surface }, tokenColors: rules };
    await write(`assets/themes/editor-${entry.id}.json`, JSON.stringify(normalized) + '\n');
    manifest[entry.id] = { id: entry.id, name: entry.name, font: 'jetbrains-mono',
      syntax: { [mode]: { '--ink-foreground': foreground, '--ink-highlight': selection } },
      background: { from: mix(surface, entry.accents[0], light ? .25 : .38), to: mix(surface, entry.accents[1], light ? .18 : .22) },
      editor: { family: entry.family, surface, foreground, gutter,
        border: mix(surface, foreground, .15), solid: mix(surface, entry.accents[0], light ? .12 : .14) } };
    const license = await input(entry.license);
    assert.match(license, /Permission is hereby granted/);
    notices.set(entry.license.url, license);
    entries.push({ id: entry.id, upstream: entry.upstream, input: entry.input, license: 'MIT', licenseSource: entry.license,
      ...(entry.appearance ? { modification: `Correct upstream type=${theme.type} to ${entry.appearance} for the light palette` } : {}) });
  }
  for (const extra of config.extraNotices) notices.set(extra.url, await input(extra));
  // Record the exact source of the small, manually adapted token glow mapping.
  await input(config.glow);
  await write('assets/editor/themes.json', JSON.stringify(manifest, null, 2) + '\n');
  await write('licenses/editor-themes-NOTICE.txt', 'MONOPRINT EDITOR THEME NOTICES\n\nOriginal backgrounds and frame: Apache-2.0, Monoprint contributors.\nTheme inputs retain the licenses below. No upstream logos or wallpapers included.\n\n' + [...notices].map(([url, text]) => `${url}\n${text}\n`).join('\n---\n\n'));
  for (const file of [configPath, 'scripts/import-editor-themes.mjs']) checksums[file] = digest(await fs.readFile(file));
  await fs.writeFile(receiptPath, JSON.stringify({ entries, glow: config.glow, checksums }, null, 2) + '\n');
  console.log(`Imported ${entries.length} editor palettes. Rebuild the Giallo registry next.`);
}
