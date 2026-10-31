import { shell } from './highlight.mjs';

const motion = document.documentElement.classList.contains('motion');
const GRID = 14;
const commandText = document.querySelector<HTMLElement>('#command-text')!;
const appSection = document.querySelector<HTMLElement>('.app-section')!;
const stage = document.querySelector<HTMLElement>('.stage')!;
const screenshot = appSection.querySelector<HTMLElement>('.app-screenshot')!;
const sheet = stage.querySelector<HTMLElement>('.sheet')!;
const images = [...sheet.querySelectorAll<HTMLImageElement>('img')];
const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

// Wraps a figure so crop marks and a dimension line can sit just outside its trim.
function proof(figure: HTMLElement, measurement: string) {
  const wrap = document.createElement('div');
  wrap.className = 'proof';
  figure.replaceWith(wrap);
  wrap.append(figure);
  const crop = document.createElement('div');
  crop.className = 'crop';
  crop.innerHTML = '<i></i>'.repeat(8);
  const dimension = document.createElement('div');
  dimension.className = 'dimension';
  dimension.innerHTML = `<span>${measurement}</span>`;
  for (const el of [crop, dimension]) el.setAttribute('aria-hidden', 'true');
  wrap.append(crop, dimension);
  return { wrap, label: dimension.querySelector('span')! };
}

function label(section: HTMLElement, html: string) {
  const fig = document.createElement('p');
  fig.className = 'fig';
  fig.setAttribute('aria-hidden', 'true');
  fig.innerHTML = html;
  section.prepend(fig);
}

// A plain fade: the marks and measurements carry the character, the image just arrives.
function reveal(target: HTMLElement, duration: number, delay = 0) {
  const fade = target.animate([{ opacity: 0 }, { opacity: 1 }], { duration, delay, easing: 'ease-out', fill: 'both' });
  target.classList.remove('pending');
  return fade;
}

const studio = proof(screenshot, '');
const sample = proof(sheet, '640 pt × 2 = 1280 px');
label(appSection, 'Fig. 1 <b>Studio</b> · macOS');
label(stage, 'Fig. 2 <b>share.rs</b>');

new ResizeObserver(([entry]) => {
  studio.label.textContent = `${Math.round(entry.contentRect.width)} px`;
}).observe(studio.wrap);

// Register, reveal, measure: each figure runs this once, the first time it is on screen.
async function develop(figure: ReturnType<typeof proof>, target: HTMLElement, duration: number) {
  figure.wrap.classList.add('marked');
  await wait(160);
  const fade = reveal(target, duration);
  await fade.finished;
  target.classList.add('printed');
  fade.cancel();
  figure.wrap.classList.add('measured');
}

function whenSeen(element: Element, threshold: number, then: () => void) {
  new IntersectionObserver((entries, observer) => {
    if (!entries[0].isIntersecting) return;
    observer.disconnect();
    then();
  }, { threshold }).observe(element);
}

// The hero command is set as plain text (typing, theme picker); colour it whenever that happens.
new MutationObserver(() => {
  if (!commandText.firstElementChild) commandText.innerHTML = shell(commandText.textContent!);
}).observe(commandText, { childList: true });
commandText.innerHTML = shell(commandText.textContent!);

// Hero: the command types itself and reports its output; the studio proof follows.
const status = document.createElement('p');
status.className = 'render-status';
status.setAttribute('aria-hidden', 'true');
status.innerHTML = '<span>→ <b>share.png</b></span><span>1280 × 666 px</span><span>rendered on this machine</span>';
document.querySelector('.hero-copy')!.append(status);

const full = commandText.textContent!;
let typing = 0;
let typed = !motion;
// Ends typing however it is cut short (finished, copied, theme picked) and starts the studio proof, once.
function finishTyping() {
  if (typed) return;
  typed = true;
  clearInterval(typing);
  commandText.classList.remove('typing');
  if (commandText.textContent!.length < full.length && full.startsWith(commandText.textContent!)) commandText.textContent = full;
  status.classList.add('shown');
  whenSeen(studio.wrap, .08, () => develop(studio, screenshot, 280));
}

if (motion) {
  const box = commandText.closest<HTMLElement>('.cli-command')!;
  box.style.minWidth = `min(100%, ${box.getBoundingClientRect().width}px)`;
  commandText.textContent = '';
  commandText.classList.add('typing');
  let i = 0;
  typing = window.setTimeout(() => {
    typing = window.setInterval(() => {
      commandText.textContent = full.slice(0, ++i);
      if (i < full.length) return;
      clearInterval(typing);
      setTimeout(finishTyping, 80);
    }, 7);
  }, 120);
  document.querySelector('#copy-command')!.addEventListener('pointerdown', finishTyping);
  whenSeen(sample.wrap, .35, () => develop(sample, sheet, 240));
} else {
  status.classList.add('shown');
  for (const figure of [studio, sample]) figure.wrap.classList.add('marked', 'measured');
  screenshot.classList.add('printed');
  sheet.classList.add('printed');
}

// Theme change: marks re-register while the new proof fades in over the old one.
let base = images.find(image => !image.hidden)!;
let running: Animation | null = null;
for (const input of document.querySelectorAll<HTMLInputElement>('input[name="theme"]')) {
  input.addEventListener('change', () => {
    finishTyping();
    setTimeout(() => {
      const next = images.find(image => image.dataset.theme === input.value)!;
      running?.cancel();
      const settle = () => {
        base = next;
        for (const image of images) image.hidden = image !== next;
      };
      if (!motion || next === base || !sheet.classList.contains('printed')) { settle(); return; }
      for (const image of images) {
        image.hidden = image !== next && image !== base;
        image.classList.toggle('leaving', image === base);
        image.classList.toggle('arriving', image === next);
      }
      sample.wrap.querySelector('.crop')!.animate([{ transform: 'scale(1.008)', opacity: .6 }, { transform: 'none', opacity: 1 }], { duration: 220, easing: 'cubic-bezier(.2,.7,.2,1)' });
      const fade = running = reveal(next, 180);
      fade.finished.then(() => { settle(); fade.cancel(); running = null; }, () => {});
    });
  });
}

// Export examples: each sits on its own mat with marks and a measurement, captioned like a spec sheet.
const tiles = [...document.querySelectorAll<HTMLElement>('.showcase figure')].map((figure, i) => {
  const tile = figure.querySelector<HTMLElement>('.showcase-tile')!;
  const image = tile.querySelector<HTMLImageElement>('img')!;
  const vector = figure.dataset.format === 'svg';
  const exported = proof(image, vector ? 'vector' : '');
  if (!vector) {
    // The PNGs are rendered at 2x, so the file is twice the width it is drawn at.
    const measure = () => { exported.label.textContent = `${image.naturalWidth} px @2x`; };
    image.complete && image.naturalWidth ? measure() : image.addEventListener('load', measure, { once: true });
  }
  label(tile, `Fig. ${i + 3} <b>${figure.dataset.source}</b>`);
  tile.classList.toggle('transparent', figure.dataset.transparent !== undefined);
  const spec = document.createElement('span');
  spec.className = 'spec';
  spec.textContent = `--theme ${figure.dataset.theme!.replace(' ', ' --')} · ${figure.dataset.format!.toUpperCase()}`;
  figure.querySelector('figcaption')!.append(spec);
  return { tile, image, exported };
});

if (motion) {
  for (const { image } of tiles) image.classList.add('pending');
  whenSeen(document.querySelector('.showcase')!, .25, () => tiles.forEach(({ image, exported }, i) => {
    setTimeout(() => develop(exported, image, 260), i * 90);
  }));
} else {
  for (const { exported } of tiles) exported.wrap.classList.add('marked', 'measured');
}

// A crosshair that snaps to the lattice, shown only where the pointer is over open grid.
if (matchMedia('(hover: hover) and (pointer: fine)').matches) {
  for (const section of [appSection, stage, ...tiles.map(t => t.tile)]) {
    const cross = document.createElement('div');
    cross.className = 'crosshair';
    cross.setAttribute('aria-hidden', 'true');
    cross.innerHTML = '<div class="h"></div><div class="v"></div><div class="readout"></div>';
    section.prepend(cross);
    const [h, v, readout] = cross.children as unknown as HTMLElement[];
    section.addEventListener('pointermove', event => {
      const overFigure = (event.target as Element).closest('.proof > :first-child, .fig');
      cross.classList.toggle('on', !overFigure);
      if (overFigure) return;
      const box = section.getBoundingClientRect();
      const x = Math.round((event.clientX - box.left) / GRID) * GRID;
      const y = Math.round((event.clientY - box.top) / GRID) * GRID;
      h.style.transform = `translateY(${y}px)`;
      v.style.transform = `translateX(${x}px)`;
      readout.textContent = `x ${x}  y ${y}`;
      const flipX = x + 110 > box.width, flipY = y + 30 > box.height;
      readout.style.transform = `translate(${flipX ? x - readout.offsetWidth - 6 : x + 6}px, ${flipY ? y - readout.offsetHeight - 6 : y + 6}px)`;
    });
    section.addEventListener('pointerleave', () => cross.classList.remove('on'));
  }
}
