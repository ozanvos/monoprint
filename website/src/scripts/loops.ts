// The looping demos: Fig. 1's Studio and one on each interface card. Each scene is drawn at a fixed
// size and scaled to its figure. With motion allowed, a loop plays while it is on screen and pauses
// between beats when it is not; reduced motion keeps the first frame, which is complete on its own.
import { themes } from '../data/demo.mjs';
import { loopThemes, renders, names } from '../data/loops.mjs';

const TRAVEL = 'cubic-bezier(0.45, 0, 0.1, 1)';
const ARRIVE = 'cubic-bezier(0.25, 0.8, 0.25, 1)';
const motion = document.documentElement.classList.contains('motion');
const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
const byCli = Object.fromEntries(themes.map(theme => [theme.cli, theme]));
const themeArgs = (cli: string) => `${cli}${byCli[cli].appearance === 'light' ? ' --light' : ''}`;

function scaled(loop: HTMLElement) {
  const scene = loop.querySelector<HTMLElement>('.loop-scene')!;
  new ResizeObserver(([entry]) => loop.style.setProperty('--k', String(entry.contentRect.width / scene.offsetWidth))).observe(loop);
}

// Where a point inside `target` sits in its scene's own units, as fractions of its box.
function pointIn(scene: HTMLElement, target: Element, fx: number, fy: number) {
  const box = target.getBoundingClientRect();
  const origin = scene.getBoundingClientRect();
  const k = origin.width / scene.offsetWidth;
  return { x: (box.left - origin.left + box.width * fx) / k, y: (box.top - origin.top + box.height * fy) / k };
}

// A hand on a mouse: a slight arc rather than a ruler line, smooth speed up and slow down
// (minimum jerk) and longer for longer distances. `offset` is the glyph's hotspot.
function glide(element: Element, from: { x: number; y: number }, to: { x: number; y: number }, offset = { x: 0, y: 0 }) {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const distance = Math.hypot(dx, dy);
  // Bow to one side by a small share of the distance, perpendicular to the line.
  const bow = Math.min(distance * 0.08, 28);
  const cx = (from.x + to.x) / 2 - (dy / (distance || 1)) * bow;
  const cy = (from.y + to.y) / 2 + (dx / (distance || 1)) * bow;
  const keyframes = Array.from({ length: 25 }, (_, i) => {
    const t = i / 24;
    const s = t * t * t * (10 - 15 * t + 6 * t * t);
    const x = (1 - s) * (1 - s) * from.x + 2 * (1 - s) * s * cx + s * s * to.x;
    const y = (1 - s) * (1 - s) * from.y + 2 * (1 - s) * s * cy + s * s * to.y;
    return { transform: `translate(${x - offset.x}px, ${y - offset.y}px)` };
  });
  const duration = distance ? Math.min(380 + distance * 0.75, 900) : 0;
  return element.animate(keyframes, { duration, easing: 'linear', fill: 'forwards' }).finished;
}

// Resolves at once while the loop is on screen, otherwise when it next comes into view.
function onScreen(loop: HTMLElement) {
  let visible = false;
  let wake: (() => void) | undefined;
  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    if (visible) wake?.();
  }, { threshold: 0.4 }).observe(loop);
  return () => visible ? Promise.resolve() : new Promise<void>(resolve => { wake = resolve; });
}

// A key cap appears, goes down and comes up; `hold` keeps it down until the returned release.
async function press(keys: HTMLElement, labels: string[], hold = false) {
  const caps = labels.map(label => Object.assign(document.createElement('kbd'), { textContent: label }));
  keys.replaceChildren(...caps);
  keys.animate([{ opacity: 0, transform: 'translateY(4px)' }, { opacity: 1, transform: 'none' }], { duration: 180, easing: ARRIVE });
  await wait(200);
  for (const cap of caps) cap.classList.add('down');
  const release = () => caps.forEach(cap => cap.classList.remove('down'));
  if (!hold) { await wait(140); release(); }
  return release;
}

function clearKeys(keys: HTMLElement) {
  if (!keys.childElementCount) return;
  keys.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 240, fill: 'forwards' }).finished.then(animation => {
    keys.replaceChildren();
    animation.cancel();
  });
}

// A value rolls to its new text, upwards when it grows; the slot keeps the line's height.
async function roll(slot: HTMLElement, text: string, up: boolean) {
  const from = slot.textContent!;
  const track = document.createElement('span');
  for (const line of up ? [from, text] : [text, from]) track.append(Object.assign(document.createElement('span'), { textContent: line }));
  slot.replaceChildren(track);
  const [start, end] = up ? ['none', 'translateY(-19px)'] : ['translateY(-19px)', 'none'];
  await track.animate([{ transform: start }, { transform: end }], { duration: 460, easing: TRAVEL, fill: 'forwards' }).finished;
  slot.textContent = text;
}

// A text caret after `anchor`, taking no width so the line does not shift.
function caretAfter(anchor: Element) {
  const caret = document.createElement('span');
  caret.className = 'text-caret';
  anchor.after(caret);
  return caret;
}

// Studio: double-click a word, retype it, and the code image follows each keystroke.
async function studio(loop: HTMLElement, seen: () => Promise<void>) {
  const scene = loop.querySelector<HTMLElement>('.loop-scene')!;
  const word = loop.querySelector<HTMLElement>('.studio-editor em')!;
  const preview = loop.querySelector<HTMLElement>('.studio-card em')!;
  const ibeam = loop.querySelector<SVGElement>('.loop-ibeam')!;
  const caret = caretAfter(word);
  const rest = { x: 112, y: 168 };
  let at = rest;
  const moveTo = (to: { x: number; y: number }) => {
    const from = at;
    at = to;
    return glide(ibeam, from, to);
  };
  const show = (on: boolean) => ibeam.animate([{ opacity: on ? 0 : 1 }, { opacity: on ? 1 : 0 }], { duration: on ? 260 : 140, fill: 'forwards' }).finished;
  for (let turn = 1; ; turn++) {
    await seen();
    at = rest;
    await moveTo(rest);
    await show(true);
    await wait(250);
    // Aim inside the word but off its letters' middle, so the I-beam doesn't cover what it selects.
    const aim = pointIn(scene, word, 0.62, 0.5);
    await moveTo({ x: aim.x - 5, y: aim.y - 8 });
    await wait(260);
    caret.hidden = true;
    word.classList.add('selected');
    await wait(520);
    await show(false);
    const name = names[turn % names.length];
    for (let i = 1; i <= name.length; i++) {
      word.classList.remove('selected');
      caret.hidden = false;
      caret.classList.remove('blink');
      word.textContent = preview.textContent = name.slice(0, i);
      await wait(70 + (i % 3) * 25);
    }
    caret.classList.add('blink');
    await wait(2600);
  }
}

// CLI: recall the last command, change its theme, run it, and the new image lands over the last.
async function cli(loop: HTMLElement, seen: () => Promise<void>) {
  const lines = loop.querySelector<HTMLElement>('.terminal-scroll')!;
  const keys = loop.querySelector<HTMLElement>('.loop-keys')!;
  const output = loop.querySelector<HTMLElement>('.output')!;
  const template = lines.firstElementChild!.cloneNode(true) as HTMLElement;
  const images = loopThemes.map(cli => Object.assign(new Image(535, 269), { src: `/assets/output-${byCli[cli].id}.png`, alt: '' }));
  let current = lines.querySelector<HTMLElement>('.current')!;
  for (let turn = 1; ; turn++) {
    await seen();
    const previous = loopThemes[(turn - 1) % loopThemes.length];
    const next = loopThemes[turn % loopThemes.length];
    await wait(500);
    await press(keys, ['↑']);
    const recalled = template.cloneNode(true) as HTMLElement;
    current.querySelector('code')!.replaceWith(recalled.querySelector('code')!);
    const value = current.querySelector('em')!;
    value.textContent = themeArgs(previous);
    await wait(420);
    const release = await press(keys, ['⌫'], true);
    for (let n = value.textContent.length - 1; n >= 0; n--) {
      value.textContent = value.textContent.slice(0, n);
      await wait(28);
    }
    release();
    clearKeys(keys);
    await wait(180);
    const typed = themeArgs(next);
    for (let i = 1; i <= typed.length; i++) {
      value.textContent = typed.slice(0, i);
      await wait(62 + (i % 4) * 14);
    }
    await wait(380);
    await press(keys, ['↩']);
    current.classList.remove('current');
    current.querySelector('.caret')!.remove();
    current = Object.assign(document.createElement('div'), { className: 'current', innerHTML: '<span class="prompt">$</span><code></code><span class="caret"></span>' });
    lines.append(current);
    // The terminal keeps its last four lines: the oldest scrolls out, then leaves the DOM unseen.
    if (lines.childElementCount > 4) {
      await lines.animate([{ transform: 'none' }, { transform: 'translateY(-19px)' }], { duration: 160, easing: 'ease-out' }).finished;
      lines.firstElementChild!.remove();
    }
    await wait(220);
    const image = images[turn % images.length].cloneNode() as HTMLImageElement;
    output.append(image);
    // It arrives as a solid print: opaque within the first stretch of its rise, never a ghost over the last.
    await image.animate([{ opacity: 0, transform: 'translateY(26px)' }, { opacity: 1, offset: 0.25 }, { opacity: 1, transform: 'none' }], { duration: 900, easing: ARRIVE }).finished;
    while (output.childElementCount > 1) output.firstElementChild!.remove();
    clearKeys(keys);
    await wait(2200);
  }
}

// Library: change the scale, save, and every file the loop writes is rewritten at the new size.
async function library(loop: HTMLElement, seen: () => Promise<void>) {
  const scale = loop.querySelector<HTMLElement>('.scale')!;
  const keys = loop.querySelector<HTMLElement>('.loop-keys')!;
  const sizes = [...loop.querySelectorAll<HTMLElement>('.size em')];
  const caret = caretAfter(scale);
  caret.hidden = true;
  for (let turn = 1; ; turn++) {
    await seen();
    const value = turn % 2 ? 4 : 2;
    await wait(500);
    caret.hidden = false;
    caret.classList.add('blink');
    await wait(900);
    caret.classList.remove('blink');
    scale.textContent = '';
    await wait(240);
    scale.textContent = String(value);
    await wait(500);
    await press(keys, ['⌘', 'S']);
    caret.classList.add('blink');
    await wait(280);
    for (const [i, size] of sizes.entries()) {
      roll(size, String(Math.round(renders[i].bytes[value] / 1000)), value === 4);
      await wait(240);
    }
    clearKeys(keys);
    await wait(900);
    caret.hidden = true;
    await wait(1700);
  }
}

// A segmented control's marker sits under its selected option, measured in its own unscaled layout.
function place(segmented: HTMLElement) {
  const selected = segmented.querySelector<HTMLElement>('[data-selected]')!;
  segmented.style.setProperty('--x', `${selected.offsetLeft}px`);
  segmented.style.setProperty('--w', `${selected.offsetWidth}px`);
}

// Fig. 1: switch the appearance, then the backdrop, then copy the image; the next pass switches
// back. The preview is the CLI's render for each pair, so every frame is real output.
async function hero(figure: HTMLElement, seen: () => Promise<void>) {
  const scene = figure.querySelector<HTMLElement>('.loop-scene')!;
  const pointer = figure.querySelector<SVGElement>('.pointer')!;
  const copy = figure.querySelector<HTMLElement>('.copy-button')!;
  const faces = [...copy.querySelectorAll<HTMLElement>('.copy-face')];
  const control = (name: string) => figure.querySelector<HTMLElement>(`[data-segmented="${name}"]`)!;
  const showOnly = (selector: string, value: string, attribute: string) => {
    for (const image of figure.querySelectorAll<HTMLElement>(selector)) image.toggleAttribute('data-on', image.getAttribute(attribute) === value);
  };
  for (const image of figure.querySelectorAll<HTMLImageElement>('img[loading="lazy"]')) image.loading = 'eager';
  // The macOS arrow's hotspot is 11, 8 into its 36 px glyph. It waits in the inspector's empty
  // stretch, between the controls it uses, so it never sweeps across the panel.
  const rest = { x: 1262, y: 590 };
  let at = rest;
  const moveTo = (to: { x: number; y: number }) => {
    const from = at;
    at = to;
    return glide(pointer, from, to, { x: 11, y: 8 });
  };
  const press = async () => {
    await pointer.animate([{ scale: 1 }, { scale: 0.86 }, { scale: 1 }], { duration: 180, easing: 'ease-out', composite: 'add' }).finished;
  };
  const choose = async (name: string, option: string) => {
    const segmented = control(name);
    const target = segmented.querySelector<HTMLElement>(`[data-option="${option}"]`)!;
    await moveTo(pointIn(scene, target, 0.55, 0.78));
    await wait(120);
    await press();
    segmented.querySelector('[data-selected]')!.removeAttribute('data-selected');
    target.setAttribute('data-selected', '');
    place(segmented);
  };
  await moveTo(rest);
  let shown = false;
  for (let turn = 0; ; turn++) {
    await seen();
    if (!shown) {
      shown = true;
      await pointer.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 300, fill: 'forwards' }).finished;
    }
    const [appearance, backdrop] = turn % 2 ? ['light', 'solid'] : ['dark', 'lattice'];
    await wait(500);
    await choose('appearance', appearance);
    showOnly('[data-thumb]', appearance, 'data-thumb');
    showOnly('[data-preview]', `${appearance}-${control('backdrop').querySelector('[data-selected]')!.getAttribute('data-option')}`, 'data-preview');
    await wait(950);
    await choose('backdrop', backdrop);
    showOnly('[data-preview]', `${appearance}-${backdrop}`, 'data-preview');
    await wait(1000);
    await moveTo(pointIn(scene, copy, 0.72, 0.7));
    await wait(120);
    copy.classList.add('down');
    await press();
    copy.classList.remove('down');
    faces[0].removeAttribute('data-on');
    faces[1].setAttribute('data-on', '');
    await wait(1700);
    faces[1].removeAttribute('data-on');
    faces[0].setAttribute('data-on', '');
    await moveTo(rest);
    await wait(1300);
  }
}

for (const segmented of document.querySelectorAll<HTMLElement>('[data-segmented]')) place(segmented);
const scripts = { Hero: hero, Studio: studio, CLI: cli, Library: library };
for (const loop of document.querySelectorAll<HTMLElement>('[data-loop]')) {
  scaled(loop);
  if (motion) scripts[loop.dataset.loop as keyof typeof scripts](loop, onScreen(loop));
}
