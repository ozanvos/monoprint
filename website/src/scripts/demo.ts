import { initialTheme, command } from '../data/demo.mjs';

const feedback = document.querySelector<HTMLElement>('#feedback')!;
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
      button.innerHTML = check;
      button.title = 'Copied';
      feedback.textContent = message;
    } catch {
      button.title = 'Copy unavailable. Select the command to copy it.';
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
copyAction(document.querySelector<HTMLButtonElement>('#copy-command')!, () => navigator.clipboard.writeText(command(initialTheme)), 'CLI command copied.');
