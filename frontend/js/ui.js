// Small shared UI helpers: escaping, dates, menus, dialogs, toasts, split panes.

export const $ = (sel, root = document) => root.querySelector(sel);

export function esc(value) {
  return String(value ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

export const PRIORITIES = [
  { value: 0, label: "None", cls: "" },
  { value: 1, label: "Low", cls: "low" },
  { value: 2, label: "Medium", cls: "med" },
  { value: 3, label: "High", cls: "high" },
];

export const COLORS = ["oxblood", "forest", "navy", "ochre", "plum", "slate"];

// ----- dates -----

const pad = (n) => String(n).padStart(2, "0");

export function ymd(d) {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

export const todayISO = () => ymd(new Date());

export function parseISODate(iso) {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d);
}

// Whole days from today to the given date (negative means past).
export function dayDiff(iso) {
  return Math.round((parseISODate(iso) - parseISODate(todayISO())) / 86400000);
}

export function fmtDue(iso) {
  return parseISODate(iso).toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
}

export function fmtLongDate(d = new Date()) {
  return d.toLocaleDateString(undefined, { weekday: "long", year: "numeric", month: "long", day: "numeric" });
}

// ----- toasts -----

export function toast(message, tone = "") {
  let box = $("#toasts");
  if (!box) {
    box = document.createElement("div");
    box.id = "toasts";
    box.setAttribute("role", "status");
    document.body.append(box);
  }
  const item = document.createElement("div");
  item.className = `toast ${tone}`;
  item.textContent = message;
  box.append(item);
  setTimeout(() => item.remove(), 3200);
}

// ----- context menu -----

let menuEl = null;

export function closeMenu() {
  menuEl?.remove();
  menuEl = null;
}

// items: [{label, run, current?, danger?}] or "sep"
export function showMenu(x, y, items) {
  closeMenu();
  menuEl = document.createElement("div");
  menuEl.className = "menu";
  menuEl.setAttribute("role", "menu");
  for (const item of items) {
    if (item === "sep") {
      menuEl.insertAdjacentHTML("beforeend", `<div class="menu-sep"></div>`);
      continue;
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.setAttribute("role", "menuitem");
    btn.className = `menu-item${item.current ? " current" : ""}${item.danger ? " danger" : ""}`;
    btn.textContent = item.label;
    btn.addEventListener("click", () => {
      closeMenu();
      item.run();
    });
    menuEl.append(btn);
  }
  document.body.append(menuEl);
  const { innerWidth: vw, innerHeight: vh } = window;
  const rect = menuEl.getBoundingClientRect();
  menuEl.style.left = `${Math.min(x, vw - rect.width - 8)}px`;
  menuEl.style.top = `${Math.min(y, vh - rect.height - 8)}px`;
  menuEl.querySelector("button")?.focus();
}

document.addEventListener("pointerdown", (e) => {
  if (menuEl && !menuEl.contains(e.target)) closeMenu();
});
window.addEventListener("resize", closeMenu);

// ----- dialogs -----

function sheet(title, bodyHtml) {
  const dlg = document.createElement("dialog");
  dlg.className = "sheet";
  dlg.innerHTML = `<h2 class="sheet-title">${esc(title)}</h2>${bodyHtml}`;
  document.body.append(dlg);
  return dlg;
}

export function confirmDialog(message, okLabel = "Delete") {
  return new Promise((resolve) => {
    const dlg = sheet("Are you sure?", `
      <p class="sheet-text">${esc(message)}</p>
      <div class="sheet-actions">
        <button type="button" class="btn ghost" data-act="cancel">Cancel</button>
        <button type="button" class="btn danger" data-act="ok">${esc(okLabel)}</button>
      </div>`);
    dlg.addEventListener("close", () => {
      const ok = dlg.returnValue === "ok";
      dlg.remove();
      resolve(ok);
    });
    dlg.addEventListener("click", (e) => {
      const act = e.target.dataset?.act;
      if (act) dlg.close(act);
    });
    dlg.showModal();
    dlg.querySelector('[data-act="cancel"]').focus();
  });
}

// fields: [{name, label, value?, type?: 'text'|'date'|'select'|'color', options?, required?}]
// Resolves to an object of values, or null if cancelled.
export function formDialog({ title, submitLabel = "Save", fields }) {
  return new Promise((resolve) => {
    const rows = fields
      .map((f) => {
        let control;
        if (f.type === "select") {
          control = `<select name="${f.name}">${f.options
            .map((o) => `<option value="${esc(o.value)}"${o.value === f.value ? " selected" : ""}>${esc(o.label)}</option>`)
            .join("")}</select>`;
        } else if (f.type === "color") {
          control = `<div class="swatches">${f.options
            .map(
              (c) => `<label class="swatch swatch-${c}"><input type="radio" name="${f.name}" value="${c}"${
                c === f.value ? " checked" : ""
              }><span class="sr">${c}</span></label>`,
            )
            .join("")}</div>`;
        } else {
          control = `<input name="${f.name}" type="${f.type || "text"}" value="${esc(f.value ?? "")}"${
            f.required ? " required" : ""
          } autocomplete="off" maxlength="${f.maxlength || 200}">`;
        }
        return `<label class="field"><span class="field-label">${esc(f.label)}</span>${control}</label>`;
      })
      .join("");
    const dlg = sheet(title, `<form method="dialog" class="sheet-form">${rows}
      <div class="sheet-actions">
        <button type="button" class="btn ghost" data-act="cancel">Cancel</button>
        <button type="submit" class="btn">${esc(submitLabel)}</button>
      </div></form>`);
    dlg.addEventListener("click", (e) => {
      if (e.target.dataset?.act === "cancel") dlg.close("");
    });
    dlg.addEventListener("close", () => {
      const read = (name) => {
        const radio = dlg.querySelector(`[name="${name}"][type="radio"]`);
        if (radio) return dlg.querySelector(`[name="${name}"]:checked`)?.value ?? "";
        return dlg.querySelector(`[name="${name}"]`)?.value.trim() ?? "";
      };
      const result = dlg.returnValue === "ok" ? Object.fromEntries(fields.map((f) => [f.name, read(f.name)])) : null;
      dlg.remove();
      resolve(result);
    });
    dlg.querySelector("form").addEventListener("submit", (e) => {
      e.preventDefault();
      dlg.close("ok");
    });
    dlg.showModal();
    dlg.querySelector("input, select")?.focus();
  });
}

// ----- keyboard help -----

export function showHelp(rows) {
  const dlg = sheet("Keyboard", `<dl class="keys-list">${rows
    .map(([k, label]) => `<dt><kbd>${k}</kbd></dt><dd>${esc(label)}</dd>`)
    .join("")}</dl>
    <div class="sheet-actions"><button type="button" class="btn" data-act="cancel">Close</button></div>`);
  dlg.addEventListener("click", (e) => {
    if (e.target.dataset?.act === "cancel" || e.target === dlg) dlg.close();
  });
  dlg.addEventListener("close", () => dlg.remove());
  dlg.showModal();
}

// ----- split pane with a draggable rule -----

const SPLIT_MIN = 22;
const SPLIT_MAX = 78;

// Two panes separated by a rule the reader can drag (or nudge with arrow keys).
export function splitPane(left, right, storageKey) {
  const wrap = document.createElement("div");
  wrap.className = "split";
  let pct = 40;
  try {
    pct = Number(localStorage.getItem(storageKey)) || pct;
  } catch {
    /* storage unavailable: use the default */
  }
  const setPct = (value) => {
    pct = Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, value));
    wrap.style.setProperty("--split", `${pct}%`);
  };
  setPct(pct);

  const rule = document.createElement("div");
  rule.className = "split-rule";
  rule.setAttribute("role", "separator");
  rule.setAttribute("aria-orientation", "vertical");
  rule.setAttribute("aria-label", "Drag to resize panes");
  rule.tabIndex = 0;
  rule.title = "Drag to resize · arrow keys nudge";
  wrap.append(left, rule, right);

  rule.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    rule.setPointerCapture(e.pointerId);
    rule.classList.add("dragging");
    const box = wrap.getBoundingClientRect();
    const move = (ev) => setPct(((ev.clientX - box.left) / box.width) * 100);
    const up = () => {
      rule.removeEventListener("pointermove", move);
      rule.removeEventListener("pointerup", up);
      rule.classList.remove("dragging");
      try {
        localStorage.setItem(storageKey, String(Math.round(pct)));
      } catch {
        /* ignore */
      }
    };
    rule.addEventListener("pointermove", move);
    rule.addEventListener("pointerup", up);
  });

  rule.addEventListener("keydown", (e) => {
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      setPct(pct + (e.key === "ArrowLeft" ? -2 : 2));
      try {
        localStorage.setItem(storageKey, String(Math.round(pct)));
      } catch {
        /* ignore */
      }
    }
  });
  return wrap;
}

// ----- focus mode (expand to just the text) -----

export function toggleFocus(force) {
  const on = document.body.classList.toggle("focus-mode", force);
  // Also go fullscreen where the browser allows it; both are undone together.
  try {
    if (on && !document.fullscreenElement) document.documentElement.requestFullscreen?.().catch(() => {});
    if (!on && document.fullscreenElement) document.exitFullscreen?.().catch(() => {});
  } catch {
    /* fullscreen unavailable: the expanded layout still applies */
  }
}

// If the browser leaves fullscreen by itself (Esc), leave the expanded view too.
document.addEventListener("fullscreenchange", () => {
  if (!document.fullscreenElement) document.body.classList.remove("focus-mode");
});


// ----- style (theme) -----

export const getTheme = () => (document.documentElement.dataset.theme === "plain" ? "plain" : "dark");

export function setTheme(name) {
  if (name === "plain") document.documentElement.dataset.theme = "plain";
  else delete document.documentElement.dataset.theme;
  try {
    localStorage.setItem("bs.theme", name);
  } catch {
    /* storage unavailable: the choice lasts for this visit only */
  }
}
