import path from 'node:path';
import { run } from '../../bindings/node/scripts/shared.mjs';
import { checkRelease } from '../../scripts/release-config.mjs';
import { website } from './prepare-assets.mjs';
// Keep the same explicit publication boundary as the app and npm releases.
await checkRelease({ publicPublish: true });
run(process.execPath, [path.join(website, 'node_modules/wrangler/bin/wrangler.js'), 'deploy'], { cwd: website });
