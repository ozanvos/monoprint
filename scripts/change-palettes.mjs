// Shared palette enrichment for terminal captures and unified changes.
// Upstream colors retain their hue; only luminance/alpha is adjusted when needed
// for legibility on the actual code surface. Originals use a curated terminal palette tuned to their reading surfaces.
export const ansiNames = ['Black','Red','Green','Yellow','Blue','Magenta','Cyan','White','BrightBlack','BrightRed','BrightGreen','BrightYellow','BrightBlue','BrightMagenta','BrightCyan','BrightWhite'];
const rgb = c => [1,3,5].map(i=>parseInt(c.slice(i,i+2),16));
const hex = c => '#'+c.map(v=>Math.round(v).toString(16).padStart(2,'0')).join('');
export const mix = (a,b,t) => hex(rgb(a).map((v,i)=>v*(1-t)+rgb(b)[i]*t));
const luminance = c => rgb(c).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;}).reduce((a,v,i)=>a+v*[.2126,.7152,.0722][i],0);
export const contrast = (a,b) => (Math.max(luminance(a),luminance(b))+.05)/(Math.min(luminance(a),luminance(b))+.05);
function readable(c,surface,minimum) {
  const target=contrast('#000000',surface)>contrast('#ffffff',surface)?'#000000':'#ffffff';
  for(let i=0;i<=100;i++){const result=mix(c,target,i/100);if(contrast(result,surface)>=minimum)return result;}
  return target;
}
export function changePalette(syntax, surface, upstream={}) {
  const dark=luminance(surface)<.3;
  const fallback=dark?['#292c33','#f06c75','#87c987','#e4c76d','#82aaff','#c792ea','#70c9ce','#e6e6e6','#8a8f98','#ff8790','#a5e3a5','#ffe29a','#a6c2ff','#e2adff','#9be4e8','#ffffff']:['#24262b','#a12132','#226b32','#806000','#2459a6','#853d99','#146e78','#d8dce0','#666b73','#b32339','#26783b','#886700','#315ebe','#9843aa','#147884','#ffffff'];
  const output={'--ink-ansi-surface':surface.slice(0,7)};const fallbackNames=[];
  ansiNames.forEach((name,i)=>{const key=`terminal.ansi${name}`;if(!upstream[key])fallbackNames.push(key);output[`--ink-ansi-${i}`]=readable(upstream[key]||fallback[i],surface,3);});
  for(const [kind,name,color] of [['added','inserted',output['--ink-ansi-2']],['removed','removed',output['--ink-ansi-1']]]) {
    const sign=readable(upstream[`gitDecoration.${kind==='added'?'added':'deleted'}ResourceForeground`]||color,surface,3);
    for(const [suffix,key,alpha] of [['',`${name}LineBackground`,.10],['-word',`${name}TextBackground`,.22]]) {
      const input=upstream[`diffEditor.${key}`];let a=input?.length===9?parseInt(input.slice(7),16)/255:input?alpha:alpha;
      const ink=input||sign;let result=mix(surface,ink,a);
      while(a>0 && contrast(syntax['--ink-foreground'],result)<4.5){a=Math.max(0,a-.01);result=mix(surface,ink,a);}
      if(contrast(syntax['--ink-foreground'],result)<4.5) {
        const target=contrast('#000000',syntax['--ink-foreground'])>contrast('#ffffff',syntax['--ink-foreground'])?'#000000':'#ffffff';
        for(let i=1;i<=100;i++){const adjusted=mix(result,target,i/100);if(contrast(syntax['--ink-foreground'],adjusted)>=4.5){result=adjusted;break;}}
      }
      output[`--ink-diff-${kind}${suffix}`]=result;
    }
    output[`--ink-diff-${kind}-sign`]=readable(sign,output[`--ink-diff-${kind}`],3);
  }
  return {tokens:output,fallbackNames};
}
