// Menus shared by the composer, task rows and the task modal.

import { Flag, Hash, Inbox } from "@lucide/svelte";
import { PRIORITY, hex } from "./colors.js";
import { app } from "./store.svelte.js";

export function priorityItems(current, onpick) {
  return PRIORITY.map((p) => ({
    label: p.label,
    icon: Flag,
    color: p.color,
    checked: current === p.value,
    action: () => onpick(p.value),
  }));
}

/** Inbox, then each class with its sections indented. */
export function placeItems(current, onpick) {
  const items = [{ label: "Inbox", icon: Inbox, checked: current.class_id == null && current.section_id == null, action: () => onpick({ class_id: null, section_id: null }) }];
  for (const s of app.sections.filter((s) => s.class_id == null)) {
    items.push({ label: `   ${s.name}`, checked: current.section_id === s.id, action: () => onpick({ class_id: null, section_id: s.id }) });
  }
  const classes = app.classes.filter((c) => !c.archived);
  if (classes.length) items.push({ heading: "Classes" });
  for (const c of classes) {
    items.push({ label: c.name, icon: Hash, color: hex(c.color), checked: current.class_id === c.id && current.section_id == null, action: () => onpick({ class_id: c.id, section_id: null }) });
    for (const s of app.sections.filter((s) => s.class_id === c.id)) {
      items.push({ label: `   ${s.name}`, checked: current.section_id === s.id, action: () => onpick({ class_id: c.id, section_id: s.id }) });
    }
  }
  return items;
}

export function placeLabel(class_id, section_id) {
  const c = app.classes.find((x) => x.id === class_id);
  const s = app.sections.find((x) => x.id === section_id);
  const base = c ? c.name : "Inbox";
  return s ? `${base} / ${s.name}` : base;
}
