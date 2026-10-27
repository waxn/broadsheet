<script>
  import { Hourglass } from "@lucide/svelte";
  import ViewHeader from "../components/ViewHeader.svelte";
  import DayGroup from "../components/DayGroup.svelte";
  import { app, loadEvents, updateTodo } from "../lib/store.svelte.js";
  import * as D from "../lib/dates.js";
  import { hex } from "../lib/colors.js";

  const today = $derived(D.today());
  const byTime = (a, b) => (a.due_time || "99") .localeCompare(b.due_time || "99") || b.priority - a.priority || a.position - b.position;
  const open = $derived(app.todos.filter((t) => !t.done && t.parent_id == null));
  const overdue = $derived(open.filter((t) => t.due_date && t.due_date < today).sort((a, b) => a.due_date.localeCompare(b.due_date) || byTime(a, b)));
  const todays = $derived(open.filter((t) => t.due_date === today).sort(byTime));
  const doneToday = $derived(app.todos.filter((t) => t.done && t.completed_at?.slice(0, 10) === new Date().toISOString().slice(0, 10)).length);
  const events = $derived(app.events.filter((e) => e.start_at.slice(0, 10) <= today && e.end_at.slice(0, 10) >= today).sort((a, b) => b.all_day - a.all_day || a.start_at.localeCompare(b.start_at)));
  const countdowns = $derived(app.countdowns.slice(0, 4));

  $effect(() => {
    loadEvents(D.addDays(today, -7), D.addDays(today, 35));
  });

  function rescheduleAll() {
    for (const t of overdue) updateTodo(t.id, { due_date: today });
  }

  const total = $derived(todays.length + overdue.length);
</script>

<div class="wrap">
  <ViewHeader title="Today" subtitle={`${D.long(today)} · ${total ? `${total} task${total === 1 ? "" : "s"}` : "all clear"}${doneToday ? ` · ${doneToday} done` : ""}`} />

  {#if countdowns.length}
    <div class="countdowns">
      {#each countdowns as c (c.id)}
        {@const n = D.diffDays(c.start_at.slice(0, 10), today)}
        <div class="cd" style:--c={hex(c.color || c.class_color, "#f4c542")}>
          <b>{n === 0 ? "Today" : n}</b><span>{n === 0 ? c.title : `day${n === 1 ? "" : "s"} until ${c.title}`}</span>
        </div>
      {/each}
    </div>
  {/if}

  {#if overdue.length}
    <DayGroup title="Overdue" tasks={overdue} addable={false}>
      <button type="button" class="link" onclick={rescheduleAll}>Reschedule to today</button>
    </DayGroup>
  {/if}
  <DayGroup title={D.label(today)} sub={D.long(today)} day={today} tasks={todays} {events} />

  {#if !total}
    <div class="empty">
      <div class="sun">☀︎</div>
      <p><b>Enjoy your day</b></p>
      <p class="muted">Nothing due today. Press <kbd>Q</kbd> to add something.</p>
    </div>
  {/if}
</div>

<style>
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 56px 120px; }
  .link { border: 0; background: none; color: var(--accent); font-weight: 600; font-size: 13px; padding: 0; }
  .countdowns { display: flex; gap: 10px; flex-wrap: wrap; margin: 0 0 18px; }
  .cd { display: flex; align-items: baseline; gap: 8px; padding: 10px 14px; border-radius: 12px; background: color-mix(in srgb, var(--c) 14%, var(--surface)); border: 1px solid color-mix(in srgb, var(--c) 30%, transparent); }
  .cd b { font-size: 22px; font-weight: 850; color: var(--c); font-variant-numeric: tabular-nums; }
  .cd span { font-size: 13px; color: var(--text-2); font-weight: 550; }
  .empty { text-align: center; padding: 40px 0; }
  .sun { width: 72px; height: 72px; margin: 0 auto 14px; border-radius: 50%; display: grid; place-items: center; font-size: 36px; color: var(--yellow); background: rgba(244, 197, 66, 0.12); }
  .empty p { margin: 4px 0; }
</style>
