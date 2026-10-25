// Screens: to-do list, to-do detail, class pages, classes index, and sign in / sign up.

import { get, patch, post, del, store } from "./api.js";
import { mountDocs } from "./docs.js";
import {
  COLORS,
  PRIORITIES,
  confirmDialog,
  dayDiff,
  esc,
  fmtDue,
  formDialog,
  splitPane,
  toast,
} from "./ui.js";

const GROUP_ORDER = ["Overdue", "Today", "This week", "Later", "No date", "Completed"];
const FILTERS = [
  ["open", "Open"],
  ["done", "Done"],
  ["all", "All"],
];
const PRIORITY_KEYS = { 1: 3, 2: 2, 3: 1, 0: 0 }; // keys 1 = high, 2 = medium, 3 = low, 0 = none

function fromHtml(html) {
  const tpl = document.createElement("template");
  tpl.innerHTML = html.trim();
  return tpl.content.firstElementChild;
}

function keysRule(pairs) {
  return `<footer class="rule keys" aria-label="Keyboard shortcuts">${pairs
    .map(([k, label]) => `<span class="key"><kbd>${esc(k)}</kbd> ${esc(label)}</span>`)
    .join("")}</footer>`;
}

// ----- to-do rows and grouping -----

function sortTodos(items) {
  return [...items].sort((a, b) => {
    if (!!a.done !== !!b.done) return a.done ? 1 : -1;
    if (a.due_date !== b.due_date) {
      if (!a.due_date) return 1;
      if (!b.due_date) return -1;
      return a.due_date < b.due_date ? -1 : 1;
    }
    if (a.priority !== b.priority) return b.priority - a.priority;
    return a.id - b.id;
  });
}

function groupOf(t) {
  if (t.done) return "Completed";
  if (!t.due_date) return "No date";
  const d = dayDiff(t.due_date);
  if (d < 0) return "Overdue";
  if (d === 0) return "Today";
  if (d <= 7) return "This week";
  return "Later";
}

function dueTone(t) {
  if (!t.due_date || t.done) return "";
  const d = dayDiff(t.due_date);
  if (d < 0) return "overdue";
  if (d === 0) return "today";
  if (d <= 2) return "soon";
  return "";
}

function rowHtml(t, selectedId) {
  const p = PRIORITIES[t.priority] || PRIORITIES[0];
  const selected = t.id === selectedId;
  return `<div class="row${t.done ? " done" : ""}${selected ? " selected" : ""}" data-id="${t.id}" role="option" aria-selected="${selected}">
    <button type="button" class="check" data-act="check" aria-label="${t.done ? "Mark not done" : "Mark done"}" aria-pressed="${!!t.done}"></button>
    <div class="row-main">
      <div class="row-title">${esc(t.title)}</div>
      <div class="row-meta">
        ${t.class_name ? `<span class="tag c-${esc(t.class_color)}">${esc(t.class_name)}</span>` : ""}
        ${t.due_date ? `<span class="due ${dueTone(t)}">${esc(fmtDue(t.due_date))}</span>` : ""}
      </div>
    </div>
    ${p.value ? `<span class="prio ${p.cls}">${p.label}</span>` : ""}
  </div>`;
}

function listHtml(items, selectedId, grouped) {
  if (!items.length) {
    return `<p class="empty">Nothing here yet. <span class="hint">Press <kbd>n</kbd> to add one.</span></p>`;
  }
  const sorted = sortTodos(items);
  if (!grouped) return sorted.map((t) => rowHtml(t, selectedId)).join("");
  const buckets = new Map(GROUP_ORDER.map((g) => [g, []]));
  for (const t of sorted) buckets.get(groupOf(t)).push(t);
  return [...buckets]
    .filter(([, rows]) => rows.length)
    .map(
      ([label, rows]) =>
        `<div class="rule group"><span class="group-label">${label}</span><span class="group-count">${rows.length}</span></div>` +
        rows.map((t) => rowHtml(t, selectedId)).join(""),
    )
    .join("");
}

function moveSelection(listEl, delta, pick) {
  const rows = [...listEl.querySelectorAll(".row")];
  if (!rows.length) return;
  const idx = rows.findIndex((r) => r.classList.contains("selected"));
  const next = rows[Math.min(rows.length - 1, Math.max(0, idx === -1 ? 0 : idx + delta))];
  pick(Number(next.dataset.id));
  next.scrollIntoView({ block: "nearest" });
}

function refreshList(listEl, items, selectedId, grouped) {
  listEl.innerHTML = listHtml(items, selectedId, grouped);
}

function reflectSelection(listEl, id) {
  for (const row of listEl.querySelectorAll(".row")) {
    const on = Number(row.dataset.id) === id;
    row.classList.toggle("selected", on);
    row.setAttribute("aria-selected", String(on));
  }
}

// Update one to-do in the shared store, then repaint. Reverts if the server refuses.
async function updateTodo(id, fields, repaint) {
  const todo = store.todos.find((t) => t.id === id);
  if (!todo) return;
  const before = { ...todo };
  Object.assign(todo, fields);
  if ("class_id" in fields) {
    const cls = store.classes.find((c) => c.id === fields.class_id);
    todo.class_name = cls?.name ?? null;
    todo.class_color = cls?.color ?? null;
  }
  if ("done" in fields) todo.done = fields.done ? 1 : 0;
  repaint();
  try {
    Object.assign(todo, await patch(`/api/todos/${id}`, fields));
    repaint();
  } catch (err) {
    Object.assign(todo, before);
    repaint();
    toast(err.message, "error");
  }
}

// ----- to-dos -----

export function renderTodos(main, selectedId) {
  let filter = "open";
  try {
    filter = localStorage.getItem("bs.filter") || filter;
  } catch {
    /* storage unavailable */
  }
  let selected = selectedId ? Number(selectedId) : null;
  let docs = null;

  const left = fromHtml(`<section class="pane list-pane">
      <header class="pane-head">
        <h1 class="pane-title">To-do</h1>
        <div class="seg-tabs" role="tablist" aria-label="Filter">
          ${FILTERS.map(([key, label]) => `<button type="button" role="tab" data-filter="${key}">${label}</button>`).join("")}
        </div>
      </header>
      <form class="quick-add" autocomplete="off">
        <input name="title" maxlength="300" placeholder="Add a to-do…" aria-label="New to-do">
        <input name="due" type="date" title="Due date" aria-label="Due date">
        <button class="btn" type="submit">Add</button>
      </form>
      <div class="list" role="listbox" aria-label="To-dos"></div>
      ${keysRule([
        ["n", "new"],
        ["j k", "move"],
        ["⏎", "open"],
        ["x", "done"],
        ["1 2 3 0", "priority"],
        ["d", "due"],
        ["/", "search all"],
        ["?", "help"],
      ])}
    </section>`);
  const right = fromHtml(`<section class="pane detail-pane"></section>`);
  const listEl = left.querySelector(".list");
  const quickAdd = left.querySelector(".quick-add");

  const visible = () => store.todos.filter((t) => (filter === "open" ? !t.done : filter === "done" ? t.done : true));

  function paintFilters() {
    for (const b of left.querySelectorAll("[data-filter]")) {
      const on = b.dataset.filter === filter;
      b.classList.toggle("on", on);
      b.setAttribute("aria-selected", String(on));
    }
  }

  function renderList() {
    refreshList(listEl, visible(), selected, filter !== "done");
    paintFilters();
  }

  function repaintAll() {
    renderList();
    paintDetail();
  }

  function paintDetail() {
    const todo = store.todos.find((t) => t.id === selected);
    const root = right.querySelector(".detail");
    if (!todo || !root) return;
    root.classList.toggle("done", !!todo.done);
    root.querySelector(".kicker").innerHTML = todo.class_name
      ? `<span class="tag c-${esc(todo.class_color)}">${esc(todo.class_name)}</span> <span class="kicker-text">Assignment</span>`
      : `<span class="kicker-text">To-do</span>`;
    root.querySelector(".due-input").value = todo.due_date || "";
    root.querySelector(".class-input").value = todo.class_id ?? "";
    for (const b of root.querySelectorAll("[data-prio]")) {
      const on = Number(b.dataset.prio) === todo.priority;
      b.classList.toggle("on", on);
      b.setAttribute("aria-checked", String(on));
    }
    const doneBtn = root.querySelector('[data-act="done"]');
    doneBtn.textContent = todo.done ? "Mark not done" : "Mark done";
  }

  function renderClassOptions(select) {
    select.innerHTML =
      `<option value="">None</option>` +
      store.classes.map((c) => `<option value="${c.id}">${esc(c.name)}</option>`).join("");
  }

  async function renderDetail() {
    docs?.destroy();
    docs = null;
    const todo = store.todos.find((t) => t.id === selected);
    if (!todo) {
      right.innerHTML = `<div class="empty-pane"><p class="empty-title">Nothing selected</p><p class="dek">Pick a to-do on the left, or press <kbd>n</kbd> to add one.</p></div>`;
      return;
    }
    right.innerHTML = `<article class="detail">
      <header class="detail-head">
        <div class="kicker"></div>
        <textarea class="title-input" rows="1" maxlength="300" aria-label="To-do title"></textarea>
        <div class="meta-row">
          <label class="meta"><span>Due</span><input type="date" class="due-input" aria-label="Due date"></label>
          <label class="meta"><span>Class</span><select class="class-input" aria-label="Class"></select></label>
          <div class="prio-seg" role="radiogroup" aria-label="Priority">
            ${PRIORITIES.map((p) => `<button type="button" role="radio" class="seg ${p.cls}" data-prio="${p.value}">${p.label}</button>`).join("")}
          </div>
        </div>
        <div class="meta-row actions">
          <button type="button" class="btn" data-act="done"></button>
          <button type="button" class="btn ghost danger-text" data-act="delete">Delete</button>
        </div>
      </header>
      <div class="detail-docs"></div>
    </article>`;
    const root = right.querySelector(".detail");
    const titleEl = root.querySelector(".title-input");
    titleEl.value = todo.title;
    renderClassOptions(root.querySelector(".class-input"));
    paintDetail();
    autosize(titleEl);
    requestAnimationFrame(() => autosize(titleEl));
    document.fonts?.ready.then(() => autosize(titleEl));

    titleEl.addEventListener("input", () => autosize(titleEl));
    titleEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        titleEl.blur();
      }
    });
    titleEl.addEventListener("change", () => {
      const value = titleEl.value.trim();
      if (!value) {
        titleEl.value = todo.title;
        return;
      }
      updateTodo(todo.id, { title: value }, repaintAll);
    });
    root.querySelector(".due-input").addEventListener("change", (e) =>
      updateTodo(todo.id, { due_date: e.target.value || null }, repaintAll),
    );
    root.querySelector(".class-input").addEventListener("change", (e) =>
      updateTodo(todo.id, { class_id: e.target.value ? Number(e.target.value) : null }, repaintAll),
    );
    root.querySelector(".prio-seg").addEventListener("click", (e) => {
      const b = e.target.closest("[data-prio]");
      if (b) updateTodo(todo.id, { priority: Number(b.dataset.prio) }, repaintAll);
    });
    root.querySelector('[data-act="done"]').addEventListener("click", () =>
      updateTodo(todo.id, { done: !todo.done }, repaintAll),
    );
    root.querySelector('[data-act="delete"]').addEventListener("click", () => removeTodo(todo.id));

    docs = mountDocs(root.querySelector(".detail-docs"), { kind: "todo", id: todo.id });
  }

  async function removeTodo(id) {
    const todo = store.todos.find((t) => t.id === id);
    if (!todo) return;
    const ok = await confirmDialog(`Delete “${todo.title}” and all its pages?`);
    if (!ok) return;
    try {
      await del(`/api/todos/${id}`);
    } catch (err) {
      toast(err.message, "error");
      return;
    }
    store.todos = store.todos.filter((t) => t.id !== id);
    const after = visible()[0]?.id ?? null;
    select(after);
    toast("To-do deleted");
  }

  function select(id) {
    selected = id;
    if (id) history.replaceState(null, "", `#/t/${id}`);
    else history.replaceState(null, "", "#/todos");
    reflectSelection(listEl, id);
    renderDetail();
    if (id) listEl.querySelector(`.row[data-id="${id}"]`)?.scrollIntoView({ block: "nearest" });
  }

  // ----- events -----

  left.querySelector(".seg-tabs").addEventListener("click", (e) => {
    const b = e.target.closest("[data-filter]");
    if (!b) return;
    filter = b.dataset.filter;
    try {
      localStorage.setItem("bs.filter", filter);
    } catch {
      /* ignore */
    }
    renderList();
  });

  listEl.addEventListener("click", (e) => {
    const row = e.target.closest(".row");
    if (!row) return;
    const id = Number(row.dataset.id);
    if (e.target.closest('[data-act="check"]')) {
      const todo = store.todos.find((t) => t.id === id);
      updateTodo(id, { done: !todo.done }, repaintAll);
      return;
    }
    select(id);
  });

  quickAdd.addEventListener("submit", async (e) => {
    e.preventDefault();
    const input = quickAdd.elements.title;
    const title = input.value.trim();
    if (!title) return;
    const due = quickAdd.elements.due.value || null;
    input.value = "";
    quickAdd.elements.due.value = "";
    try {
      const todo = await post("/api/todos", { title, due_date: due });
      store.todos.push(todo);
      filter = filter === "done" ? "open" : filter;
      renderList();
      select(todo.id);
    } catch (err) {
      toast(err.message, "error");
    }
  });

  // ----- mount -----

  // Open the first to-do when nothing is chosen, so the detail pane is rarely empty.
  if (!selected && visible().length) selected = sortTodos(visible())[0].id;
  main.replaceChildren(splitPane(left, right, "bs.split.todos"));
  renderList();
  renderDetail();

  const onKey = (e) => {
    const key = e.key;
    if (key === "n") {
      quickAdd.elements.title.focus();
      return true;
    }
    if (key === "j" || key === "ArrowDown") {
      moveSelection(listEl, 1, select);
      return true;
    }
    if (key === "k" || key === "ArrowUp") {
      moveSelection(listEl, -1, select);
      return true;
    }
    if (!selected) return false;
    const todo = store.todos.find((t) => t.id === selected);
    if (!todo) return false;
    if (key === "Enter" || key === "e") {
      docs?.focusEditor();
      return true;
    }
    if (key === "x") {
      updateTodo(todo.id, { done: !todo.done }, repaintAll);
      return true;
    }
    if (key === "d") {
      right.querySelector(".due-input")?.focus();
      return true;
    }
    if (key in PRIORITY_KEYS) {
      updateTodo(todo.id, { priority: PRIORITY_KEYS[key] }, repaintAll);
      return true;
    }
    return false;
  };

  return { onKey, teardown: () => docs?.destroy() };
}

function autosize(ta) {
  ta.style.height = "auto";
  ta.style.height = `${ta.scrollHeight}px`;
}

// ----- classes index -----

export function renderClassesIndex(main, ctx) {
  const cards = store.classes.length
    ? store.classes
        .map(
          (c) => `<a class="class-card c-${esc(c.color)}" href="#/c/${c.id}">
            <span class="card-bar" aria-hidden="true"></span>
            <h2 class="card-title">${esc(c.name)}</h2>
            <p class="card-section">${esc(c.section || "No section")}</p>
            <p class="card-counts">${c.open_count} open · ${c.task_count} total</p>
          </a>`,
        )
        .join("")
    : `<p class="empty">No classes yet. Add one and every assignment you tag with it lands on its page.</p>`;

  main.innerHTML = `<section class="index">
    <header class="page-intro">
      <h1 class="heading">Classes</h1>
      <button type="button" class="btn" data-act="new-class">New class</button>
    </header>
    <div class="cards">${cards}</div>
  </section>`;

  main.querySelector('[data-act="new-class"]').addEventListener("click", () => createClass());
  return {
    onKey: (e) => {
      if (e.key !== "n") return false;
      createClass();
      return true;
    },
  };
}

async function createClass() {
  const values = await formDialog({
    title: "New class",
    submitLabel: "Create",
    fields: [
      { name: "name", label: "Name", required: true, maxlength: 120 },
      { name: "section", label: "Section or room", maxlength: 120 },
      { name: "color", label: "Colour", type: "color", options: COLORS, value: "oxblood" },
    ],
  });
  if (!values || !values.name) return;
  try {
    const created = await post("/api/classes", values);
    store.classes.push({ ...created, task_count: 0, open_count: 0 });
    location.hash = `#/c/${created.id}`;
  } catch (err) {
    toast(err.message, "error");
  }
}

// ----- class page -----

export function renderClass(main, classId, ctx) {
  const cls = store.classes.find((c) => c.id === Number(classId));
  if (!cls) {
    main.innerHTML = `<section class="index"><p class="empty">That class could not be found. <a href="#/classes">Back to classes</a>.</p></section>`;
    return {};
  }
  let selected = null;

  const left = fromHtml(`<section class="pane list-pane">
      <header class="pane-head class-head">
        <h1 class="pane-title class-title c-${esc(cls.color)}">${esc(cls.name)}</h1>
        <span class="class-section">${esc(cls.section || "")}</span>
        <span class="class-actions">
          <button type="button" class="btn ghost" data-act="edit">Edit</button>
          <button type="button" class="btn ghost danger-text" data-act="delete">Delete</button>
        </span>
      </header>
      <form class="quick-add" autocomplete="off">
        <input name="title" maxlength="300" placeholder="New assignment…" aria-label="New assignment">
        <input name="due" type="date" title="Due date" aria-label="Due date">
        <button class="btn" type="submit">Add</button>
      </form>
      <div class="list" role="listbox" aria-label="Assignments"></div>
      ${keysRule([
        ["n", "assignment"],
        ["j k", "move"],
        ["⏎", "open to-do"],
        ["x", "done"],
        ["1 2 3 0", "priority"],
      ])}
    </section>`);
  const right = fromHtml(`<section class="pane doc-pane">
      <header class="pane-head"><h2 class="pane-title small">Class document</h2></header>
      <div class="class-docs"></div>
    </section>`);
  const listEl = left.querySelector(".list");
  const quickAdd = left.querySelector(".quick-add");

  const assignments = () => store.todos.filter((t) => t.class_id === cls.id);

  function renderList() {
    refreshList(listEl, assignments(), selected, true);
  }

  function repaint() {
    renderList();
  }

  function select(id) {
    selected = id;
    reflectSelection(listEl, id);
  }

  listEl.addEventListener("click", (e) => {
    const row = e.target.closest(".row");
    if (!row) return;
    const id = Number(row.dataset.id);
    if (e.target.closest('[data-act="check"]')) {
      const todo = store.todos.find((t) => t.id === id);
      updateTodo(id, { done: !todo.done }, repaint);
      return;
    }
    // Assignments open as to-dos in the main list so their documents stay in one place.
    location.hash = `#/t/${id}`;
  });

  quickAdd.addEventListener("submit", async (e) => {
    e.preventDefault();
    const input = quickAdd.elements.title;
    const title = input.value.trim();
    if (!title) return;
    const due = quickAdd.elements.due.value || null;
    input.value = "";
    quickAdd.elements.due.value = "";
    try {
      const todo = await post("/api/todos", { title, class_id: cls.id, due_date: due });
      store.todos.push(todo);
      selected = todo.id;
      renderList();
    } catch (err) {
      toast(err.message, "error");
    }
  });

  left.querySelector('[data-act="edit"]').addEventListener("click", async () => {
    const values = await formDialog({
      title: "Edit class",
      fields: [
        { name: "name", label: "Name", value: cls.name, required: true, maxlength: 120 },
        { name: "section", label: "Section or room", value: cls.section, maxlength: 120 },
        { name: "color", label: "Colour", type: "color", options: COLORS, value: cls.color },
      ],
    });
    if (!values || !values.name) return;
    try {
      Object.assign(cls, await patch(`/api/classes/${cls.id}`, values));
      ctx.rerender?.();
    } catch (err) {
      toast(err.message, "error");
    }
  });

  left.querySelector('[data-act="delete"]').addEventListener("click", async () => {
    const ok = await confirmDialog(`Delete the class “${cls.name}”? Its assignments stay in your to-do list, but its central document is removed.`);
    if (!ok) return;
    try {
      await del(`/api/classes/${cls.id}`);
    } catch (err) {
      toast(err.message, "error");
      return;
    }
    store.classes = store.classes.filter((c) => c.id !== cls.id);
    store.todos = store.todos.map((t) => (t.class_id === cls.id ? { ...t, class_id: null, class_name: null, class_color: null } : t));
    location.hash = "#/classes";
  });

  main.replaceChildren(splitPane(left, right, "bs.split.class"));
  const docs = mountDocs(right.querySelector(".class-docs"), { kind: "class", id: cls.id });
  renderList();

  return {
    teardown: () => docs.destroy(),
    onKey: (e) => {
      const key = e.key;
      if (key === "n") {
        quickAdd.elements.title.focus();
        return true;
      }
      if (key === "j" || key === "ArrowDown") {
        moveSelection(listEl, 1, select);
        return true;
      }
      if (key === "k" || key === "ArrowUp") {
        moveSelection(listEl, -1, select);
        return true;
      }
      if (!selected) return false;
      if (key === "Enter") {
        location.hash = `#/t/${selected}`;
        return true;
      }
      const todo = store.todos.find((t) => t.id === selected);
      if (!todo) return false;
      if (key === "x") {
        updateTodo(todo.id, { done: !todo.done }, repaint);
        return true;
      }
      if (key in PRIORITY_KEYS) {
        updateTodo(todo.id, { priority: PRIORITY_KEYS[key] }, repaint);
        return true;
      }
      return false;
    },
  };
}

// ----- sign in / sign up -----

export function renderAuth(main, mode, ctx) {
  const signup = mode === "signup";
  main.innerHTML = `<section class="auth">
    <div class="auth-card">
      <h1 class="heading">${signup ? "Create account" : "Sign in"}</h1>
      <p class="dek">${signup ? "Username: 3–32 letters, digits or underscores. Password: at least 8 characters." : ""}</p>
      <form class="auth-form" autocomplete="on">
        <label class="field"><span class="field-label">Username</span>
          <input name="username" autocomplete="username" required minlength="3" maxlength="32" pattern="[A-Za-z0-9_]{3,32}" autofocus></label>
        <label class="field"><span class="field-label">Password</span>
          <input name="password" type="password" autocomplete="${signup ? "new-password" : "current-password"}" required minlength="8" maxlength="200"></label>
        <p class="form-error" role="alert"></p>
        <button class="btn wide primary" type="submit">${signup ? "Create account" : "Sign in"}</button>
      </form>
      <p class="auth-switch">${
        signup
          ? `Already have an account? <a href="#/login">Sign in</a>`
          : `New here? <a href="#/signup">Create an account</a>`
      }</p>
    </div>
  </section>`;

  const form = main.querySelector(".auth-form");
  const errorEl = form.querySelector(".form-error");
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    errorEl.textContent = "";
    const body = {
      username: form.elements.username.value.trim(),
      password: form.elements.password.value,
    };
    try {
      store.user = await post(signup ? "/api/signup" : "/api/login", body);
      form.elements.password.value = "";
      ctx.onAuth();
    } catch (err) {
      errorEl.textContent = err.message;
    }
  });
  return {};
}
