import fs from 'node:fs/promises';
import path from 'node:path';
import { build } from 'esbuild';
import { directory, root, pkg, targets, run, readJson, sha256, inputHash, revision, isDirty } from './shared.mjs';

const args = process.argv.slice(2);
if (args.some((arg, index) => !['--js-only', '--target'].includes(arg) && args[index - 1] !== '--target')) throw new Error('Usage: npm run build -- [--js-only] [--target RUST_TRIPLE]');
const pairs = await readJson(path.join(root, 'assets/editor/pairs.json'));
const themes = [...new Set([...Object.keys({ ...await readJson(path.join(root, 'assets/editor/themes.json')),
  ...await readJson(path.join(root, 'assets/originals/themes.json')),
  ...await readJson(path.join(root, 'assets/selected/themes.json')),
  ...await readJson(path.join(root, 'assets/code-styles/themes.json')) }), ...pairs.map(pair => pair.id)])].sort();
const languages = (await readJson(path.join(root, 'assets/grammar-metadata.json')))
  .map(g => g.name).filter(n => n !== 'tex').concat('plain').sort();
const union = values => values.map(v => `  | ${JSON.stringify(v)}`).join('\n');
await fs.writeFile(path.join(directory, 'js/catalog-types.ts'),
  `// Generated from the reviewed Rust assets by scripts/build.mjs.\nexport type ThemeId =\n${union(themes)};\n\nexport type LanguageId =\n${union(languages)};\n`);
await fs.mkdir(path.join(directory, 'dist'), { recursive: true });
await build({ entryPoints: [path.join(directory, 'js/index.ts')], outfile: path.join(directory, 'dist/index.js'),
  platform: 'node', format: 'cjs', target: 'node22', bundle: true, sourcemap: true,
  external: ['./loader.cjs'], legalComments: 'none' });
run(process.execPath, [path.join(directory, 'node_modules/typescript/bin/tsc'), '-p', path.join(directory, 'tsconfig.json')]);
await fs.copyFile(path.join(directory, 'js/loader.cjs'), path.join(directory, 'dist/loader.cjs'));
await fs.writeFile(path.join(directory, 'dist/index.mjs'),
  "export { Renderer, Font, MonoprintError, limits, png, svg, html, pngSync, svgSync, htmlSync, getThemes, getLanguages, languageForFileName } from './index.js';\n");
await fs.writeFile(path.join(directory, 'dist/index.d.mts'), "export * from './index.js';\n");
if (!args.includes('--js-only')) {
  const commit = revision();
  const hash = await inputHash();
  const dirty = isDirty();
  const explicit = args.indexOf('--target');
  const triple = explicit < 0 ? run('rustc', ['-vV'], { stdio: 'pipe' }).toString().match(/^host: (.+)$/m)[1] : args[explicit + 1];
  const target = targets.find(t => t.triple === triple);
  if (!target) throw new Error(`Unsupported native build target ${triple}`);
  const manifest = await fs.readFile(path.join(directory, 'Cargo.toml'), 'utf8');
  if (!manifest.includes(`version = "${pkg.version}"`)) throw new Error('Keep the npm and monoprint-node Cargo versions equal');
  run('cargo', ['build', '--locked', '--release', '-p', 'monoprint-node', ...(explicit < 0 ? [] : ['--target', triple])]);
  const metadata = JSON.parse(run('cargo', ['metadata', '--locked', '--offline', '--no-deps', '--format-version', '1'], { stdio: 'pipe' }));
  const extension = target.os === 'darwin' ? 'dylib' : target.os === 'win32' ? 'dll' : 'so';
  const library = `${target.os === 'win32' ? '' : 'lib'}monoprint_node.${extension}`;
  const bytes = await fs.readFile(path.join(metadata.target_directory, ...(explicit < 0 ? [] : [triple]), 'release', library));
  const native = path.join(directory, 'native');
  if (hash !== await inputHash() || commit !== revision()) throw new Error('Native source changed while building; rebuild the addon');
  await fs.mkdir(native, { recursive: true });
  await fs.writeFile(path.join(native, `monoprint.${target.id}.node`), bytes);
  await fs.writeFile(path.join(native, `monoprint.${target.id}.json`), JSON.stringify({
    target: target.id, triple, version: pkg.version, sha256: sha256(bytes), inputHash: hash,
    commit, dirty: dirty || isDirty(), compiler: run('rustc', ['-vV'], { stdio: 'pipe' }).toString().trim(),
  }, null, 2) + '\n');
  console.log(`Built ${target.id}: ${(bytes.length / 1048576).toFixed(2)} MiB`);
}
