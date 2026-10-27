<script>
  import { CalendarDays, Repeat, GitBranch, FileText, Pencil, Ellipsis, Hash, Inbox, GripVertical, ChevronRight, Flag, Sun, CalendarArrowUp, Ban, Copy, Trash2, FolderInput, CalendarClock, Clock } from "@lucide/svelte";
  import TaskCheck from "./TaskCheck.svelte";
  import { app, childrenOf, createTodo, deleteTodo, toggleTodo, updateTodo } from "../lib/store.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { placeItems, priorityItems } from "../lib/pickers.js";
  import { hex } from "../lib/colors.js";
  import { startDrag, endDrag } from "../lib/drag.svelte.js";
  import * as D from "../lib/dates.js";

  let { task, showClass = false, showDate = true, depth = 0, expanded = true, ontoggle, draggable = true, compact = false } = $props();

  const kids = $derived(childrenOf(task.id));
  const kidsDone = $derived(kids.filter((k) => k.done).length);
  const today = $derived(D.today());
  const tone = $derived(task.due_date ? D.tone(task.due_date, today) : null);
  const descLine = $derived(task.description?.split("\n").find((l) => l.trim()) ?? "");
  const block = $derived(task.start_at && task.end_at ? `${D.fmtTime(D.timePart(task.start_at))}–${D.fmtTime(D.timePart(task.end_at))}` : "");

  function open() {
    app.selected = task.id;
    app.detail = task.id;
  }

  function more(e) {
    const t = task;
    openMenu(e, [
      { label: "Edit task", icon: Pencil, hint: "↵", action: open },
      { sep: true },
      { heading: "Date" },
      { label: "Today", icon: CalendarDays, color: "var(--today)", action: () => updateTodo(t.id, { due_date: today }) },
      { label: "Tomorrow", icon: Sun, color: "var(--tomorrow)", action: () => updateTodo(t.id, { due_date: D.addDays(today, 1) }) },
      { label: "Next week", icon: CalendarArrowUp, color: "var(--week)", action: () => updateTodo(t.id, { due_date: D.addDays(D.startOfWeek(today), 7) }) },
      { label: "Pick a date…", icon: CalendarClock, hint: "D", action: () => (app.datePickerFor = { id: t.id, anchor: e.target }) },
      ...(t.due_date ? [{ label: "No date", icon: Ban, action: () => updateTodo(t.id, { due_date: null, due_time: null, recurrence: null }) }] : []),
      { sep: true },
      { heading: "Priority" },
      ...priorityItems(t.priority, (p) => updateTodo(t.id, { priority: p })),
      { sep: true },
      { label: "Move to…", icon: FolderInput, action: () => openMenu({ x: e.clientX, y: e.clientY }, placeItems(t, (place) => updateTodo(t.id, place))) },
      { label: "Duplicate", icon: Copy, action: () => duplicate(t) },
      { sep: true },
      { label: "Delete task", icon: Trash2, danger: true, action: () => deleteTodo(t.id) },
    ]);
  }

  async function duplicate(t) {
    const { title, description, due_date, due_time, priority, class_id, section_id, parent_id, recurrence } = t;
    await createTodo({ title, description, due_date, due_time, priority, class_id, section_id, parent_id, recurrence });
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<div
  class="row"
  class:done={task.done}
  class:selected={app.selected === task.id}
  class:compact
  style:padding-left="{depth * 28}px"
  data-task-id={task.id}
  draggable={draggable ? "true" : undefined}
  ondragstart={(e) => startDrag(e, "task", task.id)}
  ondragend={endDrag}
  oncontextmenu={more}
  onclick={open}
  role="listitem"
>
  {#if draggable}<span class="grip" aria-hidden="true"><GripVertical size={14} /></span>{/if}
  {#if kids.length}
    <button type="button" class="twist" class:open={expanded} aria-label={expanded ? "Hide subtasks" : "Show subtasks"} onclick={(e) => { e.stopPropagation(); ontoggle?.(); }}><ChevronRight size={14} /></button>
  {:else}
    <span class="twist-space"></span>
  {/if}
  <span class="checkcell"><TaskCheck done={!!task.done} priority={task.priority} onclick={() => toggleTodo(task.id)} /></span>
  <div class="body">
    <div class="title">{task.title}</div>
    {#if descLine && !compact}<div class="desc">{descLine}</div>{/if}
    <div class="meta">
      {#if showDate && task.due_date}
        <span class="due {tone}"><CalendarDays size={12} /> {D.label(task.due_date, today)}{task.due_time ? ` ${D.fmtTime(task.due_time)}` : ""}{#if task.recurrence}<Repeat size={11} />{/if}</span>
      {:else if task.due_time}
        <span class="due"><Clock size={12} /> {D.fmtTime(task.due_time)}{#if task.recurrence}<Repeat size={11} />{/if}</span>
      {:else if task.recurrence}
        <span class="due"><Repeat size={12} /></span>
      {/if}
      {#if block}<span class="m"><CalendarClock size={12} /> {block}</span>{/if}
      {#if kids.length}<span class="m"><GitBranch size={12} /> {kidsDone}/{kids.length}</span>{/if}
    </div>
  </div>
  <div class="right">
    <div class="actions">
      <button type="button" class="icon-btn" aria-label="Edit" onclick={(e) => { e.stopPropagation(); open(); }}><Pencil size={15} /></button>
      <button type="button" class="icon-btn" aria-label="Set date" onclick={(e) => { e.stopPropagation(); app.datePickerFor = { id: task.id, anchor: e.currentTarget }; }}><CalendarDays size={15} /></button>
      <button type="button" class="icon-btn" aria-label="More actions" onclick={(e) => { e.stopPropagation(); more(e); }}><Ellipsis size={16} /></button>
    </div>
    {#if showClass}
      <span class="place">
        {task.class_name ?? "Inbox"}
        {#if task.class_id}<Hash size={12} color={hex(task.class_color)} />{:else}<Inbox size={12} />{/if}
      </span>
    {/if}
  </div>
</div>

<style>
  .row {
    position: relative; display: flex; align-items: flex-start; gap: 4px; padding: 9px 0 9px; margin-left: -40px; padding-right: 4px;
    border-bottom: 1px solid var(--border); cursor: pointer;
  }
  .row.compact { padding: 6px 0; }
  .row > :first-child { margin-left: 0; }
  .grip { width: 18px; height: 22px; display: grid; place-items: center; color: var(--faint); opacity: 0; cursor: grab; flex: none; }
  .twist, .twist-space { width: 22px; height: 22px; flex: none; }
  .twist { display: grid; place-items: center; border: 0; background: none; color: var(--muted); padding: 0; border-radius: 4px; }
  .twist:hover { background: var(--hover); }
  .twist :global(svg) { transition: transform 120ms; }
  .twist.open :global(svg) { transform: rotate(90deg); }
  .row:hover .grip { opacity: 1; }
  .checkcell { padding-top: 2px; }
  .body { flex: 1; min-width: 0; padding-left: 6px; }
  .title { font-size: 14px; line-height: 20px; color: var(--text); overflow-wrap: anywhere; }
  .done .title { color: var(--muted); text-decoration: line-through; }
  .desc { font-size: 12.5px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 1px; }
  .meta { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 3px; font-size: 12px; color: var(--muted); }
  .meta:empty { display: none; }
  .due, .m { display: inline-flex; align-items: center; gap: 4px; }
  .due.overdue { color: var(--overdue); }
  .due.today { color: var(--today); }
  .due.tomorrow { color: var(--tomorrow); }
  .due.week { color: var(--week); }
  .right { align-self: flex-end; display: flex; align-items: flex-end; min-height: 18px; }
  .actions { position: absolute; right: 2px; top: 6px; display: flex; gap: 2px; padding-left: 12px; opacity: 0; pointer-events: none; background: linear-gradient(90deg, transparent, var(--bg) 12px); }
  .row:hover .actions, .row.selected .actions { opacity: 1; pointer-events: auto; }
  .row.selected .actions { background: linear-gradient(90deg, transparent, var(--hover) 12px); }
  .place { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--muted); white-space: nowrap; }
  .row.selected { background: linear-gradient(90deg, transparent 40px, var(--hover) 40px); }
  .row.selected::before { content: ""; position: absolute; left: 36px; top: 6px; bottom: 6px; width: 2px; border-radius: 2px; background: var(--accent); }
</style>
