<script>
  import { ChevronLeft, ChevronRight } from "@lucide/svelte";
  import ViewHeader from "../components/ViewHeader.svelte";
  import DayGroup from "../components/DayGroup.svelte";
  import { app, loadEvents } from "../lib/store.svelte.js";
  import * as D from "../lib/dates.js";

  const today = $derived(D.today());
  let weekStart = $state(D.startOfWeek(D.today()));
  let days = $state(28);
  const from = $derived(weekStart < today ? today : weekStart);
  const list = $derived(D.range(from, days));
  const strip = $derived(D.range(weekStart, 7));

  const open = $derived(app.todos.filter((t) => !t.done && t.parent_id == null && t.due_date));
  const overdue = $derived(weekStart <= today ? open.filter((t) => t.due_date < today) : []);
  const byDay = $derived.by(() => {
    const m = new Map();
    for (const t of open) {
      if (!m.has(t.due_date)) m.set(t.due_date, []);
      m.get(t.due_date).push(t);
    }
    for (const arr of m.values()) arr.sort((a, b) => (a.due_time || "99").localeCompare(b.due_time || "99") || b.priority - a.priority || a.position - b.position);
    return m;
  });
  const eventsOn = (d) => app.events.filter((e) => e.start_at.slice(0, 10) <= d && e.end_at.slice(0, 10) >= d).sort((a, b) => b.all_day - a.all_day || a.start_at.localeCompare(b.start_at));

  $effect(() => {
    loadEvents(from, D.addDays(from, days));
  });

  function jump(d) {
    if (d < from || d >= D.addDays(from, days)) {
      weekStart = D.startOfWeek(d);
    }
    requestAnimationFrame(() => document.getElementById(`day-${d}`)?.scrollIntoView({ behavior: "smooth", block: "start" }));
  }

  function shift(n) {
    weekStart = D.addDays(weekStart, 7 * n);
    if (weekStart < D.startOfWeek(today)) weekStart = D.startOfWeek(today);
    document.querySelector("main")?.scrollTo({ top: 0 });
  }
</script>

<div class="wrap">
  <ViewHeader title="Upcoming" subtitle={D.monthTitle(from)}>
    <button type="button" class="icon-btn" aria-label="Previous week" disabled={weekStart <= D.startOfWeek(today)} onclick={() => shift(-1)}><ChevronLeft size={18} /></button>
    <button type="button" class="btn ghost sm" onclick={() => (weekStart = D.startOfWeek(today))}>Today</button>
    <button type="button" class="icon-btn" aria-label="Next week" onclick={() => shift(1)}><ChevronRight size={18} /></button>
  </ViewHeader>

  <div class="strip">
    {#each strip as d}
      <button type="button" class="day" class:today={d === today} class:past={d < today} disabled={d < today} onclick={() => jump(d)}>
        <span class="wd">{D.DAY_SHORT[D.weekday(d)]}</span>
        <span class="num">{Number(d.slice(8))}</span>
        {#if byDay.get(d)?.length}<span class="pip"></span>{/if}
      </button>
    {/each}
  </div>

  {#if overdue.length}
    <DayGroup title="Overdue" tasks={overdue} addable={false} />
  {/if}
  {#each list as d (d)}
    <DayGroup id="day-{d}" title={`${D.MONTH_SHORT[D.parse(d).getMonth()]} ${Number(d.slice(8))}`} sub={[["Today", "Tomorrow"].includes(D.label(d, today)) ? D.label(d, today) : "", D.DAY_NAMES[D.weekday(d)]].filter(Boolean).join(" · ")} day={d} tasks={byDay.get(d) || []} events={eventsOn(d)} />
  {/each}
  <button type="button" class="btn ghost more" onclick={() => (days += 28)}>Show more days</button>
</div>

<style>
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 56px 120px; }
  .strip { display: grid; grid-template-columns: repeat(7, 1fr); gap: 4px; margin-bottom: 18px; padding-bottom: 10px; border-bottom: 1px solid var(--border); position: sticky; top: 84px; background: var(--bg); z-index: 4; }
  .day { position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 6px 0; border: 0; background: none; border-radius: 8px; color: var(--text-2); }
  .day:hover:not(:disabled) { background: var(--hover); }
  .day:disabled { cursor: default; color: var(--faint); }
  .wd { font-size: 11.5px; font-weight: 600; text-transform: uppercase; letter-spacing: .04em; }
  .num { font-size: 16px; font-weight: 750; font-variant-numeric: tabular-nums; }
  .day.today .num { color: #fff; background: var(--accent); border-radius: 50%; width: 28px; height: 28px; display: grid; place-items: center; }
  .pip { position: absolute; bottom: 0; width: 4px; height: 4px; border-radius: 50%; background: var(--muted); }
  .more { width: 100%; }
</style>
