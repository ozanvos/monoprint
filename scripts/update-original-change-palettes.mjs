import fs from 'node:fs/promises';
import { changePalette } from './change-palettes.mjs';
// Representative code-surface colors from frames.json; gradient themes use the
// central reading surface. These never replace the frame artwork.
const surfaces={
 'the-impression':['#f7f5ec','#171a19'],'counterform':['#f2f1e9','#1c201e'],
 'light-table':['#fff5cf','#29232e'],'ragged-block':['#cfe0de','#223f49'],
 'spot-colour':['#f6f5f1','#161615'],'segment-glass':['#b4bfa2','#1c2740'],
 'label-tape':['#ecebe5','#121214'],'urushi':['#f5ead4','#130807'],
 'one-bit':['#ffffff','#000000'],'holofoil':['#f1f0f6','#101017'],
 'ground':['#f3f0e8','#1d1c19'],'galley':['#f6f5f1','#1b1f24'],
 'dusk':['#e6e9ef','#1f232a'],'cantilever':['#f4f1ea','#1f1a17'],
 'ordinal':['#e3e6df','#1c201d'],'aside':['#a9bdb0','#1f2a25'],
 'jetty':['#f6f8f7','#0d1517'],'capsule':['#fbf3f1','#1a1213'],
 'flange':['#f7f6f3','#16181b'],'inlay':['#f7f7f3','#14171d'],
};
const palettes=JSON.parse(await fs.readFile('assets/pierre-palettes.json','utf8'));
for(const family of ['originals','selected','code-styles']) {
  const path=`assets/${family}/themes.json`;
  const themes=JSON.parse(await fs.readFile(path,'utf8'));
  const frames=family==='originals'?{}:JSON.parse(await fs.readFile(`assets/${family}/frames.json`,'utf8'));
  for(const theme of Object.values(themes)) for(const [mode,syntax] of Object.entries(theme.syntax)) {
    const key=`original-${theme.id}${Object.keys(theme.syntax).length>1?`-${mode}`:''}`;
    const palette=palettes[key];
    if(!palette)throw Error(`Missing ${key}`);
    let surface=surfaces[theme.id]?.[mode==='light'?0:1];
    if (!surface && frames[theme.id]) {
      const css=frames[theme.id].css;
      const rules=[...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].filter(([,selector])=>selector.trim()==='#capture'||selector.trim()===`#capture[data-theme="${mode}"]`);
      for(const [,selector,body] of rules){const value=body.match(/--(?:surface|paper|plate-b):(#[0-9a-fA-F]+)/);if(value)surface=value[1].slice(0,7);}
    }
    surface ||= palette.background;
    syntax['--ink-ansi-surface']=surface;
    Object.assign(syntax,changePalette(syntax,surface).tokens);
  }
  await fs.writeFile(path,JSON.stringify(themes,null,2)+'\n');
}
