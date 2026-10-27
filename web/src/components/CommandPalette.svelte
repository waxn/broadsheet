<script>
  // Ctrl+K: jump anywhere, run a command, or search tasks, pages and events.
  import { Search, Inbox, CalendarDays, CalendarRange, Calendar, NotebookPen, Flame, Plus, FilePlus, CalendarPlus, PanelLeft, Maximize2, Keyboard, LogOut, Hash, FileText, CircleCheck, Trash2 } from "@lucide/svelte";
  import Modal from "./Modal.svelte";
  import { app, createPage, localSet, signOut } from "../lib/store.svelte.js";
  import { go } from "../lib/router.svelte.js";
  import { get, qs } from "../lib/api.js";
  import { hex } from "../lib/colors.js";
  import * as D from "../lib/dates.js";

  let q = $state("");
  let active = $state(0);
  let remote = $state({ todos: [], pages: [], events: [] });
  let timer;

  const close = () => (app.palette = false);
  const run = (fn) => () => {
    close();
    fn();
  };

  const commands = [
    { label: "Go to Inbox", icon: Inbox, hint: "G I", run: () => go("/inbox") },
    { label: "Go to Today", icon: CalendarDays, hint: "G T", run: () => go("/today") },
    { label: "Go to Upcoming", icon: CalendarRange, hint: "G U", run: () => go("/upcoming") },
    { label: "Go to Calendar", icon: Calendar, hint: "G C", run: () => go("/calendar") },
    { label: "Go to Notes", icon: NotebookPen, hint: "G N", run: () => go("/notes") },
    { label: "Go to Habits", icon: Flame, hint: "G H", run: () => go("/habits") },
    { label: "Completed tasks", icon: CircleCheck, run: () => go("/completed") },
    { label: "Trash", icon: Trash2, run: () => go("/trash") },
    { label: "Add task", icon: Plus, hint: "Q", run: () => (app.quickAdd = { defaults: {} }) },
    { label: "New page", icon: FilePlus, run: async () => { const p = await createPage({}); if (p) go(`/page/${p.id}`); } },
    { label: "New event", icon: CalendarPlus, run: () => (app.eventEditor = { event: { title: "", all_day: 0, start_at: D.join(D.today(), "09:00"), end_at: D.join(D.today(), "10:00") } }) },
    { label: "Toggle sidebar", icon: PanelLeft, hint: "[", run: () => { app.sidebar = !app.sidebar; localSet("sidebar", app.sidebar); } },
    { label: "Focus mode", icon: Maximize2, hint: "Ctrl ⇧ F", run: () => (app.focus = !app.focus) },
    { label: "Keyboard shortcuts", icon: Keyboard, hint: "?", run: () => (app.help = true) },
    { label: "Log out", icon: LogOut, run: signOut },
  ];

  const norm = (s) => (s || "").toLowerCase();
  const fuzzy = (text, query) => {
    const t = norm(text);
    let i = 0;
    for (const ch of norm(query)) {
      i = t.indexOf(ch, i);
      if (i < 0) return false;
      i++;
    }
    return true;
  };

  const results = $derived.by(() => {
    const query = q.trim();
    const out = [];
    const cmd = query ? commands.filter((c) => fuzzy(c.label, query)) : commands.slice(0, 6);
    if (cmd.length) out.push({ heading: query ? "Commands" : "Jump to" }, ...cmd.map((c) => ({ ...c, run: run(c.run) })));
    if (!query) {
      const recent = [...app.pages].sort((a, b) => (b.updated_at || "").localeCompare(a.updated_at || "")).slice(0, 5);
      if (recent.length) out.push({ heading: "Recent pages" }, ...recent.map((p) => ({ label: p.title || "Untitled", emoji: p.icon, icon: FileText, run: run(() => go(`/page/${p.id}`)) })));
      return out;
    }
    const classes = app.classes.filter((c) => norm(c.name).includes(norm(query))).slice(0, 5);
    if (classes.length) out.push({ heading: "Classes" }, ...classes.map((c) => ({ label: c.name, icon: Hash, color: hex(c.color), run: run(() => go(`/class/${c.id}`)) })));
    const localPages = app.pages.filter((p) => norm(p.title).includes(norm(query)));
    const pageIds = new Set(localPages.map((p) => p.id));
    const pages = [...localPages.map((p) => ({ id: p.id, title: p.title, icon: p.icon })), ...remote.pages.filter((p) => !pageIds.has(p.id))].slice(0, 8);
    if (pages.length) out.push({ heading: "Pages" }, ...pages.map((p) => ({ label: p.title || "Untitled", sub: p.excerpt, emoji: p.icon, icon: FileText, run: run(() => go(`/page/${p.id}`)) })));
    const localTasks = app.todos.filter((t) => norm(t.title).includes(norm(query)));
    const taskIds = new Set(localTasks.map((t) => t.id));
    const tasks = [...localTasks, ...remote.todos.filter((t) => !taskIds.has(t.id))].slice(0, 8);
    if (tasks.length)
      out.push({ heading: "Tasks" }, ...tasks.map((t) => ({ label: t.title, sub: [t.class_name || "Inbox", t.due_date && D.label(t.due_date)].filter(Boolean).join(" · "), icon: CircleCheck, done: t.done, run: run(() => { if (!app.todos.some((x) => x.id === t.id)) app.todos.push(t); app.detail = t.id; }) })));
    if (remote.events.length)
      out.push({ heading: "Events" }, ...remote.events.map((e) => ({ label: e.title, sub: D.long(e.start_at.slice(0, 10)), icon: Calendar, run: run(() => go(`/calendar?date=${e.start_at.slice(0, 10)}`)) })));
    return out;
  });

  const items = $derived(results.filter((r) => !r.heading));

  $effect(() => {
    const query = q.trim();
    active = 0;
    clearTimeout(timer);
    if (!query) {
      remote = { todos: [], pages: [], events: [] };
      return;
    }
    timer = setTimeout(async () => {
      try {
        const r = await get(`/api/search?${qs({ q: query })}`);
        if (q.trim() === query) remote = r;
      } catch {
        /* offline: local results still show */
      }
    }, 140);
  });

  function key(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(items.length - 1, active + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(0, active - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      items[active]?.run();
    }
  }

  $effect(() => {
    active;
    document.querySelector(".pal .menu-item.active")?.scrollIntoView({ block: "nearest" });
  });
</script>

<Modal onclose={close} width={640} top="12vh" label="Search">
  <div class="pal">
    <div class="bar">
      <Search size={18} />
      <!-- svelte-ignore a11y_autofocus -->
      <input placeholder="Search or type a command…" bind:value={q} onkeydown={key} autofocus />
      <kbd>Esc</kbd>
    </div>
    <div class="list">
      {#each results as r}
        {#if r.heading}
          <div class="menu-label">{r.heading}</div>
        {:else}
          {@const idx = items.indexOf(r)}
          <button type="button" class="menu-item" class:active={idx === active} onmouseenter={() => (active = idx)} onclick={r.run}>
            {#if r.emoji}<span class="emoji">{r.emoji}</span>{:else}<r.icon size={16} strokeWidth={1.75} color={r.color} />{/if}
            <span class="text"><span class:done={r.done}>{r.label}</span>{#if r.sub}<span class="sub">{r.sub}</span>{/if}</span>
            {#if r.hint}<span class="hint">{r.hint}</span>{/if}
          </button>
        {/if}
      {:else}
        <p class="none">No results for “{q}”.</p>
      {/each}
    </div>
  </div>
</Modal>

<style>
  .bar { display: flex; align-items: center; gap: 10px; padding: 12px 16px; border-bottom: 1px solid var(--border); color: var(--muted); }
  input { flex: 1; border: 0; outline: none; background: none; font-size: 16px; color: var(--text); }
  .list { max-height: 56vh; overflow: auto; padding: 6px; }
  .emoji { width: 16px; text-align: center; }
  .text { display: flex; flex-direction: column; min-width: 0; }
  .text span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { font-size: 12px; color: var(--muted); }
  .done { text-decoration: line-through; color: var(--muted); }
  .none { padding: 20px; text-align: center; color: var(--muted); }
</style>
