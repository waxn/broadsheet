// One context menu at a time, opened from anywhere: openMenu(eventOrElement, items).
// An item is { label, icon, hint, action, danger, checked } or { sep: true } or { heading }.

export const menu = $state({ open: false, x: 0, y: 0, items: [] });

export function openMenu(at, items) {
  let x, y;
  if (at instanceof Event) {
    at.preventDefault();
    at.stopPropagation();
    if (at.type === "contextmenu" || !at.currentTarget?.getBoundingClientRect) [x, y] = [at.clientX, at.clientY];
    else {
      const r = at.currentTarget.getBoundingClientRect();
      [x, y] = [r.left, r.bottom + 4];
    }
  } else if (at?.getBoundingClientRect) {
    const r = at.getBoundingClientRect();
    [x, y] = [r.left, r.bottom + 4];
  } else [x, y] = [at.x, at.y];
  Object.assign(menu, { open: true, x, y, items });
}

export function closeMenu() {
  menu.open = false;
}
