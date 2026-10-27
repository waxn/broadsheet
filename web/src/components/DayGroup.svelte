<script>
  // Tasks grouped under one day (or "Overdue"); dropping a task here moves it to that day.
  import TaskList from "./TaskList.svelte";
  import { updateTodo } from "../lib/store.svelte.js";
  import { drag, endDrag } from "../lib/drag.svelte.js";
  import { hex } from "../lib/colors.js";
  import * as D from "../lib/dates.js";

  let { title, sub = "", day = null, tasks, events = [], addable = true, showClass = true, children, id } = $props();
  let over = $state(false);

  function dragover(e) {
    if (drag.kind !== "task" || !day) return;
    e.preventDefault();
    over = true;
  }

  function drop(e) {
    if (!over) return;
    e.preventDefault();
    over = false;
    updateTodo(drag.id, { due_date: day });
    endDrag();
  }
</script>

<section class="group" class:over {id} ondragover={dragover} ondragleave={(e) => !e.currentTarget.contains(e.relatedTarget) && (over = false)} ondrop={drop} aria-label={title}>
  <h2>
    <span>{title}</span>{#if sub}<span class="sub">· {sub}</span>{/if}
    {#if children}<span class="act">{@render children()}</span>{/if}
  </h2>
  {#if events.length}
    <div class="events">
      {#each events as ev (ev.id)}
        <span class="ev" style:--c={hex(ev.color || ev.class_color, "#7aa7ff")}>
          {#if !ev.all_day}<b>{D.fmtTime(D.timePart(ev.start_at))}</b>{/if}
          {ev.title}
        </span>
      {/each}
    </div>
  {/if}
  <TaskList {tasks} sortable={false} {showClass} showDate={!day} addDefaults={addable && day ? { due_date: day } : null} />
</section>

<style>
  .group { margin-bottom: 22px; border-radius: 8px; }
  .group.over { background: rgba(122, 167, 255, 0.08); box-shadow: 0 0 0 2px rgba(122, 167, 255, 0.4); }
  h2 { display: flex; align-items: center; gap: 6px; margin: 0; padding: 10px 0 8px; font-size: 14.5px; font-weight: 750; border-bottom: 1px solid var(--border); }
  .sub { color: var(--muted); font-weight: 500; }
  .act { margin-left: auto; }
  .events { display: flex; flex-wrap: wrap; gap: 6px; padding: 8px 0 2px; }
  .ev { display: inline-flex; gap: 6px; align-items: center; padding: 3px 9px; border-radius: 6px; font-size: 12.5px; background: color-mix(in srgb, var(--c) 18%, transparent); color: color-mix(in srgb, var(--c) 70%, white); border-left: 3px solid var(--c); }
</style>
