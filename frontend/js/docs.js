// The document panel: a tab strip of pages, a title, and the block editor.
// Used for to-do details and for each class's central document.

import { get, post, patch, del } from "./api.js";
import { Editor } from "./editor.js";
import { confirmDialog, esc, toast, toggleFocus } from "./ui.js";

const SAVE_DELAY_MS = 600;
const openPanels = new Set();

// Save anything still waiting in an open document (called before navigating away).
export async function flushAllDocs() {
  await Promise.all([...openPanels].map((p) => p.flush()));
}

// Last-saved page is remembered per viewer so reopening a to-do lands on the same page.
const rememberKey = (owner) => `bs.page.${owner.kind}.${owner.id}`;

function rememberedPage(owner) {
  try {
    return Number(localStorage.getItem(rememberKey(owner))) || null;
  } catch {
    return null;
  }
}

function setRememberedPage(owner, id) {
  try {
    localStorage.setItem(rememberKey(owner), String(id));
  } catch {
    /* storage unavailable */
  }
}

// owner: { kind: "todo" | "class", id }
export function mountDocs(container, owner) {
  const ownerQuery = owner.kind === "todo" ? `todo_id=${owner.id}` : `class_id=${owner.id}`;
  const ownerBody = owner.kind === "todo" ? { todo_id: owner.id } : { class_id: owner.id };

  container.innerHTML = `
    <div class="docs">
      <div class="page-tabs" role="tablist" aria-label="Pages"></div>
      <div class="page-head">
        <input class="page-title" aria-label="Page title" maxlength="120" value="">
        <span class="save-state" aria-live="polite"></span>
        <button type="button" class="btn ghost" data-act="new-page">New page</button>
        <button type="button" class="btn ghost" data-act="del-page">Delete page</button>
        <button type="button" class="btn ghost" data-act="fullscreen" title="Fullscreen notes (Ctrl+Shift+F)">Fullscreen</button>
      </div>
      <div class="doc-scroll"><div class="doc-host"></div><nav class="outline" aria-label="Headings" hidden></nav></div>
    </div>`;

  const tabs = container.querySelector(".page-tabs");
  const titleInput = container.querySelector(".page-title");
  const stateEl = container.querySelector(".save-state");
  const outlineEl = container.querySelector(".outline");
  let host = container.querySelector(".doc-host");

  let pages = [];
  let active = null;
  let editor = null;
  let dirty = false;
  let timer = null;
  let chain = Promise.resolve();
  let seq = 0;
  let destroyed = false;

  const setState = (text) => {
    stateEl.textContent = text;
  };

  // Saves run one after another so an older request can never overwrite a newer one.
  function save(id, body) {
    chain = chain.then(async () => {
      setState("Saving…");
      try {
        await patch(`/api/pages/${id}`, { body });
        if (active?.id === id) setState("Saved");
      } catch (err) {
        dirty = true;
        setState("Not saved");
        toast(err.message, "error");
      }
    });
    return chain;
  }

  async function flush() {
    clearTimeout(timer);
    timer = null;
    if (!dirty || !editor || !active) return chain;
    dirty = false;
    return save(active.id, editor.getMarkdown());
  }

  // Tiny table of contents: headings nested by level, click to jump.
  let outlineItems = [];
  function renderOutline() {
    outlineItems = editor ? editor.headings() : [];
    outlineEl.hidden = outlineItems.length === 0;
    outlineEl.replaceChildren();
    const stack = []; // levels of the open ancestors
    outlineItems.forEach((h, i) => {
      while (stack.length && stack[stack.length - 1] >= h.level) stack.pop();
      const btn = document.createElement("button");
      btn.type = "button";
      btn.dataset.i = i;
      btn.textContent = h.text || "Untitled";
      btn.title = btn.textContent;
      btn.style.paddingLeft = `${stack.length * 9}px`;
      outlineEl.append(btn);
      stack.push(h.level);
    });
  }

  outlineEl.addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-i]");
    if (btn) outlineItems[Number(btn.dataset.i)]?.blk.scrollIntoView({ block: "start", behavior: "smooth" });
  });

  let outlineTimer = null;
  function queueSave() {
    clearTimeout(outlineTimer);
    outlineTimer = setTimeout(renderOutline, 250);
    dirty = true;
    setState("Editing…");
    clearTimeout(timer);
    timer = setTimeout(flush, SAVE_DELAY_MS);
  }

  function renderTabs() {
    tabs.innerHTML = pages
      .map(
        (p) => `<button type="button" role="tab" class="page-tab${active?.id === p.id ? " on" : ""}" data-id="${p.id}" aria-selected="${
          active?.id === p.id
        }">${esc(p.title || "Untitled")}</button>`,
      )
      .join("");
  }

  async function select(id) {
    const ticket = ++seq;
    await flush();
    if (destroyed || ticket !== seq) return;
    const page = await get(`/api/pages/${id}`);
    if (destroyed || ticket !== seq) return;
    active = page;
    setRememberedPage(owner, id);
    renderTabs();
    titleInput.value = page.title;

    // A fresh host element per page keeps the old editor's listeners from piling up.
    const next = document.createElement("div");
    next.className = "doc-host";
    host.replaceWith(next);
    host = next;
    editor = new Editor(host, { onChange: queueSave });
    editor.setMarkdown(page.body);
    renderOutline();
    setState("Saved");
  }

  async function load() {
    pages = await get(`/api/pages?${ownerQuery}`);
    if (!pages.length) {
      pages = [await post("/api/pages", { ...ownerBody, title: "Notes" })];
    }
    const remembered = rememberedPage(owner);
    const start = pages.find((p) => p.id === remembered) || pages[0];
    await select(start.id);
  }

  // ----- events -----

  tabs.addEventListener("click", (e) => {
    const tab = e.target.closest(".page-tab");
    if (tab) select(Number(tab.dataset.id)).catch((err) => toast(err.message, "error"));
  });

  titleInput.addEventListener("change", async () => {
    if (!active) return;
    const title = titleInput.value.trim() || "Untitled";
    titleInput.value = title;
    try {
      const updated = await patch(`/api/pages/${active.id}`, { title });
      active.title = updated.title;
      pages = pages.map((p) => (p.id === updated.id ? { ...p, title: updated.title } : p));
      renderTabs();
    } catch (err) {
      toast(err.message, "error");
    }
  });
  titleInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      titleInput.blur();
      editor?.focusStart();
    }
  });

  container.querySelector('[data-act="new-page"]').addEventListener("click", async () => {
    await flush();
    const page = await post("/api/pages", { ...ownerBody, title: "Untitled" });
    pages = [...pages, page];
    await select(page.id);
    titleInput.focus();
    titleInput.select();
  });

  container.querySelector('[data-act="fullscreen"]').addEventListener("click", () => {
    toggleFocus();
    editor?.focusStart();
  });

  container.querySelector('[data-act="del-page"]').addEventListener("click", async () => {
    if (!active) return;
    const ok = await confirmDialog(`Delete “${active.title}”? This cannot be undone.`);
    if (!ok) return;
    const doomed = active.id;
    dirty = false;
    clearTimeout(timer);
    await del(`/api/pages/${doomed}`);
    active = null;
    pages = pages.filter((p) => p.id !== doomed);
    if (!pages.length) pages = [await post("/api/pages", { ...ownerBody, title: "Notes" })];
    await select(pages[0].id);
  });

  const panel = {
    flush,
    focusEditor: () => editor?.focusStart(),
    destroy() {
      destroyed = true;
      flush();
      openPanels.delete(panel);
    },
  };
  openPanels.add(panel);

  load().catch((err) => toast(err.message, "error"));
  return panel;
}
