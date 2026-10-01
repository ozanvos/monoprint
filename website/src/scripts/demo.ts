import { themes, initialTheme, command } from '../data/demo.mjs';

const image = (id: string) => document.querySelector<HTMLImageElement>(`#preview-${id}`)!;
const exportLink = document.querySelector<HTMLAnchorElement>('#export-image')!;
const feedback = document.querySelector<HTMLElement>('#feedback')!;
const dimensions = document.querySelector<HTMLElement>('#preview-dimensions')!;
let selected = initialTheme;
function updateDimensions() {
  const current = image(selected.id);
  dimensions.textContent = `${current.naturalWidth} × ${current.naturalHeight}`;
}
themes.forEach(theme => image(theme.id).addEventListener('load', updateDimensions));
if (image(selected.id).complete) updateDimensions();

document.querySelectorAll<HTMLButtonElement>('[data-theme]').forEach(button => {
  button.addEventListener('click', () => {
    selected = themes.find(theme => theme.id === button.dataset.theme)!;
    document.querySelectorAll<HTMLButtonElement>('[data-theme]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
    themes.forEach(theme => { image(theme.id).hidden = theme.id !== selected.id; });
    document.querySelector<HTMLElement>('#command-text')!.replaceChildren(
      document.createTextNode('monoprint share.rs --theme '),
      Object.assign(document.createElement('span'), { className: 'command-theme', textContent: selected.cli }),
      document.createTextNode(`${selected.appearance === 'light' ? ' --light' : ''} --highlight 4`),
    );
    exportLink.href = image(selected.id).src;
    exportLink.download = `monoprint-${selected.id}.png`;
    updateDimensions();
  });
});
const search = document.querySelector<HTMLInputElement>('#theme-search')!;
search.addEventListener('input', () => {
  const query = search.value.trim().toLowerCase();
  let matches = 0;
  document.querySelectorAll<HTMLButtonElement>('[data-theme]').forEach(button => {
    button.hidden = !button.textContent!.toLowerCase().includes(query);
    if (!button.hidden) matches++;
  });
  document.querySelector<HTMLElement>('#no-themes')!.hidden = matches !== 0;
});
const check = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m5 12 4 4L19 6"/></svg>';
function copyAction(button: HTMLButtonElement, action: () => Promise<void>, message: string) {
  const original = button.innerHTML;
  let timer: ReturnType<typeof setTimeout>;
  button.addEventListener('click', async () => {
    if (button.disabled) return;
    clearTimeout(timer);
    button.disabled = true;
    try {
      await action();
      button.innerHTML = check + (button.id === 'copy-image' ? '<span>Copied</span>' : '');
      button.title = 'Copied';
      feedback.textContent = message;
    } catch {
      button.title = button.id === 'copy-image' ? 'Copy unavailable. Use Export to save the image.' : 'Copy unavailable. Select the command to copy it.';
      feedback.textContent = button.title;
    } finally {
      timer = setTimeout(() => {
        button.innerHTML = original;
        button.disabled = false;
        button.title = button.getAttribute('aria-label') ?? 'Copy';
      }, 1800);
    }
  });
}
copyAction(document.querySelector<HTMLButtonElement>('#copy-command')!, () => navigator.clipboard.writeText(command(selected)), 'CLI command copied.');
copyAction(document.querySelector<HTMLButtonElement>('#copy-image')!, async () => {
  const response = await fetch(image(selected.id).src);
  if (!response.ok) throw new Error('Preview image unavailable');
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': await response.blob() })]);
}, 'Preview image copied.');
