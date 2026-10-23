import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { run } from '../../bindings/node/scripts/shared.mjs';
import { checkRelease } from '../../scripts/release-config.mjs';
import { website } from './prepare-assets.mjs';
// Website publication is separate from app and npm distribution.
const config = await checkRelease();
if (config.websitePublishing !== true || process.env.MONOPRINT_WEBSITE_PUBLISH !== 'approved') {
  throw new Error('Website publishing is disabled. Enable release/config.json and explicitly authorize this invocation with MONOPRINT_WEBSITE_PUBLISH=approved.');
}
// Deploy only this upload, preserving the existing custom domain and triggers.
// This permits a token with Editor access to monoprint-site alone.
const wrangler = path.join(website, 'node_modules/wrangler/bin/wrangler.js');
const tag = randomUUID();
run(process.execPath, [wrangler, 'versions', 'upload', '--tag', tag], { cwd: website });
run(process.execPath, [wrangler, 'versions', 'deploy', '--version-tag', tag,
  '--percentage', '100', '--yes'], { cwd: website });
