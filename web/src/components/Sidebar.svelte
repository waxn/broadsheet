<script>
  import {
    Inbox, CalendarDays, CalendarRange, Calendar, NotebookPen, Flame, Search, CirclePlus, PanelLeft,
    ChevronDown, Plus, Ellipsis, Hash, LogOut, Keyboard, Trash2, Pencil, Archive, CircleCheck,
  } from "@lucide/svelte";
  import PageTreeItem from "./PageTreeItem.svelte";
  import ClassDialog from "./ClassDialog.svelte";
  import { app, createPage, deleteClass, localSet, pageChildren, reorder, signOut, updateClass, updateTodo } from "../lib/store.svelte.js";
  import { go, route } from "../lib/router.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { hex } from "../lib/colors.js";
  import { drag, endDrag, half, moved, startDrag } from "../lib/drag.svelte.js";
  import * as D from "../lib/dates.js";

  let classDialog = $state(null); // null | {} (new) | class
  let openSections = $state({ favorites: true, classes: true, pages: true, archived: false });

  const today = $derived(D.today());
  const open = $derived(app.todos.filter((t) => !t.done));
  const inboxCount = $derived(open.filter((t) => t.class_id == null).length);
  const todayCount = $derived(open.filter((t) => t.due_date && t.due_date <= today).length);
  const classes = $derived(app.classes.filter((c) => !c.archived));
  const archived = $derived(app.classes.filter((c) => c.archived));
  const favorites = $derived(app.pages.filter((p) => p.favorite));
  const rootPages = $derived(pageChildren(null));
  const dayNum = $derived(Number(today.slice(8)));

  let overClass = $state(null);
  let overNav = $state(null);

  function toggleSidebar() {
    app.sidebar = !app.sidebar;
    localSet("sidebar", app.sidebar);
  }

  function userMenu(e) {
    openMenu(e, [
      { heading: app.user.username },
      { label: "Keyboard shortcuts", icon: Keyboard, hint: "?", action: () => (app.help = true) },
      { label: "Completed tasks", icon: CircleCheck, action: () => go("/completed") },
      { label: "Trash", icon: Trash2, action: () => go("/trash") },
      { sep: true },
      { label: "Log out", icon: LogOut, action: signOut },
    ]);
  }

  function classMenu(e, c) {
    openMenu(e, [
      { label: "Edit", icon: Pencil, action: () => (classDialog = c) },
      { label: c.archived ? "Unarchive" : "Archive", icon: Archive, action: () => updateClass(c.id, { archived: !c.archived }) },
      { sep: true },
      {
        label: "Delete",
        icon: Trash2,
        danger: true,
        action: () => {
          if (confirm(`Delete “${c.name}”? Its tasks move to the Inbox; its notes are deleted.`)) {
            deleteClass(c.id);
            if (route.name === "class" && route.id === c.id) go("/inbox");
          }
        },
      },
    ]);
  }

  async function newPage() {
    const p = await createPage({});
    if (p) go(`/page/${p.id}`);
  }

  // Classes reorder by dragging; tasks dropped on a class (or Inbox/Today) move there.
  function classOver(e, c) {
    if (drag.kind === "class" && drag.id !== c.id) {
      e.preventDefault();
      overClass = { id: c.id, where: half(e) };
    } else if (drag.kind === "task") {
      e.preventDefault();
      overClass = { id: c.id, where: "inside" };
    }
  }

  function classDrop(e, c) {
    e.preventDefault();
    const where = overClass?.where;
    overClass = null;
    if (drag.kind === "class") reorder("classes", moved(classes, drag.id, c.id, where));
    else if (drag.kind === "task") updateTodo(drag.id, { class_id: c.id });
    endDrag();
  }

  function navOver(e, name) {
    if (drag.kind !== "task") return;
    e.preventDefault();
    overNav = name;
  }

  function navDrop(e, name) {
    e.preventDefault();
    overNav = null;
    if (drag.kind === "task") {
      if (name === "inbox") updateTodo(drag.id, { class_id: null });
      if (name === "today") updateTodo(drag.id, { due_date: today });
      if (name === "upcoming") updateTodo(drag.id, { due_date: D.addDays(today, 1) });
    }
    endDrag();
  }

  const nav = [
    { name: "inbox", href: "/inbox", label: "Inbox", icon: Inbox },
    { name: "today", href: "/today", label: "Today", icon: CalendarDays },
    { name: "upcoming", href: "/upcoming", label: "Upcoming", icon: CalendarRange },
    { name: "calendar", href: "/calendar", label: "Calendar", icon: Calendar },
    { name: "notes", href: "/notes", label: "Notes", icon: NotebookPen },
    { name: "habits", href: "/habits", label: "Habits", icon: Flame },
  ];
</script>

<aside class="sidebar">
  <div class="top">
    <button type="button" class="user" onclick={userMenu}>
      <span class="avatar">{app.user.username[0].toUpperCase()}</span>
      <span class="uname">{app.user.username}</span>
      <ChevronDown size={14} />
    </button>
    <button type="button" class="icon-btn" aria-label="Hide sidebar ([)" title="Hide sidebar  [" onclick={toggleSidebar}><PanelLeft size={18} strokeWidth={1.75} /></button>
  </div>

  <button type="button" class="add" onclick={() => (app.quickAdd = { defaults: {} })}>
    <CirclePlus size={22} fill="var(--accent)" color="var(--sidebar)" strokeWidth={2} />
    <span>Add task</span>
    <kbd>Q</kbd>
  </button>
  <button type="button" class="nav" onclick={() => (app.palette = true)}>
    <Search size={18} strokeWidth={1.75} /><span>Search</span><kbd class="hint">Ctrl K</kbd>
  </button>

  {#each nav as n}
    <a
      href={n.href}
      class="nav"
      class:current={route.name === n.name}
      class:drop={overNav === n.name}
      ondragover={(e) => navOver(e, n.name)}
      ondragleave={() => (overNav = null)}
      ondrop={(e) => navDrop(e, n.name)}
    >
      {#if n.name === "today"}
        <span class="todayicon"><CalendarDays size={18} strokeWidth={1.75} /><b>{dayNum}</b></span>
      {:else}
        <n.icon size={18} strokeWidth={1.75} />
      {/if}
      <span>{n.label}</span>
      {#if n.name === "inbox" && inboxCount}<span class="count">{inboxCount}</span>{/if}
      {#if n.name === "today" && todayCount}<span class="count">{todayCount}</span>{/if}
    </a>
  {/each}

  <div class="scroll">
    {#if favorites.length}
      <div class="group">
        <button type="button" class="ghead" onclick={() => (openSections.favorites = !openSections.favorites)}>Favorites <ChevronDown size={14} class={openSections.favorites ? "" : "closed"} /></button>
        {#if openSections.favorites}
          {#each favorites as p (p.id)}
            <a href="/page/{p.id}" class="nav sub" class:current={route.name === "page" && route.id === p.id}>
              <span class="emoji">{p.icon || "📄"}</span><span class="ellipsis">{p.title || "Untitled"}</span>
            </a>
          {/each}
        {/if}
      </div>
    {/if}

    <div class="group">
      <div class="ghead">
        <button type="button" class="glabel" onclick={() => (openSections.classes = !openSections.classes)}>My classes <ChevronDown size={14} class={openSections.classes ? "" : "closed"} /></button>
        <button type="button" class="icon-btn mini" aria-label="Add class" onclick={() => (classDialog = {})}><Plus size={16} /></button>
      </div>
      {#if openSections.classes}
        {#each classes as c (c.id)}
          <a
            href="/class/{c.id}"
            class="nav sub"
            class:current={route.name === "class" && route.id === c.id}
            class:over-before={overClass?.id === c.id && overClass.where === "before"}
            class:over-after={overClass?.id === c.id && overClass.where === "after"}
            class:drop={overClass?.id === c.id && overClass.where === "inside"}
            draggable="true"
            ondragstart={(e) => startDrag(e, "class", c.id)}
            ondragend={endDrag}
            ondragover={(e) => classOver(e, c)}
            ondragleave={() => (overClass = null)}
            ondrop={(e) => classDrop(e, c)}
            oncontextmenu={(e) => classMenu(e, c)}
          >
            {#if c.emoji}<span class="emoji">{c.emoji}</span>{:else}<Hash size={16} color={hex(c.color)} strokeWidth={2} />{/if}
            <span class="ellipsis">{c.name}</span>
            {#if c.open_count}<span class="count">{c.open_count}</span>{/if}
            <button type="button" class="icon-btn mini more" aria-label="Class actions" onclick={(e) => { e.preventDefault(); classMenu(e, c); }}><Ellipsis size={15} /></button>
          </a>
        {:else}
          <button type="button" class="empty" onclick={() => (classDialog = {})}>Add your first class</button>
        {/each}
        {#if archived.length}
          <button type="button" class="nav sub faint" onclick={() => (openSections.archived = !openSections.archived)}>
            <Archive size={15} strokeWidth={1.75} /><span>Archived ({archived.length})</span>
          </button>
          {#if openSections.archived}
            {#each archived as c (c.id)}
              <a href="/class/{c.id}" class="nav sub faint" oncontextmenu={(e) => classMenu(e, c)}><Hash size={15} color={hex(c.color)} /><span class="ellipsis">{c.name}</span></a>
            {/each}
          {/if}
        {/if}
      {/if}
    </div>

    <div class="group">
      <div class="ghead">
        <button type="button" class="glabel" onclick={() => (openSections.pages = !openSections.pages)}>Pages <ChevronDown size={14} class={openSections.pages ? "" : "closed"} /></button>
        <button type="button" class="icon-btn mini" aria-label="New page" onclick={newPage}><Plus size={16} /></button>
      </div>
      {#if openSections.pages}
        <div role="tree">
          {#each rootPages as p (p.id)}
            <PageTreeItem page={p} />
          {:else}
            <button type="button" class="empty" onclick={newPage}>Start a page</button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</aside>

{#if classDialog}
  <ClassDialog cls={classDialog.id ? classDialog : null} onclose={() => (classDialog = null)} />
{/if}

<style>
  .sidebar {
    width: var(--sidebar-w); flex: none; height: 100%; background: var(--sidebar);
    display: flex; flex-direction: column; padding: 10px 10px 0; gap: 1px; border-right: 1px solid #2a2926;
  }
  .top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
  .user { display: flex; align-items: center; gap: 8px; border: 0; background: none; padding: 4px 6px; border-radius: 6px; font-weight: 650; color: var(--text); }
  .user:hover { background: var(--hover); }
  .avatar { width: 26px; height: 26px; border-radius: 50%; display: grid; place-items: center; background: linear-gradient(135deg, #f4c542, #e5483b); color: #1b1a18; font-weight: 800; font-size: 13px; }
  .uname { max-width: 150px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .add { display: flex; align-items: center; gap: 8px; height: 36px; padding: 0 6px; border: 0; background: none; border-radius: 6px; color: var(--accent); font-weight: 650; }
  .add:hover { background: var(--hover); }
  .add kbd, .hint { margin-left: auto; opacity: 0; transition: opacity 120ms; }
  .add:hover kbd, .nav:hover .hint { opacity: 1; }
  .nav {
    display: flex; align-items: center; gap: 10px; height: 34px; padding: 0 8px; border-radius: 6px; border: 0;
    background: none; color: var(--text-2); text-decoration: none; font-weight: 500; width: 100%; text-align: left;
  }
  .nav:hover { background: var(--hover); text-decoration: none; color: var(--text); }
  .nav.current { background: var(--accent-soft); color: #ff8a7f; }
  .nav.current :global(svg) { color: #ff8a7f; }
  .nav.drop { background: rgba(122, 167, 255, 0.18); }
  .nav.sub { height: 32px; }
  .nav.faint { color: var(--muted); }
  .over-before { box-shadow: inset 0 2px 0 var(--link); }
  .over-after { box-shadow: inset 0 -2px 0 var(--link); }
  .count { margin-left: auto; font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .nav.sub .more { display: none; margin-left: auto; }
  .nav.sub:hover .more { display: grid; }
  .nav.sub:hover .count { display: none; }
  .todayicon { position: relative; display: grid; place-items: center; }
  .todayicon b { position: absolute; top: 6px; font-size: 8px; font-weight: 800; }
  .scroll { flex: 1; overflow-y: auto; margin: 10px -10px 0; padding: 0 10px 20px; }
  .group { margin-top: 14px; }
  .ghead { display: flex; align-items: center; justify-content: space-between; height: 30px; padding: 0 0 0 8px; border: 0; background: none; width: 100%; color: var(--muted); font-weight: 650; font-size: 12.5px; }
  .glabel { display: flex; align-items: center; gap: 4px; border: 0; background: none; padding: 0; color: inherit; font: inherit; }
  .ghead :global(.closed) { transform: rotate(-90deg); }
  .mini { width: 24px; height: 24px; }
  .emoji { width: 18px; text-align: center; font-size: 14px; }
  .ellipsis { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .empty { border: 1px dashed var(--border); background: none; color: var(--muted); border-radius: 6px; padding: 8px; width: 100%; margin-top: 4px; font-size: 13px; }
  .empty:hover { color: var(--text); border-color: var(--border-strong); }
</style>
