<script>
  // A list of tasks with their subtasks, drag-to-reorder (drag right to make a subtask),
  // and an "Add task" row at the bottom.
  import { Plus } from "@lucide/svelte";
  import Self from "./TaskList.svelte";
  import TaskRow from "./TaskRow.svelte";
  import TaskComposer from "./TaskComposer.svelte";
  import { app, childrenOf, localGet, localSet, reorderTodos, updateTodo } from "../lib/store.svelte.js";
  import { drag, endDrag, half, moved } from "../lib/drag.svelte.js";

  let {
    tasks,
    depth = 0,
    sortable = true,
    showClass = false,
    showDate = true,
    showDone = false,
    addDefaults = null, // when set, shows "Add task" with these defaults
    addLabel = "Add task",
    place = {}, // { class_id, section_id } the list belongs to, for drops at the end
    compact = false,
  } = $props();

  let collapsed = $state(new Set(localGet("collapsedTasks", [])));
  let over = $state(null); // { id, where: "before" | "after", child: bool }
  let adding = $state(false);
  let endOver = $state(false);

  function toggle(id) {
    const next = new Set(collapsed);
    next.has(id) ? next.delete(id) : next.add(id);
    collapsed = next;
    localSet("collapsedTasks", [...next].slice(-500));
  }

  function isInside(id, maybeAncestor) {
    let t = app.todos.find((x) => x.id === id);
    for (let i = 0; t && i < 50; i++) {
      if (t.parent_id === maybeAncestor) return true;
      t = app.todos.find((x) => x.id === t.parent_id);
    }
    return false;
  }

  function dragover(e, t) {
    if (!sortable || drag.kind !== "task" || drag.id === t.id || isInside(t.id, drag.id)) return;
    e.preventDefault();
    e.stopPropagation();
    const body = e.currentTarget.querySelector(".body")?.getBoundingClientRect();
    over = { id: t.id, where: half(e), child: !!body && e.clientX > body.left + 40 };
  }

  function drop(e, t) {
    if (!over || drag.kind !== "task") return;
    e.preventDefault();
    e.stopPropagation();
    const id = drag.id;
    const { where, child } = over;
    over = null;
    endDrag();
    if (child) {
      const kids = childrenOf(t.id).filter((k) => k.id !== id);
      updateTodo(id, { parent_id: t.id, position: kids.length ? Math.max(...kids.map((k) => k.position)) + 1 : 0 });
      return;
    }
    const siblings = t.parent_id ? childrenOf(t.parent_id).filter((k) => showDone || !k.done) : tasks;
    const items = moved(siblings, id, t.id, where);
    const dragged = items.find((x) => x.id === id);
    const src = app.todos.find((x) => x.id === id);
    if (src && (src.parent_id ?? null) !== (t.parent_id ?? null)) dragged.parent_id = t.parent_id ?? null;
    if (!t.parent_id && src && (src.section_id ?? null) !== (t.section_id ?? null)) dragged.section_id = t.section_id ?? null;
    if (!t.parent_id && src && (src.class_id ?? null) !== (t.class_id ?? null) && t.section_id == null) dragged.class_id = t.class_id ?? null;
    if (dragged.parent_id === null && src?.parent_id == null) delete dragged.parent_id;
    reorderTodos(items);
  }

  function endDragover(e) {
    if (!sortable || drag.kind !== "task" || depth > 0) return;
    e.preventDefault();
    endOver = true;
  }

  function endDrop(e) {
    if (!endOver) return;
    e.preventDefault();
    endOver = false;
    const id = drag.id;
    endDrag();
    const items = moved(tasks, id, null, "after");
    const dragged = items.find((x) => x.id === id);
    dragged.parent_id = null;
    if ("section_id" in place) dragged.section_id = place.section_id ?? null;
    if ("class_id" in place && place.section_id == null) dragged.class_id = place.class_id ?? null;
    reorderTodos(items);
  }

  const visibleKids = (id) => childrenOf(id).filter((k) => showDone || !k.done);
</script>

<div class="list" role="list">
  {#each tasks as t (t.id)}
    {@const kids = visibleKids(t.id)}
    <div
      class="slot"
      class:before={over?.id === t.id && over.where === "before" && !over.child}
      class:after={over?.id === t.id && over.where === "after" && !over.child}
      class:child={over?.id === t.id && over.child}
      ondragover={(e) => dragover(e, t)}
      ondragleave={(e) => !e.currentTarget.contains(e.relatedTarget) && (over = null)}
      ondrop={(e) => drop(e, t)}
      role="presentation"
    >
      <TaskRow task={t} {depth} {showClass} {showDate} {compact} expanded={!collapsed.has(t.id)} ontoggle={() => toggle(t.id)} />
    </div>
    {#if kids.length && !collapsed.has(t.id)}
      <Self tasks={kids} depth={depth + 1} {sortable} {showClass} {showDate} {showDone} {compact} />
    {/if}
  {/each}

  {#if addDefaults}
    <div class="end" class:over={endOver} ondragover={endDragover} ondragleave={() => (endOver = false)} ondrop={endDrop} role="presentation">
      {#if adding}
        <TaskComposer defaults={addDefaults} oncancel={() => (adding = false)} />
      {:else}
        <button type="button" class="add" onclick={() => (adding = true)}>
          <span class="plus"><Plus size={16} /></span> {addLabel}
        </button>
      {/if}
    </div>
  {:else if sortable && depth === 0}
    <div class="end small" class:over={endOver} ondragover={endDragover} ondragleave={() => (endOver = false)} ondrop={endDrop} role="presentation"></div>
  {/if}
</div>

<style>
  .slot { position: relative; }
  .slot.before::after, .slot.after::after, .slot.child::after {
    content: ""; position: absolute; left: 0; right: 0; height: 2px; background: var(--link); border-radius: 2px; z-index: 2; pointer-events: none;
  }
  .slot.before::after { top: -1px; }
  .slot.after::after { bottom: -1px; }
  .slot.child::after { bottom: -1px; left: 34px; }
  .end { min-height: 8px; border-radius: 6px; }
  .end.small { min-height: 14px; }
  .end.over { box-shadow: inset 0 2px 0 var(--link); }
  .add { display: flex; align-items: center; gap: 8px; width: 100%; height: 38px; padding: 0; border: 0; background: none; color: var(--muted); font-size: 14px; }
  .plus { width: 18px; height: 18px; display: grid; place-items: center; color: var(--accent); border-radius: 50%; }
  .add:hover { color: var(--accent); }
  .add:hover .plus { background: var(--accent); color: #fff; }
</style>
