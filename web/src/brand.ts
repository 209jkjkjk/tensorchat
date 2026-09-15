/** Runtime product name supplied by the server before the UI is rendered. */

let name = 'TensorChat';
let retention = '';

export function siteName(): string {
  return name;
}

export function setSiteName(next: string): void {
  name = next;
  document.title = next;
}

export function retentionLabel(): string {
  return retention;
}

export function setRetentionLabel(next: string | null): void {
  retention = next ?? '';
}
