// Keeps a floating element inside the viewport, next to an anchor rectangle.

export function place(node, { anchor, align = "start", gap = 6 }) {
  function update(opts) {
    const a = opts.anchor;
    if (!a) return;
    const r = a.getBoundingClientRect ? a.getBoundingClientRect() : a;
    const w = node.offsetWidth;
    const h = node.offsetHeight;
    let left = opts.align === "end" ? r.right - w : r.left;
    let top = r.bottom + (opts.gap ?? gap);
    if (top + h > innerHeight - 8) top = Math.max(8, r.top - h - (opts.gap ?? gap));
    left = Math.max(8, Math.min(left, innerWidth - w - 8));
    node.style.left = `${left}px`;
    node.style.top = `${top}px`;
  }
  requestAnimationFrame(() => update({ anchor, align, gap }));
  return { update };
}

/** Calls `handler` on a pointer press outside `node`. */
export function outside(node, handler) {
  const down = (e) => {
    if (!node.contains(e.target)) handler(e);
  };
  setTimeout(() => document.addEventListener("pointerdown", down, true));
  return {
    update(h) {
      handler = h;
    },
    destroy() {
      document.removeEventListener("pointerdown", down, true);
    },
  };
}

export function autosize(node) {
  const fit = () => {
    node.style.height = "auto";
    node.style.height = `${node.scrollHeight}px`;
  };
  node.addEventListener("input", fit);
  requestAnimationFrame(fit);
  return { update: () => requestAnimationFrame(fit), destroy: () => node.removeEventListener("input", fit) };
}

export function focusOnMount(node, select = false) {
  requestAnimationFrame(() => {
    node.focus();
    if (select) node.select?.();
  });
}
