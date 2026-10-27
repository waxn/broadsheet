// What is being dragged right now (tasks, pages, classes, sections), shared by all drop targets.

export const drag = $state({ kind: null, id: null });

export function startDrag(e, kind, id) {
  drag.kind = kind;
  drag.id = id;
  e.dataTransfer.effectAllowed = "move";
  e.dataTransfer.setData("text/plain", `${kind}:${id}`);
}

export function endDrag() {
  drag.kind = null;
  drag.id = null;
}

/** Whether a drop on this element lands in its top or bottom half. */
export function half(e) {
  const r = e.currentTarget.getBoundingClientRect();
  return e.clientY < r.top + r.height / 2 ? "before" : "after";
}

/** Renumbers a list after moving `id` before/after `targetId` (or to the end). */
export function moved(list, id, targetId, where) {
  const ids = list.map((x) => x.id).filter((x) => x !== id);
  let at = targetId == null ? ids.length : ids.indexOf(targetId) + (where === "after" ? 1 : 0);
  if (at < 0) at = ids.length;
  ids.splice(at, 0, id);
  return ids.map((x, position) => ({ id: x, position }));
}
