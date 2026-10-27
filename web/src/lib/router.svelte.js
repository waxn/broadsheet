// History-API routing. Paths: /today /inbox /upcoming /calendar /notes /habits /trash
// /class/:id /page/:id. The server answers any of them with the app.

const ROUTES = [
  [/^\/(today)?$/, "today"],
  [/^\/inbox$/, "inbox"],
  [/^\/upcoming$/, "upcoming"],
  [/^\/calendar$/, "calendar"],
  [/^\/notes$/, "notes"],
  [/^\/habits$/, "habits"],
  [/^\/completed$/, "completed"],
  [/^\/trash$/, "trash"],
  [/^\/class\/(\d+)$/, "class"],
  [/^\/page\/(\d+)$/, "page"],
];

function resolve(path) {
  for (const [re, name] of ROUTES) {
    const m = re.exec(path);
    if (m) return { name, id: name === "class" || name === "page" ? Number(m[1]) : null };
  }
  return { name: "today", id: null };
}

export const route = $state({ path: location.pathname, ...resolve(location.pathname), query: new URLSearchParams(location.search) });

function update() {
  const r = resolve(location.pathname);
  route.path = location.pathname;
  route.name = r.name;
  route.id = r.id;
  route.query = new URLSearchParams(location.search);
}

export function go(path, { replace = false } = {}) {
  if (path === location.pathname + location.search) return;
  history[replace ? "replaceState" : "pushState"](null, "", path);
  update();
  document.querySelector("main")?.scrollTo?.(0, 0);
}

window.addEventListener("popstate", update);

// Same-origin links navigate in place.
document.addEventListener("click", (e) => {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
  const a = e.target.closest?.("a[href]");
  if (!a || a.target || a.hasAttribute("download")) return;
  const url = new URL(a.href, location.href);
  if (url.origin !== location.origin || url.pathname.startsWith("/api/")) return;
  e.preventDefault();
  go(url.pathname + url.search);
});
