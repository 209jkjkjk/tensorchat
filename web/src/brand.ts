/** Runtime product name supplied by the server before the UI is rendered. */

let name = 'TensorChat';

export function siteName(): string {
  return name;
}

export function setSiteName(next: string): void {
  name = next;
  document.title = next;
}
