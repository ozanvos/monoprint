import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const POLICY = 'licenses/grammar-policy.json';
const EVIDENCE = 'licenses/grammar-license-evidence.json';
const CATALOG = 'assets/grammar-metadata.json';
const NOTICE = 'licenses/grammars-NOTICE.txt';
const ALLOWED = new Set(['MIT', 'Apache-2.0', 'Apache-2.0 WITH LLVM-exception', 'BSD-2-Clause',
  'BSD-2-Clause-Views', 'BSD-3-Clause', 'ISC', 'MPL-2.0', 'LicenseRef-TextMate-Bundle']);
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const pinned = url => /^https:\/\/(?:raw\.githubusercontent\.com\/[^/]+\/[^/]+\/|github\.com\/[^/]+\/[^/]+\/blob\/)[a-f0-9]{40}\/.+/.test(url);
const digest = value => /^[a-f0-9]{64}$/.test(value);

export function validatePolicy(policy, evidence) {
  assert.equal(policy.schemaVersion, 1, 'Unknown grammar review schema');
  assert.match(policy.collection.commit, /^[a-f0-9]{40}$/);
  assert.ok(digest(policy.collection.indexSha256), 'Missing pinned collection index hash');
  const names = new Set();
  const used = new Set();
  for (const entry of policy.entries) {
    assert.match(entry.name, /^[a-z0-9][a-z0-9-]*$/);
    assert.ok(!names.has(entry.name), `Duplicate grammar ${entry.name}`);
    names.add(entry.name);
    assert.equal(typeof entry.public, 'boolean');
    assert.ok(digest(entry.sha256) && digest(entry.metadataSha256), `Missing input hashes: ${entry.name}`);
    assert.ok(pinned(entry.source), `Unpinned grammar source: ${entry.name}`);
    assert.ok(entry.scopeName, `Missing scope: ${entry.name}`);
    if (entry.localFile) {
      assert.equal(entry.localFile, `assets/grammars/${entry.name}.json`, 'Invalid local grammar path');
      assert.equal(entry.metadata?.name, entry.name, 'Local grammar metadata name differs');
      assert.equal(entry.metadata.scopeName, entry.scopeName, 'Local grammar metadata scope differs');
      assert.equal(sha256(JSON.stringify(entry.metadata)), entry.metadataSha256, 'Local grammar metadata changed');
    }
    if (entry.status === 'excluded') {
      assert.ok(entry.reason, `Missing exclusion reason: ${entry.name}`);
      continue;
    }
    assert.equal(entry.status, 'included', `Unreviewed grammar: ${entry.name}`);
    assert.ok(ALLOWED.has(entry.license), `Unapproved license ${entry.license}: ${entry.name}`);
    const notice = evidence[entry.notice];
    assert.ok(notice?.text.trim(), `Missing license text: ${entry.name}`);
    assert.equal(sha256(notice.text), notice.sha256, `License text changed: ${entry.name}`);
    assert.ok(!/^\s*(?:<!doctype html|<html)/i.test(notice.text), `HTML page instead of license: ${entry.name}`);
    assert.ok(notice.sources.includes(entry.licenseSource) && notice.sources.every(pinned), `Unpinned license source: ${entry.name}`);
    used.add(entry.notice);
    if (entry.license === 'MPL-2.0') {
      assert.deepEqual(entry.correspondingSource?.map(s => s.file).sort(),
        [`licenses/grammar-sources/${entry.name}.json`, `licenses/grammar-sources/${entry.name}.upstream.json`],
        `Missing MPL source: ${entry.name}`);
    }
    for (const source of entry.correspondingSource ?? []) {
      assert.ok([`licenses/grammar-sources/${entry.name}.json`, `licenses/grammar-sources/${entry.name}.upstream.json`].includes(source.file));
      assert.ok(digest(source.sha256));
    }
  }
  assert.deepEqual([...used].sort(), Object.keys(evidence).sort(), 'Unused license evidence');
}

export async function loadReview() {
  const policy = JSON.parse(await fs.readFile(POLICY, 'utf8'));
  const evidence = JSON.parse(await fs.readFile(EVIDENCE, 'utf8'));
  validatePolicy(policy, evidence);
  for (const entry of policy.entries) {
    if (entry.localFile) {
      assert.equal(sha256(await fs.readFile(entry.localFile)), entry.sha256, `Local grammar changed: ${entry.name}`);
    }
    for (const source of entry.correspondingSource ?? []) {
      assert.equal(sha256(await fs.readFile(source.file)), source.sha256, `Corresponding source changed: ${source.file}`);
      if (source.file.endsWith(`/${entry.name}.json`)) assert.equal(source.sha256, entry.sha256);
    }
  }
  return { policy, evidence, collectionLicense: await fs.readFile('licenses/grammars-LICENSE.txt', 'utf8') };
}

export async function checkInputs(root, { policy }) {
  const directory = path.join(root, 'packages/tm-grammars');
  const index = path.join(directory, 'index.js');
  assert.equal(sha256(await fs.readFile(index)), policy.collection.indexSha256, 'Collection index changed; review the new revision');
  const { grammars, injections } = await import(pathToFileURL(index));
  const metadata = new Map([...grammars, ...injections].map(g => [g.name, g]));
  const collectionEntries = policy.entries.filter(e => !e.localFile);
  assert.deepEqual([...metadata.keys()].sort(), collectionEntries.map(e => e.name).sort(), 'Unreviewed metadata entries');
  const files = (await fs.readdir(path.join(directory, 'grammars'))).filter(p => p.endsWith('.json')).sort();
  assert.deepEqual(files, collectionEntries.map(e => `${e.name}.json`).sort(), 'Unreviewed grammar files');
  const excludedScopes = new Set(policy.entries.filter(e => e.status === 'excluded').map(e => e.scopeName));
  for (const entry of policy.entries) {
    const bytes = await fs.readFile(entry.localFile ?? path.join(directory, 'grammars', `${entry.name}.json`));
    assert.equal(sha256(bytes), entry.sha256, `Grammar changed; review ${entry.name}`);
    const info = entry.localFile ? entry.metadata : metadata.get(entry.name);
    assert.equal(sha256(JSON.stringify(info)), entry.metadataSha256, `Metadata changed; review ${entry.name}`);
    assert.equal(!Boolean(info.embeddedIn), entry.public, `Public/injection mismatch: ${entry.name}`);
    const grammar = JSON.parse(bytes);
    assert.equal(grammar.name, entry.name);
    assert.equal(grammar.scopeName, entry.scopeName);
    if (entry.status !== 'included') continue;
    const missing = new Set();
    function visit(value) {
      if (!value || typeof value !== 'object') return;
      if (typeof value.include === 'string') {
        const scope = value.include.split('#')[0];
        if (excludedScopes.has(scope)) missing.add(scope);
      }
      Object.values(value).forEach(visit);
    }
    visit(grammar);
    assert.deepEqual([...missing].sort(), entry.unavailableEmbeddedScopes ?? [], `Unreviewed missing embedded grammar: ${entry.name}`);
  }
  return [...grammars, ...policy.entries.filter(e => e.localFile && e.public).map(e => e.metadata)];
}

export function generatedNotice({ policy, evidence, collectionLicense }) {
  const included = policy.entries.filter(e => e.status === 'included');
  let notice = 'MONOPRINT BUNDLED TEXTMATE GRAMMAR NOTICES\n\n'
    + 'Only grammars marked included in grammar-policy.json are embedded.\n'
    + 'Original licenses apply; the project Apache-2.0 license does not replace them.\n'
    + `Collection: ${policy.collection.repository}/tree/${policy.collection.commit}\n`
    + 'The collection normalizes/patches upstream grammars; Monoprint compiles that\n'
    + 'JSON into Giallo data without further grammar edits. Exact inputs, original\n'
    + 'sources and changes are recorded in grammar-policy.json and maintenance docs.\n'
    + 'Additional local grammars and their pinned upstream sources are recorded in\n'
    + 'the same policy; their source JSON is supplied in assets/grammars.\n'
    + 'MPL-covered source is supplied in licenses/grammar-sources under MPL-2.0.\n'
    + 'Ship these notices and corresponding sources with software distributions.\n\n'
    + 'COLLECTION LICENSE\n\n' + collectionLicense;
  for (const id of [...new Set(included.map(e => e.notice))]) {
    const entries = included.filter(e => e.notice === id);
    notice += '\n' + '='.repeat(80) + '\n'
      + `Files: ${entries.map(e => `${e.name}.json`).join(', ')}\n`
      + `License: ${[...new Set(entries.map(e => e.license))].join('; ')}\n`
      + evidence[id].sources.map(source => `Source: ${source}\n`).join('')
      + '-'.repeat(80) + '\n' + evidence[id].text + '\n';
  }
  // Keep original evidence intact; normalize layout only in the assembled notice.
  return notice.split('\n').map(line => line.trimEnd()).join('\n').trimEnd() + '\n';
}

export async function registryReceipt({ policy }) {
  const metadata = JSON.parse(await fs.readFile(CATALOG, 'utf8'));
  const included = policy.entries.filter(e => e.status === 'included');
  assert.deepEqual(metadata.map(g => g.name).sort(), included.filter(e => e.public).map(e => e.name).sort(), 'Catalog and grammar policy differ');
  for (const grammar of metadata) {
    const entry = included.find(e => e.name === grammar.name);
    assert.equal(grammar.license, entry.license);
    assert.equal(grammar.licenseUrl, entry.licenseSource);
  }
  const files = [POLICY, EVIDENCE, CATALOG, NOTICE, 'licenses/grammars-LICENSE.txt', 'assets/registry.zst',
    'assets/editor/themes.json', 'assets/editor/sources.json', 'licenses/editor-themes-NOTICE.txt',
    'licenses/editor-themes-provenance.json'];
  const checksums = {};
  for (const file of files) checksums[file] = sha256(await fs.readFile(file));
  for (const entry of included.filter(e => e.localFile)) {
    checksums[entry.localFile] = sha256(await fs.readFile(entry.localFile));
  }
  for (const name of (await fs.readdir('assets/themes')).filter(n => n.endsWith('.json')).sort()) {
    const file = `assets/themes/${name}`;
    checksums[file] = sha256(await fs.readFile(file));
  }
  return { ...policy.collection, giallo: '0.5.2', includedGrammars: included.length, checksums };
}
