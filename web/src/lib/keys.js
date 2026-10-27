// Keyboard helpers: single-key shortcuts only fire when nothing is being typed and no dialog is open.

import { app } from "./store.svelte.js";
import { menu } from "./menu.svelte.js";

export function isTyping(e) {
  const t = e.target;
  return !!t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));
}

export function overlayOpen() {
  return !!(app.quickAdd || app.detail || app.palette || app.help || app.eventEditor || menu.open || document.querySelector(".backdrop, .popover"));
}

/** True when a plain-key shortcut should run for this event. */
export function shortcut(e) {
  return !e.defaultPrevented && !e.ctrlKey && !e.metaKey && !e.altKey && !isTyping(e) && !overlayOpen();
}

export const mod = (e) => e.ctrlKey || e.metaKey;
