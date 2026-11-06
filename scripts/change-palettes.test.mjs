import test from 'node:test';import assert from 'node:assert/strict';import fs from 'node:fs';import {contrast} from './change-palettes.mjs';
test('all appearances have readable terminal and diff tokens',()=>{
  for(const family of ['editor','originals','selected','code-styles'])for(const theme of Object.values(JSON.parse(fs.readFileSync(`assets/${family}/themes.json`))))for(const [mode,p] of Object.entries(theme.syntax)) {
    const surface=p['--ink-ansi-surface'];assert.ok(surface,`${theme.id} ${mode}`);
    for(let i=0;i<16;i++){const color=p[`--ink-ansi-${i}`];assert.ok(color);assert.ok(contrast(color,surface)>=2.99,`${theme.id} ${mode} ANSI ${i}`);}
    for(const kind of ['added','removed']){for(const suffix of ['','-word'])assert.ok(contrast(p['--ink-foreground'],p[`--ink-diff-${kind}${suffix}`])>=4.49,`${theme.id} ${mode} ${kind}${suffix}`);assert.ok(contrast(p[`--ink-diff-${kind}-sign`],p[`--ink-diff-${kind}`])>=2.99);}
  }
});
