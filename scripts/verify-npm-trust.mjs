import { pathToFileURL } from 'node:url';
import { pkg, targets } from '../bindings/node/scripts/shared.mjs';

// Exercise the same exchange as npm publish, without uploading a package or
// persisting the short-lived credentials. Never include response bodies in errors.
export async function verifyNpmTrust({ environment = process.env, request = fetch, report = console.log } = {}) {
  if (!environment.ACTIONS_ID_TOKEN_REQUEST_URL || !environment.ACTIONS_ID_TOKEN_REQUEST_TOKEN) {
    throw new Error('Run in GitHub Actions with id-token: write');
  }
  const url = new URL(environment.ACTIONS_ID_TOKEN_REQUEST_URL);
  url.searchParams.set('audience', 'npm:registry.npmjs.org');
  const identity = await request(url, {
    headers: { Authorization: `Bearer ${environment.ACTIONS_ID_TOKEN_REQUEST_TOKEN}` },
    signal: AbortSignal.timeout(30_000),
  });
  if (!identity.ok) throw new Error(`GitHub OIDC request failed (${identity.status})`);
  const { value } = await identity.json();
  if (!value) throw new Error('GitHub did not return an OIDC identity');
  for (const name of [...targets.map(target => `${pkg.name}-${target.id}`), pkg.name]) {
    const response = await request(`https://registry.npmjs.org/-/npm/v1/oidc/token/exchange/package/${encodeURIComponent(name)}`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${value}` },
      signal: AbortSignal.timeout(30_000),
    });
    if (!response.ok) throw new Error(`npm trusted publishing rejected ${name} (${response.status})`);
    const credentials = await response.json();
    if (!credentials.token) throw new Error(`npm returned no publishing credential for ${name}`);
    report(`Verified GitHub OIDC exchange for ${name}`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await verifyNpmTrust();
