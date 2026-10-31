// The proof-sheet motion: the hero command types itself, figures register their crop marks,
// fade in and are measured, the theme picker's marker slides, and the lattice shows a
// crosshair snapped to its grid. Everything still reads correctly without it.
import { shell } from './highlight.mjs';

const motion = document.documentElement.classList.contains('motion');
const GRID = 14;
const commandText = document.querySelector<HTMLElement>('#command-text')!;
const status = document.querySelector<HTMLElement>('.render-status')!;
const appSection = document.querySelector<HTMLElement>('.app-section')!;
const stage = document.querySelector<HTMLElement>('.stage')!;
const screenshot = appSection.querySelector<HTMLElement>('.app-screenshot')!;
const sheet = stage.querySelector<HTMLElement>('.sheet')!;
const images = [...sheet.querySelectorAll<HTMLImageElement>('img')];
const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

function proofOf(figure: Element) {
  const wrap = figure.closest<HTMLElement>('.proof')!;
  return { wrap, label: wrap.querySelector<HTMLElement>('.dimension span')! };
}
type Proof = ReturnType<typeof proofOf>;
const studio = proofOf(screenshot);
const sample = proofOf(sheet);

// A plain fade: the marks and measurements carry the character, the image just arrives.
function reveal(target: HTMLElement, duration: number, delay = 0) {
  const fade = target.animate([{ opacity: 0 }, { opacity: 1 }], { duration, delay, easing: 'ease-out', fill: 'both' });
  target.classList.remove('pending');
  return fade;
}

// Register, reveal, measure: each figure runs this once, the first time it is on screen.
async function develop(figure: Proof, target: HTMLElement, duration: number) {
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

// The studio dimension reports the figure as drawn right now, so it stays honest on resize.
new ResizeObserver(([entry]) => {
  studio.label.textContent = `${Math.round(entry.contentRect.width)} px`;
}).observe(studio.wrap);

// The hero command is set as plain text (typing, theme picker); colour it whenever that happens.
new MutationObserver(() => {
  if (!commandText.firstElementChild) commandText.innerHTML = shell(commandText.textContent!);
}).observe(commandText, { childList: true });

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

const tiles = [...document.querySelectorAll<HTMLElement>('.showcase-tile')].map(tile => {
  const image = tile.querySelector<HTMLImageElement>('img')!;
  return { tile, image, proof: proofOf(image) };
});

if (motion) {
  // Hold the box at its finished size so typing never reflows the hero.
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
  for (const { image } of tiles) image.classList.add('pending');
  whenSeen(document.querySelector('.showcase')!, .25, () => tiles.forEach(({ image, proof }, i) => {
    setTimeout(() => develop(proof, image, 260), i * 90);
  }));
} else {
  for (const figure of [studio, sample, ...tiles.map(t => t.proof)]) figure.wrap.classList.add('marked', 'measured');
}

// Theme picker: one marker slides to the chosen row, and the stage label names the theme.
const options = document.querySelector<HTMLElement>('.options')!;
const marker = options.querySelector<HTMLElement>('.option-marker')!;
const stageTheme = document.querySelector<HTMLElement>('#stage-theme')!;
function select(animate: boolean) {
  const row = options.querySelector('input:checked')!.closest<HTMLElement>('.option')!;
  marker.style.transition = animate && motion ? '' : 'none';
  marker.style.transform = `translateY(${row.offsetTop}px)`;
  marker.style.height = `${row.offsetHeight}px`;
  stageTheme.textContent = row.querySelector('.option-name > span')!.textContent;
}
new ResizeObserver(() => select(false)).observe(options);
select(false);

// Theme change: marks re-register while the new proof fades in over the old one.
let base = images.find(image => !image.hidden)!;
let running: Animation | null = null;
for (const input of options.querySelectorAll<HTMLInputElement>('input')) {
  input.addEventListener('change', () => {
    finishTyping();
    select(true);
    // Let the picker script swap `hidden` first, then restage both layers.
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
