// App shell: boot, hash router, masthead, and global keyboard shortcuts.

import { get, post, store } from "./api.js";
import { flushAllDocs } from "./docs.js";
import { $, closeMenu, esc, fmtDue, getTheme, setTheme, showHelp, toast, toggleFocus } from "./ui.js";
import { renderAuth, renderClass, renderClassesIndex, renderTodos } from "./views.js";

const main = $("#main");
const masthead = $("#masthead");

let current = null; // { onKey, teardown } for the screen on show
let routeSeq = 0;

const HELP = [
  ["n", "New to-do, or new assignment on a class page"],
  ["j / k", "Move up and down the list (opens each to-do)"],
  ["Enter", "Edit the open document"],
  ["x", "Mark the selected to-do done or not done"],
  ["1 2 3 0", "Priority: high, medium, low, none"],
  ["d", "Set the due date"],
  ["/", "Search tasks, classes and notes"],
  ["g t", "Go to to-dos"],
  ["g c", "Go to classes"],
  ["Ctrl+Shift+F", "Fullscreen notes (Esc to leave)"],
  ["Esc", "Leave fullscreen, close menus"],
  ["Ctrl+B / I / E / K", "Bold, italic, code, link (in a document)"],
  ["Right-click a block", "Change its type or delete it"],
  ["Drag the rule", "Resize the two panes"],
  ["?", "This list"],
];

function parseRoute() {
  const parts = location.hash.replace(/^#\/?/, "").split("/");
  const [head, id] = parts;
  if (head === "login" || head === "signup") return { name: head };
  if (head === "t") return { name: "todos", id };
  if (head === "classes") return { name: "classes" };
  if (head === "c") return { name: "class", id };
  return { name: "todos" };
}

function renderMasthead(active) {
  const user = store.user;
  if (!user) {
    masthead.innerHTML = "";
    return;
  }
  masthead.innerHTML = `
    <nav class="nav" aria-label="Main">
      <a href="#/todos" class="${active === "todos" ? "on" : ""}">Tasks</a>
      <a href="#/classes" class="${active === "classes" || active === "class" ? "on" : ""}">Classes</a>
    </nav>
    <div class="search-wrap">
      <input id="global-search" type="search" placeholder="Search tasks, classes and notes" autocomplete="off" aria-label="Search" aria-controls="search-results">
      <kbd class="search-key">/</kbd>
      <div id="search-results" class="results" role="listbox" hidden></div>
    </div>
    <div class="mast-user">
      <span class="who">${esc(user.username)}</span>
      <button type="button" class="btn ghost" data-act="theme" title="Switch between the dark and plain styles"></button>
      <button type="button" class="btn ghost" data-act="help" title="Keyboard shortcuts (?)">Keys</button>
      <button type="button" class="btn ghost" data-act="signout">Sign out</button>
    </div>`;

  const themeBtn = masthead.querySelector('[data-act="theme"]');
  const paintThemeBtn = () => (themeBtn.textContent = getTheme() === "plain" ? "Dark style" : "Plain style");
  paintThemeBtn();
  themeBtn.addEventListener("click", () => {
    setTheme(getTheme() === "plain" ? "dark" : "plain");
    paintThemeBtn();
  });
  masthead.querySelector('[data-act="help"]').addEventListener("click", () => showHelp(HELP));
  masthead.querySelector('[data-act="signout"]').addEventListener("click", signOut);
  bindSearch();
}

// ----- global search -----

function go(hash) {
  if (location.hash === hash) route();
  else location.hash = hash;
}

function bindSearch() {
  const input = $("#global-search");
  const box = $("#search-results");
  let items = [];
  let active = -1;
  let timer = 0;
  let ticket = 0;

  const close = () => {
    box.hidden = true;
    active = -1;
  };

  const open = (item) => {
    close();
    input.value = "";
    input.blur();
    if (item.kind === "page") {
      // Land on the matching page when the document opens.
      try {
        localStorage.setItem(`bs.page.${item.todo_id ? "todo" : "class"}.${item.todo_id || item.class_id}`, String(item.id));
      } catch {
        /* storage unavailable */
      }
    }
    go(item.href);
  };

  const paint = () => {
    box.querySelectorAll(".result").forEach((el, i) => el.classList.toggle("on", i === active));
  };

  async function run() {
    const q = input.value.trim();
    const mine = ++ticket;
    if (!q) return close();
    let data;
    try {
      data = await get(`/api/search?q=${encodeURIComponent(q)}`);
    } catch {
      return;
    }
    if (mine !== ticket) return;
    items = [
      ...data.todos.map((t) => ({ kind: "task", href: `#/t/${t.id}`, title: t.title, meta: [t.class_name, t.due_date && fmtDue(t.due_date), t.done && "done"].filter(Boolean).join(" · ") })),
      ...data.classes.map((c) => ({ kind: "class", href: `#/c/${c.id}`, title: c.name, meta: c.section || "" })),
      ...data.pages.map((p) => ({ kind: "page", id: p.id, todo_id: p.todo_id, class_id: p.class_id, href: p.todo_id ? `#/t/${p.todo_id}` : `#/c/${p.class_id}`, title: p.title, meta: p.owner_name || "" })),
    ];
    const labels = { task: "Task", class: "Class", page: "Note" };
    box.innerHTML = items.length
      ? items.map((it, i) => `<div class="result" role="option" data-i="${i}"><span class="result-kind">${labels[it.kind]}</span><span class="result-title">${esc(it.title)}</span><span class="result-meta">${esc(it.meta)}</span></div>`).join("")
      : `<div class="result none">No matches</div>`;
    active = items.length ? 0 : -1;
    box.hidden = false;
    paint();
  }

  input.addEventListener("input", () => {
    clearTimeout(timer);
    timer = setTimeout(run, 120);
  });
  input.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (!items.length) return;
      e.preventDefault();
      active = (active + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
      paint();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (items[active]) open(items[active]);
    } else if (e.key === "Escape") {
      input.value = "";
      close();
      input.blur();
    }
  });
  box.addEventListener("pointerdown", (e) => {
    const el = e.target.closest(".result[data-i]");
    if (el) {
      e.preventDefault();
      open(items[Number(el.dataset.i)]);
    }
  });
  input.addEventListener("blur", () => setTimeout(close, 100));
  input.addEventListener("focus", () => {
    if (input.value.trim() && items.length) box.hidden = false;
  });
}

async function signOut() {
  await flushAllDocs();
  try {
    await post("/api/logout");
  } catch {
    /* the session may already be gone */
  }
  store.user = null;
  store.todos = [];
  store.classes = [];
  location.hash = "#/login";
}

async function loadData() {
  const [todos, classes] = await Promise.all([get("/api/todos"), get("/api/classes")]);
  store.todos = todos;
  store.classes = classes;
}

async function route() {
  const seq = ++routeSeq;
  await flushAllDocs();
  if (seq !== routeSeq) return;

  current?.teardown?.();
  current = null;

  const r = parseRoute();
  const isAuthScreen = r.name === "login" || r.name === "signup";
  if (!store.user && !isAuthScreen) {
    location.hash = "#/login";
    return;
  }
  if (store.user && isAuthScreen) {
    location.hash = "#/todos";
    return;
  }

  renderMasthead(r.name);
  document.body.classList.toggle("on-auth", isAuthScreen);

  if (isAuthScreen) {
    current = renderAuth(main, r.name, { onAuth: onAuthed });
    return;
  }

  try {
    await loadData();
  } catch (err) {
    toast(err.message, "error");
    return;
  }
  if (seq !== routeSeq) return;

  const ctx = { rerender: () => route() };
  switch (r.name) {
    case "todos":
      current = renderTodos(main, r.id);
      break;
    case "classes":
      current = renderClassesIndex(main, ctx);
      break;
    case "class":
      current = renderClass(main, r.id, ctx);
      break;
    default:
      location.hash = "#/todos";
  }
}

function onAuthed() {
  location.hash = "#/todos";
  route();
}

// ----- global keys -----

const isTyping = (t) => t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName);
let chordPending = false;
let chordTimer = 0;

document.addEventListener("keydown", (e) => {
  const isFocusToggle = (e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "f";
  if (isFocusToggle) {
    e.preventDefault();
    toggleFocus();
    return;
  }
  if (e.key === "Escape") {
    closeMenu();
    if (document.body.classList.contains("focus-mode")) {
      e.preventDefault();
      toggleFocus(false);
    }
    return;
  }
  if (isTyping(e.target) || e.metaKey || e.ctrlKey || e.altKey || !store.user) return;
  if (document.querySelector("dialog[open]")) return;

  if (chordPending) {
    chordPending = false;
    clearTimeout(chordTimer);
    const dest = { t: "#/todos", c: "#/classes" }[e.key];
    if (dest) {
      e.preventDefault();
      location.hash = dest;
    }
    return;
  }
  if (e.key === "g") {
    chordPending = true;
    chordTimer = setTimeout(() => (chordPending = false), 1200);
    return;
  }
  if (e.key === "?") {
    e.preventDefault();
    showHelp(HELP);
    return;
  }
  if (e.key === "/") {
    e.preventDefault();
    $("#global-search")?.focus();
    return;
  }
  if (current?.onKey?.(e)) e.preventDefault();
});

// ----- boot -----

async function boot() {
  try {
    store.user = await get("/api/me");
  } catch {
    store.user = null;
  }
  window.addEventListener("hashchange", route);
  if (!location.hash) {
    location.hash = store.user ? "#/todos" : "#/login";
    return;
  }
  route();
}

window.addEventListener("pagehide", () => flushAllDocs());
$("#focus-exit").addEventListener("click", () => toggleFocus(false));
boot();
