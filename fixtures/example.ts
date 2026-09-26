// Turn source into something worth sharing.
type Theme = { name: string; dark: boolean };

export function render(theme: Theme): string {
  const scale = 4;
  return `${theme.name} at ${scale}x`;
}
