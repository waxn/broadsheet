<script>
  import { PanelLeft, Minimize2 } from "@lucide/svelte";
  import Auth from "./components/Auth.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import Toasts from "./components/Toasts.svelte";
  import QuickAdd from "./components/QuickAdd.svelte";
  import TaskDetail from "./components/TaskDetail.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import Shortcuts from "./components/Shortcuts.svelte";
  import DatePicker from "./components/DatePicker.svelte";
  import EventEditor from "./components/EventEditor.svelte";
  import Today from "./views/Today.svelte";
  import Upcoming from "./views/Upcoming.svelte";
  import ProjectView from "./views/ProjectView.svelte";
  import Completed from "./views/Completed.svelte";
  import Trash from "./views/Trash.svelte";
  import Notes from "./views/Notes.svelte";
  import PageView from "./views/PageView.svelte";
  import Calendar from "./views/Calendar.svelte";
  import Habits from "./views/Habits.svelte";
  import { app, boot, deleteTodo, localSet, todoById, toggleTodo, updateTodo } from "./lib/store.svelte.js";
  import { go, route } from "./lib/router.svelte.js";
  import { isTyping, mod, overlayOpen, shortcut } from "./lib/keys.js";
  import * as D from "./lib/dates.js";

  boot();

  let pendingG = false;
  let gTimer;

  const titles = { today: "Today", inbox: "Inbox", upcoming: "Upcoming", calendar: "Calendar", notes: "Notes", habits: "Habits", completed: "Completed", trash: "Trash" };
  $effect(() => {
    let t = titles[route.name];
    if (route.name === "class") t = app.classes.find((c) => c.id === route.id)?.name;
    if (route.name === "page") t = app.pages.find((p) => p.id === route.id)?.title || "Untitled";
    document.title = t ? `${t} · Broadsheet` : "Broadsheet";
  });

  function taskIds() {
    return [...document.querySelectorAll("main [data-task-id]")].map((el) => Number(el.dataset.taskId));
  }

  function moveSelection(dir) {
    const ids = taskIds();
    if (!ids.length) return;
    const i = ids.indexOf(app.selected);
    const next = ids[i < 0 ? (dir > 0 ? 0 : ids.length - 1) : Math.max(0, Math.min(ids.length - 1, i + dir))];
    app.selected = next;
    document.querySelector(`main [data-task-id="${next}"]`)?.scrollIntoView({ block: "nearest" });
  }

  function quickAddHere() {
    const defaults = {};
    if (route.name === "class") defaults.class_id = route.id;
    if (route.name === "today") defaults.due_date = D.today();
    app.quickAdd = { defaults };
  }

  function keydown(e) {
    // Shortcuts with modifiers work even while typing.
    if (mod(e) && e.key.toLowerCase() === "k" && !isTyping(e)) {
      e.preventDefault();
      app.palette = !app.palette;
      return;
    }
    if (mod(e) && e.key.toLowerCase() === "k" && isTyping(e) && !e.target.closest(".ProseMirror")) {
      e.preventDefault();
      app.palette = true;
      return;
    }
    if (mod(e) && e.shiftKey && e.key.toLowerCase() === "f") {
      e.preventDefault();
      app.focus = !app.focus;
      return;
    }
    if (e.key === "Escape" && app.focus && !overlayOpen()) {
      app.focus = false;
      return;
    }
    if (!shortcut(e)) return;

    const k = e.key;
    if (pendingG) {
      pendingG = false;
      clearTimeout(gTimer);
      const dest = { i: "/inbox", t: "/today", u: "/upcoming", c: "/calendar", n: "/notes", h: "/habits", p: "/notes" }[k.toLowerCase()];
      if (dest) {
        e.preventDefault();
        go(dest);
      }
      return;
    }
    if (k === "g") {
      pendingG = true;
      gTimer = setTimeout(() => (pendingG = false), 1200);
      return;
    }
    if (k === "q" || k === "n") {
      if (k === "n" && route.name === "calendar") return; // the calendar uses its own keys
      e.preventDefault();
      quickAddHere();
      return;
    }
    if (k === "/") {
      e.preventDefault();
      app.palette = true;
      return;
    }
    if (k === "?") {
      app.help = true;
      return;
    }
    if (k === "[") {
      app.sidebar = !app.sidebar;
      localSet("sidebar", app.sidebar);
      return;
    }
    if (route.name === "calendar" || route.name === "page") return;

    // Task list keys.
    if (k === "j" || k === "ArrowDown") return (e.preventDefault(), moveSelection(1));
    if (k === "k" || k === "ArrowUp") return (e.preventDefault(), moveSelection(-1));
    const sel = app.selected && todoById(app.selected) ? app.selected : null;
    if (!sel) return;
    if (k === "Enter" || k === "e") return (e.preventDefault(), (app.detail = sel));
    if (k === "x") return toggleTodo(sel);
    if (["1", "2", "3", "4", "0"].includes(k)) return updateTodo(sel, { priority: k === "0" || k === "4" ? 0 : 4 - Number(k) });
    if (k === "d") {
      e.preventDefault();
      const anchor = document.querySelector(`main [data-task-id="${sel}"] .body`);
      if (anchor) app.datePickerFor = { id: sel, anchor };
      return;
    }
    if (k === "Delete" || k === "#") {
      const t = todoById(sel);
      if (t && confirm(`Delete “${t.title}”?`)) {
        moveSelection(1);
        deleteTodo(sel);
      }
    }
  }

  const pickerTask = $derived(app.datePickerFor ? todoById(app.datePickerFor.id) : null);
</script>

<svelte:window onkeydown={keydown} />

{#if !app.loaded}
  <div class="splash"><img src="/favicon.svg" alt="" width="44" height="44" /></div>
{:else if !app.user}
  <Auth />
{:else}
  <div class="shell" class:focus={app.focus}>
    {#if app.sidebar && !app.focus}
      <Sidebar />
    {/if}
    <main class:calendar={route.name === "calendar"}>
      {#if !app.sidebar && !app.focus}
        <button type="button" class="icon-btn opener" aria-label="Show sidebar ([)" title="Show sidebar  [" onclick={() => { app.sidebar = true; localSet("sidebar", true); }}><PanelLeft size={18} /></button>
      {/if}
      {#if app.focus}
        <button type="button" class="btn ghost exit" onclick={() => (app.focus = false)}><Minimize2 size={15} /> Exit focus <kbd>Esc</kbd></button>
      {/if}
      {#if route.name === "today"}<Today />
      {:else if route.name === "upcoming"}<Upcoming />
      {:else if route.name === "inbox"}<ProjectView classId={null} />
      {:else if route.name === "class"}{#key route.id}<ProjectView classId={route.id} />{/key}
      {:else if route.name === "completed"}<Completed />
      {:else if route.name === "trash"}<Trash />
      {:else if route.name === "notes"}<Notes />
      {:else if route.name === "page"}{#key route.id}<PageView pageId={route.id} />{/key}
      {:else if route.name === "calendar"}<Calendar />
      {:else if route.name === "habits"}<Habits />
      {/if}
    </main>
  </div>

  {#if app.quickAdd}<QuickAdd defaults={app.quickAdd.defaults} />{/if}
  {#if app.detail}{#key app.detail}<TaskDetail id={app.detail} />{/key}{/if}
  {#if app.palette}<CommandPalette />{/if}
  {#if app.help}<Shortcuts />{/if}
  {#if app.eventEditor}{#key app.eventEditor}<EventEditor event={app.eventEditor.event} />{/key}{/if}
  {#if app.datePickerFor && pickerTask}
    <DatePicker
      anchor={app.datePickerFor.anchor}
      value={pickerTask}
      onchange={(v) => updateTodo(pickerTask.id, { due_date: v.due_date ?? null, due_time: v.due_time ?? null, recurrence: v.recurrence ?? null })}
      onclose={() => (app.datePickerFor = null)}
    />
  {/if}
{/if}

<ContextMenu />
<Toasts />

<style>
  .splash { height: 100%; display: grid; place-items: center; animation: fade 400ms ease-out; }
  .shell { display: flex; height: 100%; }
  main { flex: 1; min-width: 0; height: 100%; overflow-y: auto; position: relative; }
  main.calendar { overflow: hidden; }
  .opener { position: fixed; top: 12px; left: 12px; z-index: 20; }
  .exit { position: fixed; top: 12px; right: 16px; z-index: 20; opacity: 0.35; transition: opacity 150ms; }
  .exit:hover { opacity: 1; }
</style>
