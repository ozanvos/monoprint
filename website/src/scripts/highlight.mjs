// Tiny line highlighters for the page's own snippets, shared by the build and the browser.
// Classes map to Monoprint Light: hl-k keyword, hl-f function, hl-s string or value, hl-c comment.
const escape = text => text.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const span = (cls, text) => cls ? `<span class="hl-${cls}">${escape(text)}</span>` : escape(text);

// Commands bold, flags as keywords, their arguments as values.
export function shell(line) {
  let command = true;
  return line.split(/(\s+)/).map(word => {
    if (!word || /^\s+$/.test(word)) return escape(word);
    if (word === '|' || word === '&&') { command = true; return span('c', word); }
    if (command) { command = false; return span('f', word); }
    return span(word.startsWith('-') ? 'k' : 's', word);
  }).join('');
}
