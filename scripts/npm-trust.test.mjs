import assert from 'node:assert/strict';
import test from 'node:test';
import { verifyNpmTrust } from './verify-npm-trust.mjs';

const environment = { ACTIONS_ID_TOKEN_REQUEST_URL: 'https://example.invalid/token?existing=1', ACTIONS_ID_TOKEN_REQUEST_TOKEN: 'runner-secret' };
test('fails without a GitHub OIDC identity', async () => {
  await assert.rejects(verifyNpmTrust({ environment: {} }), /id-token: write/);
});
test('exchanges credentials for every package without logging secrets', async () => {
  const reports = [];
  const calls = [];
  await verifyNpmTrust({ environment, report: value => reports.push(value), request: async (url, options) => {
    calls.push([String(url), options]);
    return { ok: true, json: async () => calls.length === 1 ? { value: 'identity-secret' } : { token: 'npm-secret' } };
  } });
  assert.equal(new URL(calls[0][0]).searchParams.get('audience'), 'npm:registry.npmjs.org');
  assert.equal(calls.length, 7);
  assert.equal(new Set(calls.slice(1).map(([url]) => url)).size, 6);
  for (const [, options] of calls.slice(1)) {
    assert.equal(options.method, 'POST');
    assert.equal(options.headers.Authorization, 'Bearer identity-secret');
  }
  assert.equal(reports.length, 6);
  assert.ok(!reports.join('\n').includes('secret'));
});
test('rejects a mismatched trust relationship without leaking the response', async () => {
  let calls = 0;
  await assert.rejects(verifyNpmTrust({ environment, request: async () => ++calls === 1
    ? { ok: true, json: async () => ({ value: 'identity-secret' }) }
    : { ok: false, status: 403, json: async () => ({ token: 'secret' }) },
  }), /rejected monoprint-darwin-arm64 \(403\)/);
  assert.equal(calls, 2);
});
