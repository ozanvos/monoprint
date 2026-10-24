import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { build } from 'esbuild';
import { spawnSync } from 'node:child_process';
import { directory, pkg, run, npm, readJson } from './shared.mjs';

const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-package-'));
try {
  const artifacts = path.join(temporary, 'packed');
  run(process.execPath, [path.join(directory, 'scripts/package.mjs'), '--local', '--out', artifacts]);
  const manifest = await readJson(path.join(artifacts, 'manifest.json'));
  assert.equal(manifest.release, false);
  const refused = spawnSync(process.execPath, [path.join(directory, 'scripts/publish.mjs'), '--from', artifacts, '--dry-run'], { encoding: 'utf8' });
  assert.notEqual(refused.status, 0);
  assert.match(refused.stderr, /complete release package set/);
  for (const tarball of manifest.tarballs) {
    for (const backend of ['takumi-raster', 'takumi-svg']) {
      for (const file of ['PATCHES.md', 'LICENSE-MIT', 'LICENSE-APACHE']) {
        assert.ok(tarball.files.includes(`vendor/${backend}/${file}`), `${tarball.name} missing ${backend}/${file}`);
      }
    }
    for (const required of ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'docs/font-fallback.md', 'assets/fonts/fallback/manifest.json', 'assets/fonts/fallback/NOTICE.txt', 'assets/fonts/fallback/noto-emoji-OFL.txt', 'assets/fonts/fallback/noto-cjk-OFL.txt', 'assets/fonts/fallback/noto-fonts-OFL.txt', 'licenses/grammars-NOTICE.txt', 'licenses/editor-themes-NOTICE.txt', 'licenses/editor-themes-provenance.json', 'assets/editor/sources.json', 'licenses/node/napi-LICENSE.txt', 'licenses/sources/giallo-0.5.2.crate', 'licenses/grammar-sources/hcl.json', 'licenses/sources/selectors-0.40.0.crate', 'licenses/sources/cssparser-0.37.0.crate']) {
      assert.ok(tarball.files.includes(required), `${tarball.name} missing ${required}`);
    }
    assert.ok(!tarball.files.some(file => file.startsWith('node_modules/') || file.startsWith('test/')));
  }
  const original = path.join(temporary, 'consumer');
  await fs.mkdir(original);
  await fs.writeFile(path.join(original, 'package.json'), JSON.stringify({ name: 'monoprint-consumer', private: true, type: 'module' }));
  npm(['install', '--offline', '--ignore-scripts', '--no-audit', '--no-fund', '--package-lock=false',
    ...manifest.tarballs.map(t => path.join(artifacts, t.file))], { cwd: original });
  const consumer = path.join(temporary, 'relocated');
  await fs.rename(original, consumer);
  const main = manifest.tarballs.find(t => t.name === pkg.name);
  assert.ok(!main.files.some(f => f.endsWith('.node')), 'Main package must not include the build host binary');
  assert.ok(main.files.includes(pkg.bin.monoprint), 'Main package must include its npm executable');
  const installed = await readJson(path.join(consumer, 'node_modules', pkg.name, 'package.json'));
  assert.deepEqual(installed.bin, { monoprint: 'dist/cli.cjs' });
  const cli = path.join(consumer, 'node_modules', pkg.name, installed.bin.monoprint);
  // npm exec uses the same bin inference as npx; run offline to forbid a registry fallback.
  const help = npm(['exec', '--offline', '--', `${pkg.name}@${pkg.version}`, '--help'], { cwd: consumer, stdio: 'pipe' }).toString();
  assert.match(help, /Usage: monoprint/);
  assert.match(help, /--highlight-text/);
  assert.equal(npm(['exec', '--offline', '--', pkg.name, '--version'], { cwd: consumer, stdio: 'pipe' }).toString().trim(), `monoprint ${pkg.version}`);
  // Pass stdin and binary stdout through the wrapper, without an external CLI on PATH.
  const cleanEnv = { ...process.env, PATH: '' };
  const image = run(process.execPath, [cli, '-', '--language', 'ts', '--scale', '1', '--format', 'png', '-o', '-'],
    { cwd: consumer, env: cleanEnv, input: 'const answer = 42;\n', stdio: ['pipe', 'pipe', 'pipe'] });
  assert.equal(image.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  const input = path.join(consumer, 'example with spaces.ts');
  const output = path.join(consumer, 'example with spaces.svg');
  await fs.writeFile(input, 'const answer = 42;\n');
  npm(['exec', '--offline', '--', pkg.name, input, '-o', output], { cwd: consumer });
  assert.match(await fs.readFile(output, 'utf8'), /<svg/);
  const invalid = spawnSync(process.execPath, [cli, '--not-a-real-option'], { cwd: consumer, env: cleanEnv, encoding: 'utf8' });
  assert.equal(invalid.status, 2, 'Preserve the Rust CLI argument-error exit code');
  assert.match(invalid.stderr, /unexpected argument/);
  const inventory = await readJson(path.join(consumer, 'node_modules', pkg.name, 'licenses/rust-dependencies.json'));
  assert.ok(inventory.some(p => p.name === 'clap'), 'Include CLI dependency notices');
  assert.ok(inventory.some(p => p.name === 'napi'), 'Keep binding dependency notices');
  const sample = `import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { Renderer, png, getThemes } from ${JSON.stringify(pkg.name)};
const cjs = createRequire(import.meta.url)(${JSON.stringify(pkg.name)});
assert.equal(cjs.Renderer, Renderer);
assert.equal(getThemes().length, 69);
const image = await png('const x = 1;', {language:'ts', theme:'catppuccin-latte', background:'halftone', fontSize:24, scale:1, compression:'balanced'});
assert.equal(image[0], 137);
assert.equal((cjs.svgSync('你好 👩🏽‍💻 🇳🇱').match(/<image/g) ?? []).length, 2);
assert.match(await new Renderer().html('مرحبا'), /font-family:'MonoprintArabic'/);
assert.match(await new Renderer().html('hello'), /^<!doctype html>/);
console.log('packed consumer passed');\n`;
  await fs.writeFile(path.join(consumer, 'consumer.mjs'), sample);
  run(process.execPath, ['consumer.mjs'], { cwd: consumer, env: cleanEnv });
  await build({ entryPoints: [path.join(consumer, 'consumer.mjs')], outfile: path.join(consumer, 'bundle.mjs'),
    platform: 'node', format: 'esm', bundle: true, external: [pkg.name, `${pkg.name}-*`] });
  run(process.execPath, ['bundle.mjs'], { cwd: consumer, env: cleanEnv });
  for (const file of ['types.mts', 'types.cts']) await fs.copyFile(path.join(directory, 'test', file), path.join(consumer, file));
  run(process.execPath, [path.join(directory, 'node_modules/typescript/bin/tsc'), '--noEmit', '--strict',
    '--module', 'NodeNext', '--moduleResolution', 'NodeNext', '--target', 'ES2022',
    '--typeRoots', path.join(directory, 'node_modules/@types'), '--types', 'node', 'types.mts', 'types.cts'], { cwd: consumer });
  // Exercise missing-binary diagnostics using the installed layout.
  const platform = manifest.tarballs.find(t => t.name !== pkg.name);
  await fs.rename(path.join(consumer, 'node_modules', platform.name), path.join(temporary, 'hidden-native-package'));
  const missingCli = spawnSync(process.execPath, [cli, '--help'], { cwd: consumer, env: cleanEnv, encoding: 'utf8' });
  assert.equal(missingCli.status, 1);
  assert.match(missingCli.stderr, /Missing Monoprint native package/);
  assert.match(missingCli.stderr, /--include=optional/);
  await fs.writeFile(path.join(consumer, 'missing.mjs'), `import assert from 'node:assert/strict';
import { png, MonoprintError } from ${JSON.stringify(pkg.name)};
await assert.rejects(png('hello'), e => e instanceof MonoprintError && e.code === 'NATIVE_BINDING_UNAVAILABLE' && !!e.cause);
`);
  run(process.execPath, ['missing.mjs'], { cwd: consumer });
  console.log('Clean tarball install, npm exec, CLI rendering, relocation, externalized bundle, declarations and missing-native diagnostics passed.');
} finally {
  await fs.rm(temporary, { recursive: true, force: true });
}
