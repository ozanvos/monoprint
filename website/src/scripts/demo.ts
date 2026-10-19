const feedback = document.querySelector<HTMLElement>('#feedback')!;
const commandText = document.querySelector<HTMLElement>('#command-text')!;
const copy = document.querySelector<HTMLButtonElement>('#copy-command')!;
const check = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m5 12 4 4L19 6"/></svg>';

// The theme picker swaps the rendered sample and the CLI command together.
for (const input of document.querySelectorAll<HTMLInputElement>('input[name="theme"]')) {
  input.addEventListener('change', () => {
    for (const image of document.querySelectorAll<HTMLImageElement>('.stage img')) image.hidden = image.dataset.theme !== input.value;
    commandText.textContent = input.dataset.command!;
  });
}

const original = copy.innerHTML;
let timer: ReturnType<typeof setTimeout>;
copy.addEventListener('click', async () => {
  if (copy.disabled) return;
  clearTimeout(timer);
  copy.disabled = true;
  try {
    await navigator.clipboard.writeText(commandText.textContent!);
    copy.innerHTML = check;
    copy.title = 'Copied';
    feedback.textContent = 'CLI command copied.';
  } catch {
    copy.title = 'Copy unavailable. Select the command to copy it.';
    feedback.textContent = copy.title;
  } finally {
    timer = setTimeout(() => {
      copy.innerHTML = original;
      copy.disabled = false;
      copy.title = copy.getAttribute('aria-label') ?? 'Copy';
    }, 1800);
  }
});
