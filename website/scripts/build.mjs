import path from 'node:path';
import { run } from '../../bindings/node/scripts/shared.mjs';
import { prepareAssets, website } from './prepare-assets.mjs';
await prepareAssets();
run(process.execPath, [path.join(website, 'node_modules/astro/bin/astro.mjs'), 'build'], { cwd: website });
