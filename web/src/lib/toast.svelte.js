// Small notices at the bottom of the screen, optionally with an Undo-style action.

export const toasts = $state([]);
let seq = 0;

export function toast(message, { action, label = "Undo", timeout = 5000, error = false } = {}) {
  const id = ++seq;
  toasts.push({ id, message, action, label, error });
  setTimeout(() => dismiss(id), timeout);
  return id;
}

export function dismiss(id) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

export const fail = (err) => toast(err?.message || String(err), { error: true });
