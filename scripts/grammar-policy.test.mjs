import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { validatePolicy, checkInputs, sha256 } from './grammar-policy.mjs';

const policy = JSON.parse(await fs.readFile('licenses/grammar-policy.json', 'utf8'));
const evidence = JSON.parse(await fs.readFile('licenses/grammar-license-evidence.json', 'utf8'));

test('the checked-in review has complete license evidence', () => validatePolicy(policy, evidence));

for (const license of ['GPL-3.0', 'LGPL-3.0', 'NOASSERTION', undefined]) {
  test(`rejects unapproved license ${license}`, () => {
    const copy = structuredClone(policy);
    copy.entries.find(e => e.status === 'included').license = license;
    assert.throws(() => validatePolicy(copy, evidence), /Unapproved license/);
  });
}

test('rejects HTML saved as a license, even with a matching hash', () => {
  const copy = structuredClone(evidence);
  const notice = copy[policy.entries.find(e => e.status === 'included').notice];
  notice.text = '<!DOCTYPE html><html>A GitHub page is not a license record</html>';
  notice.sha256 = sha256(notice.text);
  assert.throws(() => validatePolicy(policy, copy), /HTML page instead of license/);
});

test('rejects missing notices, altered evidence and unpinned license sources', () => {
  const id = policy.entries.find(e => e.status === 'included').notice;
  const missing = structuredClone(evidence);
  delete missing[id];
  assert.throws(() => validatePolicy(policy, missing), /Missing license text/);
  const changed = structuredClone(evidence);
  changed[id].text += ' edited';
  assert.throws(() => validatePolicy(policy, changed), /License text changed/);
  const unpinned = structuredClone(evidence);
  unpinned[id].sources.push('https://raw.githubusercontent.com/example/repo/main/LICENSE');
  assert.throws(() => validatePolicy(policy, unpinned), /Unpinned license source/);
});

test('MPL approvals require both original and normalized corresponding source', () => {
  const copy = structuredClone(policy);
  const entry = copy.entries.find(e => e.license === 'MPL-2.0');
  entry.correspondingSource = [entry.correspondingSource[0], entry.correspondingSource[0]];
  assert.throws(() => validatePolicy(copy, evidence), /Missing MPL source/);
});

test('the importer rejects unreviewed files and changed inputs before compilation', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'monoprint-grammar-policy-'));
  try {
    const directory = path.join(root, 'packages/tm-grammars');
    await fs.mkdir(path.join(directory, 'grammars'), { recursive: true });
    await fs.writeFile(path.join(root, 'package.json'), '{"type":"module"}');
    const info = { name: 'example', scopeName: 'source.example' };
    const index = `export const grammars = [${JSON.stringify(info)}]; export const injections = [];`;
    const grammar = JSON.stringify({ ...info, patterns: [] });
    await fs.writeFile(path.join(directory, 'index.js'), index);
    const file = path.join(directory, 'grammars/example.json');
    await fs.writeFile(file, grammar);
    const review = { policy: { collection: { indexSha256: sha256(index) }, entries: [
      { ...info, public: true, status: 'included', sha256: sha256(grammar), metadataSha256: sha256(JSON.stringify(info)) },
    ] } };
    await checkInputs(root, review);
    const extra = path.join(directory, 'grammars/unreviewed.json');
    await fs.writeFile(extra, grammar);
    await assert.rejects(checkInputs(root, review), /Unreviewed grammar files/);
    await fs.unlink(extra);
    await fs.writeFile(file, grammar + '\n');
    await assert.rejects(checkInputs(root, review), /Grammar changed/);
    await fs.writeFile(file, grammar);
    await fs.writeFile(path.join(directory, 'index.js'), index + '\n');
    await assert.rejects(checkInputs(root, review), /Collection index changed/);
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
