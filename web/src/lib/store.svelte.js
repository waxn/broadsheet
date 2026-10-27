// The app's shared state and every action that changes it. Changes are optimistic: the
// screen updates at once and rolls back with a notice if the server refuses.

import { ApiError, del, get, patch, post, qs } from "./api.js";
import * as D from "./dates.js";
import { fail, toast } from "./toast.svelte.js";

export const app = $state({
  user: null,
  loaded: false,
  classes: [],
  sections: [],
  todos: [],
  pages: [],
  habits: [],
  checks: [],
  countdowns: [],
  events: [],
  // UI
  sidebar: localGet("sidebar", true),
  focus: false,
  quickAdd: null, // null or { defaults }
  detail: null, // task id shown in the task modal
  palette: false,
  help: false,
  eventEditor: null, // null or { event } (new events have no id)
  selected: null, // task id highlighted for keyboard actions
  datePickerFor: null, // { id, anchor } while the d shortcut picker is open
});

function localGet(key, fallback) {
  try {
    const v = localStorage.getItem(`bs.${key}`);
    return v === null ? fallback : JSON.parse(v);
  } catch {
    return fallback;
  }
}

export function localSet(key, value) {
  try {
    localStorage.setItem(`bs.${key}`, JSON.stringify(value));
  } catch {
    /* storage unavailable */
  }
}

export { localGet };

function replace(list, item) {
  const i = list.findIndex((x) => x.id === item.id);
  if (i >= 0) list[i] = { ...list[i], ...item };
  else list.push(item);
}

const parseRule = (t) => {
  if (!t.recurrence || typeof t.recurrence !== "string") return t;
  try {
    return { ...t, recurrence: JSON.parse(t.recurrence) };
  } catch {
    return { ...t, recurrence: null };
  }
};

// ---------- session ----------

export async function boot() {
  try {
    const data = await get(`/api/bootstrap?${qs({ today: D.today() })}`);
    app.user = data.user;
    app.classes = data.classes;
    app.sections = data.sections;
    app.todos = data.todos.map(parseRule);
    app.pages = data.pages;
    app.habits = data.habits;
    app.countdowns = data.countdowns;
  } catch (err) {
    if (!(err instanceof ApiError) || err.status !== 401) fail(err);
    app.user = null;
  }
  app.loaded = true;
}

export async function signOut() {
  await post("/api/logout").catch(() => {});
  location.href = "/";
}

// ---------- tasks ----------

export const todoById = (id) => app.todos.find((t) => t.id === id);
export const classById = (id) => app.classes.find((c) => c.id === id);
export const pageById = (id) => app.pages.find((p) => p.id === id);

export const childrenOf = (id) => app.todos.filter((t) => t.parent_id === id).sort((a, b) => a.position - b.position || a.id - b.id);

export async function createTodo(fields) {
  try {
    const t = parseRule(await post("/api/todos", fields));
    app.todos.push(t);
    bumpClassCount(t.class_id, 1);
    return t;
  } catch (err) {
    fail(err);
    return null;
  }
}

export async function updateTodo(id, fields) {
  const t = todoById(id);
  if (!t) return;
  const before = $state.snapshot(t);
  const local = { ...fields };
  if ("class_id" in fields) {
    const c = classById(fields.class_id);
    local.class_name = c?.name ?? null;
    local.class_color = c?.color ?? null;
    if (!("section_id" in fields)) local.section_id = null;
  }
  Object.assign(t, local);
  try {
    const saved = parseRule(await patch(`/api/todos/${id}`, fields));
    Object.assign(t, saved);
    if ("class_id" in fields || "parent_id" in fields || "section_id" in fields) {
      // Subtasks follow their parent on the server; mirror that here.
      for (const c of descendants(id)) Object.assign(c, { class_id: saved.class_id, section_id: saved.section_id, class_name: saved.class_name, class_color: saved.class_color });
      if (before.class_id !== saved.class_id) refreshClasses();
    }
    return t;
  } catch (err) {
    Object.assign(t, before);
    fail(err);
  }
}

function descendants(id) {
  const out = [];
  const walk = (pid) => {
    for (const c of app.todos.filter((x) => x.parent_id === pid)) {
      out.push(c);
      walk(c.id);
    }
  };
  walk(id);
  return out;
}

function bumpClassCount(classId, n) {
  const c = classById(classId);
  if (c) c.open_count = Math.max(0, (c.open_count || 0) + n);
}

/** Checks a task off (with Undo). Recurring tasks roll forward to their next date instead. */
export async function completeTodo(id) {
  const t = todoById(id);
  if (!t) return;
  const before = $state.snapshot(t);
  const kids = descendants(id);
  const kidsBefore = kids.map((k) => $state.snapshot(k));
  if (!t.recurrence) {
    t.done = 1;
    t.completed_at = new Date().toISOString();
    kids.forEach((k) => (k.done = 1));
    bumpClassCount(t.class_id, -1);
  }
  try {
    const res = await post(`/api/todos/${id}/complete`, { today: D.today() });
    Object.assign(t, parseRule(res.todo));
    if (res.recurred) kids.forEach((k) => (k.done = 0));
    toast(res.recurred ? `Next: ${D.label(res.todo.due_date)}` : "1 task completed", {
      action: async () => {
        Object.assign(t, before);
        kids.forEach((k, i) => Object.assign(k, kidsBefore[i]));
        if (!res.recurred) bumpClassCount(t.class_id, 1);
        await patch(`/api/todos/${id}`, res.recurred ? { due_date: before.due_date, start_at: before.start_at, end_at: before.end_at } : { done: false }).catch(fail);
        if (!res.recurred) for (const k of kidsBefore.filter((k) => !k.done)) patch(`/api/todos/${k.id}`, { done: false }).catch(() => {});
      },
    });
  } catch (err) {
    Object.assign(t, before);
    if (!t.recurrence) bumpClassCount(t.class_id, 1);
    fail(err);
  }
}

export async function uncompleteTodo(id) {
  const t = todoById(id);
  if (!t) return;
  t.done = 0;
  bumpClassCount(t.class_id, 1);
  await patch(`/api/todos/${id}`, { done: false }).catch(fail);
}

export async function toggleTodo(id) {
  const t = todoById(id);
  if (t?.done) return uncompleteTodo(id);
  return completeTodo(id);
}

export async function deleteTodo(id) {
  const t = todoById(id);
  if (!t) return;
  const gone = new Set([id, ...descendants(id).map((d) => d.id)]);
  const snapshot = app.todos.filter((x) => gone.has(x.id)).map((x) => $state.snapshot(x));
  app.todos = app.todos.filter((x) => !gone.has(x.id));
  if (!t.done) bumpClassCount(t.class_id, -1);
  if (app.detail === id) app.detail = null;
  try {
    await del(`/api/todos/${id}`);
    toast(`Deleted “${t.title}”`);
  } catch (err) {
    app.todos.push(...snapshot);
    fail(err);
  }
}

/** Saves a new order (and maybe a new place) for tasks after a drag. */
export async function reorderTodos(items) {
  for (const it of items) {
    const t = todoById(it.id);
    if (!t) continue;
    if ("class_id" in it && it.class_id !== t.class_id) {
      const c = classById(it.class_id);
      Object.assign(t, { class_name: c?.name ?? null, class_color: c?.color ?? null });
    }
    Object.assign(t, it);
  }
  try {
    await post("/api/reorder", { kind: "todos", items });
    if (items.some((i) => "class_id" in i)) refreshClasses();
  } catch (err) {
    fail(err);
  }
}

export async function loadCompleted() {
  try {
    const done = await get("/api/todos?done=1");
    for (const t of done) replace(app.todos, parseRule(t));
  } catch (err) {
    fail(err);
  }
}

// ---------- sections ----------

export async function createSection(fields) {
  try {
    const s = await post("/api/sections", fields);
    app.sections.push(s);
    return s;
  } catch (err) {
    fail(err);
  }
}

export async function updateSection(id, fields) {
  const s = app.sections.find((x) => x.id === id);
  if (s) Object.assign(s, fields);
  await patch(`/api/sections/${id}`, fields).catch(fail);
}

export async function deleteSection(id) {
  app.sections = app.sections.filter((s) => s.id !== id);
  app.todos = app.todos.filter((t) => t.section_id !== id);
  await del(`/api/sections/${id}`).catch(fail);
  refreshClasses();
}

export async function reorder(kind, items) {
  const list = { sections: app.sections, classes: app.classes, pages: app.pages, habits: app.habits }[kind];
  for (const it of items) {
    const x = list?.find((y) => y.id === it.id);
    if (x) Object.assign(x, it);
  }
  await post("/api/reorder", { kind, items }).catch(fail);
}

// ---------- classes ----------

export async function refreshClasses() {
  try {
    app.classes = await get("/api/classes");
  } catch {
    /* keep what we have */
  }
}

export async function createClass(fields) {
  try {
    const c = await post("/api/classes", fields);
    await Promise.all([refreshClasses(), refreshPages()]);
    return c;
  } catch (err) {
    fail(err);
  }
}

export async function updateClass(id, fields) {
  const c = classById(id);
  if (c) Object.assign(c, fields);
  try {
    await patch(`/api/classes/${id}`, fields);
    if ("color" in fields || "name" in fields) {
      for (const t of app.todos.filter((t) => t.class_id === id)) Object.assign(t, { class_name: c.name, class_color: c.color });
    }
  } catch (err) {
    fail(err);
    refreshClasses();
  }
}

export async function deleteClass(id) {
  try {
    await del(`/api/classes/${id}`);
    app.classes = app.classes.filter((c) => c.id !== id);
    for (const t of app.todos.filter((t) => t.class_id === id)) Object.assign(t, { class_id: null, section_id: null, class_name: null, class_color: null });
    app.sections = app.sections.filter((s) => s.class_id !== id);
    app.pages = app.pages.filter((p) => p.class_id !== id);
    refreshPages();
  } catch (err) {
    fail(err);
  }
}

// ---------- pages ----------

export async function refreshPages() {
  try {
    app.pages = await get("/api/pages");
  } catch {
    /* keep what we have */
  }
}

export async function createPage(fields = {}) {
  try {
    const p = await post("/api/pages", fields);
    const { content, body, text, ...meta } = p;
    app.pages.push(meta);
    return p;
  } catch (err) {
    fail(err);
  }
}

export async function updatePage(id, fields) {
  const p = pageById(id);
  const { content, text, links, body, ...meta } = fields;
  if (p) Object.assign(p, meta);
  try {
    const saved = await patch(`/api/pages/${id}`, fields);
    if (p) Object.assign(p, { updated_at: saved.updated_at });
    return saved;
  } catch (err) {
    fail(err);
    throw err;
  }
}

export async function trashPage(id) {
  await updatePage(id, { archived: true }).catch(() => {});
  await refreshPages();
  toast("Moved to trash", { action: () => restorePage(id) });
}

export async function restorePage(id) {
  await patch(`/api/pages/${id}`, { archived: false }).catch(fail);
  await refreshPages();
}

export async function deletePage(id) {
  await del(`/api/pages/${id}`).catch(fail);
}

/** Pages under a parent (or at the top level of the workspace when parent is null). */
export function pageChildren(parentId) {
  return app.pages
    .filter((p) => (parentId == null ? p.parent_id == null && p.todo_id == null && p.class_id == null : p.parent_id === parentId))
    .sort((a, b) => a.position - b.position || a.id - b.id);
}

// ---------- events ----------

let loadedRanges = [];

export async function loadEvents(from, to, force = false) {
  if (!force && loadedRanges.some(([a, b]) => a <= from && b >= to)) return;
  try {
    const list = await get(`/api/events?${qs({ from, to })}`);
    for (const e of list) replace(app.events, e);
    loadedRanges.push([from, to]);
  } catch (err) {
    fail(err);
  }
}

export async function createEvent(fields) {
  try {
    const e = await post("/api/events", fields);
    app.events.push(e);
    if (e.countdown) refreshCountdowns();
    return e;
  } catch (err) {
    fail(err);
  }
}

export async function updateEvent(id, fields) {
  const e = app.events.find((x) => x.id === id);
  const before = e && $state.snapshot(e);
  if (e) Object.assign(e, fields);
  try {
    const saved = await patch(`/api/events/${id}`, fields);
    if (e) Object.assign(e, saved);
    refreshCountdowns();
    return saved;
  } catch (err) {
    if (e) Object.assign(e, before);
    fail(err);
  }
}

export async function deleteEvent(id) {
  const e = app.events.find((x) => x.id === id);
  app.events = app.events.filter((x) => x.id !== id);
  try {
    await del(`/api/events/${id}`);
    refreshCountdowns();
    toast(`Deleted “${e?.title}”`, {
      action: async () => {
        if (!e) return;
        const { id: _id, created_at, updated_at, class_name, class_color, ...fields } = e;
        await createEvent(fields);
      },
    });
  } catch (err) {
    if (e) app.events.push(e);
    fail(err);
  }
}

export async function refreshCountdowns() {
  try {
    app.countdowns = await get(`/api/countdowns?${qs({ today: D.today() })}`);
  } catch {
    /* keep */
  }
}

// ---------- habits ----------

export async function loadHabits() {
  try {
    const data = await get("/api/habits");
    app.habits = data.habits;
    app.checks = data.checks;
  } catch (err) {
    fail(err);
  }
}

export const isChecked = (habitId, day) => app.checks.some((c) => c.habit_id === habitId && c.day === day);

export async function toggleHabit(habitId, day) {
  const done = !isChecked(habitId, day);
  if (done) app.checks.push({ habit_id: habitId, day });
  else app.checks = app.checks.filter((c) => !(c.habit_id === habitId && c.day === day));
  await post(`/api/habits/${habitId}/check`, { day, done }).catch(fail);
}

export async function createHabit(fields) {
  try {
    const h = await post("/api/habits", fields);
    app.habits.push(h);
    return h;
  } catch (err) {
    fail(err);
  }
}

export async function updateHabit(id, fields) {
  const h = app.habits.find((x) => x.id === id);
  if (h) Object.assign(h, fields);
  await patch(`/api/habits/${id}`, fields).catch(fail);
}

export async function deleteHabit(id) {
  app.habits = app.habits.filter((h) => h.id !== id);
  app.checks = app.checks.filter((c) => c.habit_id !== id);
  await del(`/api/habits/${id}`).catch(fail);
}
