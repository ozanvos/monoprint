export {};

const motion = document.documentElement.classList.contains('motion');
const GRID = 14;

// Wraps a figure so crop marks and a dimension line can sit just outside its trim.
function proof(figure: HTMLElement, measurement: string) {
  const wrap = document.createElement('div');
  wrap.className = 'proof';
  figure.replaceWith(wrap);
  wrap.append(figure);
  const crop = document.createElement('div');
  crop.className = 'crop';
  crop.setAttribute('aria-hidden', 'true');
  crop.innerHTML = '<i></i>'.repeat(8);
  const dimension = document.createElement('div');
  dimension.className = 'dimension';
  dimension.setAttribute('aria-hidden', 'true');
  dimension.innerHTML = `<span>${measurement}</span>`;
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

const appSection = document.querySelector<HTMLElement>('.app-section')!;
const stage = document.querySelector<HTMLElement>('.stage')!;
const studio = proof(appSection.querySelector('.app-screenshot')!, '');
const sample = proof(stage.querySelector('.sheet')!, '640 pt × 2 = 1280 px');
label(appSection, 'Fig. 1 <b>Studio</b> · macOS');
label(stage, 'Fig. 2 <b>share.rs</b>');

// The studio dimension reports the figure as it is drawn right now, so it stays honest on resize.
new ResizeObserver(([entry]) => {
  studio.label.textContent = `${Math.round(entry.contentRect.width)} px`;
}).observe(studio.wrap);

const marker = new IntersectionObserver(entries => {
  for (const entry of entries) {
    if (!entry.isIntersecting) continue;
    entry.target.classList.add('marked');
    marker.unobserve(entry.target);
  }
}, { threshold: .35 });
for (const wrap of [studio.wrap, sample.wrap]) motion ? marker.observe(wrap) : wrap.classList.add('marked');

// Re-registering on theme change: marks pull back and close in again around the new proof.
for (const input of document.querySelectorAll<HTMLInputElement>('input[name="theme"]')) {
  input.addEventListener('change', () => {
    if (!motion) return;
    setTimeout(() => {
      const image = stage.querySelector<HTMLImageElement>(`img[data-theme="${input.value}"]`)!;
      image.classList.remove('fading');
      void image.offsetWidth;
      image.classList.add('fading');
    });
    sample.wrap.querySelector('.crop')!.animate([{ transform: 'scale(1.025)', opacity: .4 }, { transform: 'none', opacity: 1 }], { duration: 420, easing: 'cubic-bezier(.2,.7,.2,1)' });
  });
}

// A crosshair that snaps to the lattice, shown only where the pointer is over open grid.
if (matchMedia('(hover: hover) and (pointer: fine)').matches) {
  for (const section of [appSection, stage]) {
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
