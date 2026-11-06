import test from 'node:test';
import assert from 'node:assert/strict';
import {stripAnsi,htmlSync,html,getLanguages,languageForFileName,MonoprintError} from '../dist/index.mjs';
test('terminal captures use visible text across native and JS APIs',()=>{
  assert.equal(stripAnsi('long\rX\x1b[K\n_\bZ'),'X\nZ');
  assert.ok(getLanguages().some(l=>l.id==='ansi'));
  assert.equal(languageForFileName('capture.ans'),'ansi');
  const rendered=htmlSync('\x1b[31mFAIL\x1b[0m',{language:'ansi'});
  assert.ok(rendered.includes('FAIL'));assert.ok(!rendered.includes('\x1b'));
});
test('compare, patch and marks render through the public interface',async()=>{
  const compare=await html('let count = 2;', {language:'rust',diff:{base:'let count = 1;',context:'all'},diffNumbers:'both'});
  assert.ok(compare.includes('diff-word'));assert.ok(compare.includes('diff-removed'));
  const patch=htmlSync('--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-old\n+new\n',{diff:'patch'});
  assert.ok(patch.includes('diff-added'));
  const marked=htmlSync('old\nnew',{language:'rust',removed:[1],added:[2],firstLine:120});assert.ok(marked.includes('data-number="120"'));
  const invalid=e=>e instanceof MonoprintError && e.code==='INVALID_OPTION';
  assert.throws(()=>htmlSync('x',{added:[1],removed:[1]}),invalid);
  assert.throws(()=>htmlSync('x',{diff:{base:'y'},added:[1]}),invalid);
  assert.throws(()=>htmlSync('not a patch',{diff:'patch'}),invalid);
});
