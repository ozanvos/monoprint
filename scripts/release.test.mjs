import assert from 'node:assert/strict';
import { test } from 'node:test';
import { assertPublicAllowed, checkRelease } from './release-config.mjs';

test('public publication requires both committed enablement and invocation approval', () => {
  for (const [enabled, approved] of [[false, false], [false, true], [true, false]]) {
    assert.throws(() => assertPublicAllowed({ publicPublishing: enabled }, { MONOPRINT_PUBLIC_PUBLISH: approved ? 'approved' : '' }), /disabled/);
  }
  assert.doesNotThrow(() => assertPublicAllowed({ publicPublishing: true }, { MONOPRINT_PUBLIC_PUBLISH: 'approved' }));
});
test('public publication refuses a fork or wrong repository', () => {
  assert.throws(() => assertPublicAllowed({ publicPublishing: true, repository: 'ozanvos/monoprint' },
    { MONOPRINT_PUBLIC_PUBLISH: 'approved', GITHUB_REPOSITORY: 'someone/monoprint' }), /mismatch/);
});
test('private repository cannot run public npm publication', () => {
  assert.throws(() => assertPublicAllowed({ publicPublishing: true },
    { MONOPRINT_PUBLIC_PUBLISH: 'approved', GITHUB_REPOSITORY_VISIBILITY: 'private' }), /public repository/);
});
test('release metadata agrees and publication requires invocation approval', async () => {
  const config = await checkRelease();
  assert.equal(typeof config.publicPublishing, 'boolean');
  assert.equal(config.repository, 'ozanvos/monoprint');
  await assert.rejects(checkRelease({ publicPublish: true }), /disabled/);
});
