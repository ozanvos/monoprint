export {};

const motion = document.documentElement.classList.contains('motion');
const ease = 'cubic-bezier(.45,0,.25,1)';
const commandText = document.querySelector<HTMLElement>('#command-text')!;
const screenshot = document.querySelector<HTMLElement>('.app-screenshot')!;
const sheet = document.querySelector<HTMLElement>('.stage .sheet')!;
const images = [...sheet.querySelectorAll<HTMLImageElement>('img')];

function head(parent: HTMLElement) {
  const line = document.createElement('div');
  line.className = 'print-head';
  parent.append(line);
  return line;
}

// Reveals `target` top to bottom; `line`, when given, rides the leading edge.
function print(target: HTMLElement, line: HTMLElement | null, duration: number, delay = 0) {
  const timing: KeyframeAnimationOptions = { duration, delay, easing: ease, fill: 'both' };
  const sweep = target.animate([{ clipPath: 'inset(0 0 100% 0)' }, { clipPath: 'inset(0 0 0% 0)' }], timing);
  const scan = line?.animate([{ top: '0%', opacity: 0 }, { opacity: 1, offset: .05 }, { opacity: 1, offset: .9 }, { top: '100%', opacity: 0 }], timing);
  target.classList.remove('pending');
  return { sweep, scan, cancel() { sweep.cancel(); scan?.cancel(); } };
}

// Hero: the command types itself, reports its output, then the result prints.
const status = document.createElement('p');
status.className = 'render-status';
status.setAttribute('aria-hidden', 'true');
status.innerHTML = '<span>→ <b>share.png</b></span><span>1280 × 666 px</span><span>rendered on this machine</span>';
document.querySelector('.hero-copy')!.append(status);

const frame = document.createElement('div');
frame.className = 'print-frame';
screenshot.replaceWith(frame);
frame.append(screenshot);
const screenshotHead = head(frame);

function printScreenshot() {
  const run = print(screenshot, screenshotHead, 1300);
  run.sweep.finished.then(() => { screenshot.classList.add('printed'); run.cancel(); }, () => {});
}

const full = commandText.textContent!;
let typing = 0;
function finishTyping() {
  if (!typing) return;
  clearInterval(typing);
  typing = 0;
  commandText.classList.remove('typing');
  if (commandText.textContent!.length < full.length && full.startsWith(commandText.textContent!)) commandText.textContent = full;
  status.classList.add('shown');
}

if (motion) {
  // Hold the box at its finished size so typing never reflows the hero.
  const box = commandText.closest<HTMLElement>('.cli-command')!;
  box.style.minWidth = `min(100%, ${box.getBoundingClientRect().width}px)`;
  commandText.textContent = '';
  commandText.classList.add('typing');
  let i = 0;
  setTimeout(() => {
    typing = window.setInterval(() => {
      commandText.textContent = full.slice(0, ++i);
      if (i < full.length) return;
      setTimeout(() => {
        finishTyping();
        // Print the screenshot once it is actually on screen, not before.
        new IntersectionObserver((entries, observer) => {
          if (!entries[0].isIntersecting) return;
          observer.disconnect();
          setTimeout(printScreenshot, 120);
        }, { threshold: .08 }).observe(frame);
      }, 260);
    }, 15);
  }, 380);
  // Copying or picking a theme mid-type should never act on half a command.
  document.querySelector('#copy-command')!.addEventListener('pointerdown', finishTyping);
} else {
  status.classList.add('shown');
}

// Theme samples: the new one prints over the old, which stays put until covered.
const meta = document.createElement('p');
meta.className = 'stage-meta';
sheet.parentElement!.append(meta);
const sheetHead = head(sheet);
let base = images.find(image => !image.hidden)!;
let running: ReturnType<typeof print> | null = null;

function describe(input: HTMLInputElement) {
  meta.textContent = `share.png · --theme ${input.closest('label')!.querySelector('code')!.textContent} · 1280 × 666`;
}
describe(document.querySelector<HTMLInputElement>('input[name="theme"]:checked')!);

for (const input of document.querySelectorAll<HTMLInputElement>('input[name="theme"]')) {
  input.addEventListener('change', () => {
    finishTyping();
    describe(input);
    // Let the shared picker script swap `hidden` first, then restage both layers.
    setTimeout(() => {
      const next = images.find(image => image.dataset.theme === input.value)!;
      running?.cancel();
      for (const image of images) {
        image.hidden = image !== next && image !== base;
        image.classList.toggle('leaving', image === base && image !== next);
        image.classList.toggle('arriving', image === next);
      }
      if (!motion || next === base) {
        base = next;
        for (const image of images) image.hidden = image !== next;
        return;
      }
      const run = running = print(next, sheetHead, 760);
      run.sweep.finished.then(() => {
        base = next;
        for (const image of images) image.hidden = image !== next;
        run.cancel();
        running = null;
      }, () => {});
    });
  });
}

// Export examples print in one after another as the row scrolls into view.
if (motion) {
  const tiles = [...document.querySelectorAll<HTMLImageElement>('.showcase-tile img')];
  for (const tile of tiles) tile.classList.add('pending');
  new IntersectionObserver((entries, observer) => {
    if (!entries.some(entry => entry.isIntersecting)) return;
    observer.disconnect();
    tiles.forEach((tile, i) => {
      const run = print(tile, null, 900, i * 160);
      run.sweep.finished.then(() => run.cancel(), () => {});
    });
  }, { threshold: .3 }).observe(document.querySelector('.showcase')!);
}
