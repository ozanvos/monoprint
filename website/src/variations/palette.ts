export {};

const motion = document.documentElement.classList.contains('motion');
const live = document.querySelector<HTMLElement>('.live')!;
const sample = live.querySelector<HTMLElement>('.sample')!;
const flag = live.querySelector<HTMLElement>('#flag')!;
const timer = live.querySelector<HTMLElement>('.timer')!;
const toggle = live.querySelector<HTMLButtonElement>('#cycle')!;
const commandText = document.querySelector<HTMLElement>('#command-text')!;
const palettes: Record<string, Record<string, string>> = JSON.parse(live.dataset.palettes!);
const themes: { id: string; cli: string }[] = JSON.parse(live.dataset.themes!);
const inputs = [...document.querySelectorAll<HTMLInputElement>('input[name="theme"]')];
let current = themes.findIndex(t => t.id === inputs.find(i => i.checked)!.value);

function apply(index: number) {
  current = index;
  const theme = themes[index];
  for (const [key, value] of Object.entries(palettes[theme.id])) sample.style.setProperty(`--${key}`, value);
  commandText.textContent = inputs.find(i => i.value === theme.id)!.dataset.command!;
  // The flag value slides out and the new one in, the only movement besides colour.
  if (!motion) { flag.textContent = theme.cli; return; }
  const out = flag.animate([{ opacity: 1, transform: 'none' }, { opacity: 0, transform: 'translateY(-4px)' }], { duration: 160, easing: 'ease-in' });
  out.finished.then(() => {
    flag.textContent = theme.cli;
    flag.animate([{ opacity: 0, transform: 'translateY(4px)' }, { opacity: 1, transform: 'none' }], { duration: 220, easing: 'cubic-bezier(.2,.7,.2,1)' });
  });
}

function restartTimer() {
  timer.classList.remove('running');
  void timer.offsetWidth;
  timer.classList.add('running');
}

function setPaused(paused: boolean) {
  live.classList.toggle('paused', paused);
  toggle.setAttribute('aria-pressed', String(paused));
  toggle.setAttribute('aria-label', paused ? 'Resume theme cycle' : 'Pause theme cycle');
}

timer.addEventListener('animationend', () => {
  apply((current + 1) % themes.length);
  restartTimer();
});

toggle.addEventListener('click', () => {
  const paused = !live.classList.contains('paused');
  setPaused(paused);
  if (!paused && !timer.classList.contains('running')) restartTimer();
});

// Choosing a theme yourself takes over: the sample follows the picker and stops cycling.
for (const input of inputs) {
  input.addEventListener('change', () => {
    setPaused(true);
    apply(themes.findIndex(t => t.id === input.value));
    setTimeout(() => {
      for (const image of document.querySelectorAll<HTMLImageElement>('.stage img')) image.classList.toggle('arriving', image.dataset.theme === input.value);
    });
  });
}

if (motion) restartTimer();
else setPaused(true);
