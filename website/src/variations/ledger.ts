export {};

const motion = document.documentElement.classList.contains('motion');
const header = document.querySelector<HTMLElement>('header.strip')!;
const screenshot = document.querySelector<HTMLElement>('.app-screenshot')!;
const appSection = document.querySelector<HTMLElement>('.app-section')!;

// Reading position, drawn on the header's bottom rule.
const reading = document.createElement('div');
reading.className = 'reading';
reading.setAttribute('aria-hidden', 'true');
header.append(reading);

let frame = 0;
function update() {
  frame = 0;
  const max = document.documentElement.scrollHeight - innerHeight;
  reading.style.transform = `scaleX(${max > 0 ? Math.min(1, scrollY / max) : 0})`;
  if (!motion) return;
  // 1 while the screenshot's top is at the viewport's bottom, 0 once it has risen a third of the way.
  const top = appSection.getBoundingClientRect().top;
  const settle = Math.max(0, Math.min(1, (top - innerHeight * .62) / (innerHeight * .38)));
  screenshot.style.setProperty('--settle', settle.toFixed(3));
}
addEventListener('scroll', () => { frame ||= requestAnimationFrame(update); }, { passive: true });
addEventListener('resize', update);
update();

if (motion) {
  // Mark entries per section so they stagger in reading order.
  const groups: [string, string][] = [
    ['.hero', '.eyebrow, h1, .lede, .actions'],
    ['.themes', '.picker > *'],
    ['.parts', 'article > *'],
    ['.showcase', 'figure'],
    ['.downloads', ':scope > *'],
  ];
  for (const [section, selector] of groups) {
    document.querySelectorAll<HTMLElement>(`${section} ${selector}`).forEach((el, i) => {
      el.classList.add('entry');
      el.style.setProperty('--i', String(i));
    });
  }

  const seen = new IntersectionObserver(entries => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      entry.target.classList.add('seen');
      seen.unobserve(entry.target);
      if (entry.target.classList.contains('parts')) typeAll(entry.target as HTMLElement);
    }
  }, { threshold: .2, rootMargin: '0px 0px -8% 0px' });
  for (const section of document.querySelectorAll('main > section')) seen.observe(section);
  requestAnimationFrame(() => document.querySelector('.hero')!.classList.add('seen'));

  // Each card's code is laid out at full size up front, so typing never moves the page.
  for (const pre of document.querySelectorAll<HTMLElement>('.parts pre')) {
    const lines = pre.textContent!.split('\n');
    pre.style.minHeight = `${pre.offsetHeight}px`;
    pre.classList.add('typed');
    pre.setAttribute('aria-label', pre.textContent!);
    pre.innerHTML = '';
    lines.forEach((text, i) => {
      const line = document.createElement('span');
      line.className = 'line';
      line.textContent = text;
      line.setAttribute('aria-hidden', 'true');
      pre.append(line);
      if (i < lines.length - 1) pre.append('\n');
    });
  }
}

function typeAll(parts: HTMLElement) {
  parts.querySelectorAll<HTMLElement>('pre.typed').forEach((pre, column) => {
    const lines = [...pre.querySelectorAll<HTMLElement>('.line')];
    const caret = document.createElement('span');
    caret.className = 'caret';
    let index = 0;
    const next = () => {
      const line = lines[index];
      if (!line) { setTimeout(() => caret.remove(), 900); return; }
      line.after(caret);
      const chars = line.textContent!.length;
      const width = line.scrollWidth;
      const speed = 14;
      line.animate([{ maxWidth: '0px' }, { maxWidth: `${width}px` }], { duration: chars * speed, easing: `steps(${Math.max(chars, 1)})`, fill: 'forwards' })
        .finished.then(() => { line.classList.add('done'); index++; setTimeout(next, 140); });
    };
    setTimeout(next, 450 + column * 220);
  });
}

for (const input of document.querySelectorAll<HTMLInputElement>('input[name="theme"]')) {
  input.addEventListener('change', () => setTimeout(() => {
    for (const image of document.querySelectorAll<HTMLImageElement>('.stage img')) image.classList.toggle('arriving', image.dataset.theme === input.value);
  }));
}
