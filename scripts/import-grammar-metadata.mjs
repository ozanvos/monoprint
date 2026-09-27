// Maintenance only. New collection inputs require an explicit license review.
import fs from 'node:fs/promises';
import path from 'node:path';
import { loadReview, checkInputs, generatedNotice, registryReceipt } from './grammar-policy.mjs';

const [mode, argument] = process.argv.slice(2);
const review = await loadReview();
const checkNotice = async () => {
  if (await fs.readFile('licenses/grammars-NOTICE.txt', 'utf8') !== generatedNotice(review)) {
    throw new Error('Stale grammar notices; run the importer');
  }
};
if (mode === '--check') {
  await checkNotice();
  const receipt = JSON.parse(await fs.readFile('assets/grammar-provenance.json', 'utf8'));
  if (JSON.stringify(receipt) !== JSON.stringify(await registryReceipt(review))) {
    throw new Error('Registry provenance is stale; rebuild with compile_registry');
  }
  console.log('Grammar licenses, notices, source copies, catalog and registry provenance verified.');
} else if (mode === '--record-registry') {
  await checkNotice();
  await fs.writeFile('assets/grammar-provenance.json', JSON.stringify(await registryReceipt(review), null, 2) + '\n');
} else {
  if (mode?.startsWith('--') && mode !== '--check-inputs') throw new Error(`Unknown option ${mode}`);
  const root = path.resolve((mode === '--check-inputs' ? argument : mode) ?? '/tmp/monoprint-grammars');
  const grammars = await checkInputs(root, review);
  const approved = new Map(review.policy.entries.filter(e => e.status === 'included').map(e => [e.name, e]));
  const metadata = grammars.filter(g => approved.has(g.name)).map(g => {
    const entry = approved.get(g.name);
    const result = { ...g, license: entry.license, licenseUrl: entry.licenseSource };
    if (result.embedded) result.embedded = result.embedded.filter(name => approved.has(name));
    return result;
  });
  const catalog = JSON.stringify(metadata, null, 2) + '\n';
  if (mode === '--check-inputs') {
    if (await fs.readFile('assets/grammar-metadata.json', 'utf8') !== catalog) throw new Error('Stale grammar catalog; run the importer first');
    await checkNotice();
  } else {
    await fs.writeFile('assets/grammar-metadata.json', catalog);
    await fs.writeFile('licenses/grammars-NOTICE.txt', generatedNotice(review));
    console.log(`Imported ${metadata.length} reviewed public grammars. Rebuild the registry next.`);
  }
}
