# Monoprint for Node.js

Render code as PNG, SVG or HTML with the Monoprint Rust renderer.
Prebuilt packages target Node.js 22+ on macOS arm64/x64, Linux glibc arm64/x64
and Windows x64.

## CLI

The npm package includes the same native CLI as the standalone download:

```sh
npx monoprint@latest
npx monoprint@latest example.ts -o example.png
```

Or install with `npm install -g monoprint` and run `monoprint` directly.
Running without arguments shows help.
Keep npm's optional dependencies enabled; they contain the platform's CLI and
Node binding. No Rust toolchain or install script is needed.
See the [CLI reference](https://github.com/ozanvos/monoprint/blob/main/docs/cli.md) for all options.

## Node.js

```sh
npm install monoprint
```

```ts
import { writeFile } from 'node:fs/promises';
import { png } from 'monoprint';

const image = await png('const answer = 42;', {
  language: 'typescript',
  theme: 'monoprint',
  padding: 32,
  fontSize: 18,
  scale: 2,
});
await writeFile('answer.png', image);
```

## API

| Function | Returns |
| --- | --- |
| `png(source, options?)` | `Promise<Buffer>` |
| `svg(source, options?)` | `Promise<string>` |
| `html(source, options?)` | `Promise<string>` |
| `pngSync(source, options?)` | `Buffer` |
| `svgSync`, `htmlSync` | `string` |

Async methods run on libuv's worker pool. Sync methods block the caller.
Use `Renderer` to reuse styles and caches:

```ts
import { Renderer, languageForFileName } from 'monoprint';

const renderer = new Renderer({ theme: 'monoprint', padding: 24, width: 640 });
const svg = await renderer.svg('fn main() {}', {
  language: languageForFileName('main.rs') ?? 'plain',
  title: 'main.rs',
  highlight: [1],
});
```

`getThemes()` and `getLanguages()` return the available IDs and aliases.
`Font.fromBytes()` loads a single-face TTF/OTF. Rendering does not read files,
load system fonts or access the network. The caller handles file IO.
See the [API reference](https://github.com/ozanvos/monoprint/blob/main/docs/node-api.md) for options, defaults, fonts,
errors, limits and server bundling.

## Build

From `bindings/node/`, with Rust 1.97+, a C compiler and Node.js 22+:

```sh
npm ci
npm run build
npm test
npm run test:types
npm run test:package
```

TypeScript consumers need `@types/node`. Prebuilt installations need no Rust
toolchain or install script. Keep the native package external in server bundles;
browser and edge runtimes are unsupported.

See [package builds and publishing](https://github.com/ozanvos/monoprint/blob/main/docs/node-packaging.md) for tarballs,
the platform matrix and release commands. Original code is Apache-2.0;
[third-party licenses](https://github.com/ozanvos/monoprint/blob/main/THIRD_PARTY_NOTICES.md) accompany the packages.
