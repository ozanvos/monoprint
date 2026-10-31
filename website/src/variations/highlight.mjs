// Tiny line highlighters for the page's own snippets, shared by the build and the browser.
// Classes map to Monoprint Light: hl-k keyword, hl-f function, hl-s string or value, hl-c comment.
const escape = text => text.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const span = (cls, text) => cls ? `<span class="hl-${cls}">${escape(text)}</span>` : escape(text);

// Commands bold, flags as keywords, their arguments as values.
export function shell(line) {
  let command = true;
  return line.split(/(\s+)/).map(word => {
    if (!word || /^\s+$/.test(word)) return escape(word);
    if (word === '|') { command = true; return span('c', word); }
    if (command) { command = false; return span('f', word); }
    return span(word.startsWith('-') ? 'k' : 's', word);
  }).join('');
}

const keywords = new Set(['fn', 'let', 'mut', 'use', 'pub', 'impl', 'return', 'struct']);
export function rust(line) {
  return line.replace(/([A-Za-z_][A-Za-z0-9_]*)(?=\s*\()|([A-Z][A-Z0-9_]+)\b|([A-Za-z_][A-Za-z0-9_]*)|([&?])|([^A-Za-z_&?]+)/g, (_, call, constant, word, operator, rest) => {
    if (call) return span('f', call);
    if (constant) return span('s', constant);
    if (word) return span(keywords.has(word) ? 'k' : '', word);
    if (operator) return span('k', operator);
    return escape(rest);
  });
}

// Shortcut lists: the keys, then what they do.
export function keys(line) {
  const [, key, gap, action] = line.match(/^(\S+)(\s+)(.*)$/) ?? [, line, '', ''];
  return span('k', key) + escape(gap) + span('c', action);
}

export const highlighters = { shell, rust, keys };
