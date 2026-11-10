let host: HTMLDivElement | null = null;

function ensureHost(): HTMLDivElement {
  if (host && document.body.contains(host)) return host;
  host = document.createElement('div');
  host.className = 'pos-toast-host';
  document.body.appendChild(host);
  return host;
}

export function toast(message: string, type: 'success' | 'error' = 'success'): void {
  const el = document.createElement('div');
  el.className = `pos-toast pos-toast--${type}`;
  el.textContent = message;
  ensureHost().appendChild(el);
  window.setTimeout(() => {
    el.classList.add('is-leaving');
    window.setTimeout(() => el.remove(), 200);
  }, 2200);
}
