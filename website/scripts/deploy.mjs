import path from 'node:path';
import { run } from '../../bindings/node/scripts/shared.mjs';
import { checkRelease } from '../../scripts/release-config.mjs';
import { website } from './prepare-assets.mjs';
// Website publication is separate from app and npm distribution.
const config = await checkRelease();
if (config.websitePublishing !== true || process.env.MONOPRINT_WEBSITE_PUBLISH !== 'approved') {
  throw new Error('Website publishing is disabled. Enable release/config.json and explicitly authorize this invocation with MONOPRINT_WEBSITE_PUBLISH=approved.');
}
run(process.execPath, [path.join(website, 'node_modules/wrangler/bin/wrangler.js'), 'deploy'], { cwd: website });
