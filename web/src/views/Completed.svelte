<script>
  import ViewHeader from "../components/ViewHeader.svelte";
  import TaskList from "../components/TaskList.svelte";
  import { app, loadCompleted } from "../lib/store.svelte.js";
  import * as D from "../lib/dates.js";

  $effect(() => {
    loadCompleted();
  });
  const done = $derived(app.todos.filter((t) => t.done && t.parent_id == null).sort((a, b) => (b.completed_at || "").localeCompare(a.completed_at || "")));
  const groups = $derived.by(() => {
    const m = new Map();
    for (const t of done) {
      const day = (t.completed_at || "").slice(0, 10) || "Earlier";
      if (!m.has(day)) m.set(day, []);
      m.get(day).push(t);
    }
    return [...m];
  });
</script>

<div class="wrap">
  <ViewHeader title="Completed" subtitle={`${done.length} task${done.length === 1 ? "" : "s"}`} />
  {#each groups as [day, tasks] (day)}
    <h2>{day === "Earlier" ? day : `${D.label(day)} · ${D.long(day)}`}</h2>
    <TaskList {tasks} sortable={false} showClass showDone />
  {:else}
    <p class="muted">Nothing completed yet.</p>
  {/each}
</div>

<style>
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 56px 120px; }
  h2 { font-size: 14.5px; font-weight: 750; margin: 18px 0 0; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
</style>
