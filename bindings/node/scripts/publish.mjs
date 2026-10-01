import path from 'node:path';
import { directory, pkg, npm, readJson } from './shared.mjs';
import { validatePackages } from './release-validation.mjs';
import { checkRelease } from '../../../scripts/release-config.mjs';

const args = process.argv.slice(2);
const option = (flag, fallback) => args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback;
for (let i = 0; i < args.length; i++) {
  if (['--dry-run', '--publish', '--provenance'].includes(args[i])) continue;
  if (['--from', '--tag'].includes(args[i]) && args[i + 1] && !args[i + 1].startsWith('--')) { i++; continue; }
  throw new Error(`Unknown or incomplete publish option ${args[i]}`);
}
const source = path.resolve(option('--from', path.join(directory, '.packages')));
const manifest = await readJson(path.join(source, 'manifest.json'));
if (args.includes('--publish') && args.includes('--dry-run')) throw new Error('Choose --publish or --dry-run');
await checkRelease({ publicPublish: args.includes('--publish') });
await validatePackages(source, manifest);
const tag = option('--tag', pkg.version.includes('-') ? 'next' : 'latest');
if (!args.includes('--publish')) {
  console.log(`Validated ${manifest.tarballs.length} tarballs for tag ${tag}. No registry access or publication performed.`);
} else {
  // No lifecycle hooks: publishing never runs the NAPI CLI's implicit platform publisher.
  for (const entry of manifest.tarballs) npm(['publish', path.join(source, entry.file), '--access', 'public', '--tag', tag, '--ignore-scripts', '--registry', 'https://registry.npmjs.org', ...(args.includes('--provenance') ? ['--provenance'] : [])]);
}
