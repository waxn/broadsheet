<script>
  import { ChevronLeft, ChevronRight, Plus, PanelRight } from "@lucide/svelte";
  import MonthGrid from "../components/MonthGrid.svelte";
  import TimeGrid from "../components/TimeGrid.svelte";
  import TaskCheck from "../components/TaskCheck.svelte";
  import { app, loadEvents, loadHabits, localGet, localSet, toggleTodo } from "../lib/store.svelte.js";
  import { go, route } from "../lib/router.svelte.js";
  import { startDrag, endDrag } from "../lib/drag.svelte.js";
  import { shortcut } from "../lib/keys.js";
  import { hex } from "../lib/colors.js";
  import * as D from "../lib/dates.js";

  let view = $state(route.query.get("view") || localGet("calView", "month"));
  let date = $state(route.query.get("date") || D.today());
  let tray = $state(localGet("calTray", true));
  const today = $derived(D.today());

  $effect(() => {
    const q = route.query;
    if (q.get("date")) date = q.get("date");
    if (q.get("view")) view = q.get("view");
  });

  const days = $derived(view === "day" ? [date] : D.range(D.startOfWeek(date), 7));
  const month = $derived(D.startOfMonth(date));
  const range = $derived(view === "month" ? [D.startOfWeek(month), D.addDays(D.startOfWeek(month), 42)] : [days[0], days[days.length - 1]]);

  $effect(() => {
    loadEvents(D.addDays(range[0], -7), D.addDays(range[1], 7));
  });
  $effect(() => {
    if (!app.checks.length) loadHabits();
  });

  const title = $derived.by(() => {
    const dt = D.parse(date);
    if (view === "day") return { big: `${D.DAY_NAMES[D.weekday(date)]}`, small: `${D.MONTHS[dt.getMonth()]} ${dt.getDate()}, ${dt.getFullYear()}` };
    if (view === "week") {
      const a = D.parse(days[0]);
      const b = D.parse(days[6]);
      return a.getMonth() === b.getMonth()
        ? { big: D.MONTHS[a.getMonth()], small: `${a.getDate()}–${b.getDate()}, ${a.getFullYear()}` }
        : { big: `${D.MONTH_SHORT[a.getMonth()]} – ${D.MONTH_SHORT[b.getMonth()]}`, small: `${a.getDate()} – ${b.getDate()}, ${b.getFullYear()}` };
    }
    return { big: D.MONTHS[dt.getMonth()], small: String(dt.getFullYear()) };
  });

  function setView(v) {
    view = v;
    localSet("calView", v);
  }

  function step(n) {
    date = view === "month" ? D.addMonths(D.startOfMonth(date), n) : D.addDays(date, n * (view === "week" ? 7 : 1));
  }

  function pickDay(d) {
    date = d;
    setView("day");
  }

  function newEvent() {
    const d = view === "month" ? (date.slice(0, 7) === today.slice(0, 7) ? today : D.startOfMonth(date)) : view === "day" ? date : today >= days[0] && today <= days[6] ? today : days[0];
    app.eventEditor = { event: { title: "", all_day: 0, start_at: D.join(d, "09:00"), end_at: D.join(d, "10:00") } };
  }

  // Tasks waiting for a time: overdue or due in view, without a block.
  const unscheduled = $derived(
    app.todos
      .filter((t) => !t.done && t.parent_id == null && !t.start_at && (!t.due_date || t.due_date <= range[1]))
      .sort((a, b) => (a.due_date || "9999").localeCompare(b.due_date || "9999") || b.priority - a.priority)
      .slice(0, 60),
  );

  function key(e) {
    if (!shortcut(e)) return;
    const k = e.key.toLowerCase();
    if (k === "d") setView("day");
    else if (k === "w") setView("week");
    else if (k === "m") setView("month");
    else if (k === "t") date = today;
    else if (k === "c" || k === "n") newEvent();
    else if (e.key === "ArrowLeft" || k === "h") step(-1);
    else if (e.key === "ArrowRight" || k === "l") step(1);
    else return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={key} />

<div class="cal">
  <header class="ch">
    <h1>{title.big} <span>{title.small}</span></h1>
    {#if app.countdowns.length}
      <div class="cds">
        {#each app.countdowns.slice(0, 3) as c (c.id)}
          {@const n = D.diffDays(c.start_at.slice(0, 10), today)}
          <button type="button" class="cd" style:--c={hex(c.color || c.class_color, "#f4c542")} onclick={() => (app.eventEditor = { event: c })}><b>{n === 0 ? "Today" : n}</b>{n === 0 ? "" : n === 1 ? " day" : " days"} · {c.title}</button>
        {/each}
      </div>
    {/if}
    <span class="spacer"></span>
    <button type="button" class="btn ghost sm" onclick={() => (date = today)}>Today</button>
    <button type="button" class="icon-btn" aria-label="Previous" onclick={() => step(-1)}><ChevronLeft size={18} /></button>
    <button type="button" class="icon-btn" aria-label="Next" onclick={() => step(1)}><ChevronRight size={18} /></button>
    <div class="seg" role="group" aria-label="View">
      <button type="button" class:on={view === "day"} onclick={() => setView("day")}>Day</button>
      <button type="button" class:on={view === "week"} onclick={() => setView("week")}>Week</button>
      <button type="button" class:on={view === "month"} onclick={() => setView("month")}>Month</button>
    </div>
    <button type="button" class="btn primary sm" onclick={newEvent}><Plus size={15} /> Event</button>
    <button type="button" class="icon-btn" class:on={tray} aria-label="Toggle task tray" title="Tasks to schedule" onclick={() => { tray = !tray; localSet("calTray", tray); }}><PanelRight size={18} /></button>
  </header>

  <div class="main">
    <div class="grid">
      {#if view === "month"}
        <MonthGrid {month} onpickday={pickDay} />
      {:else}
        {#key view + days[0]}<TimeGrid {days} onpickday={pickDay} />{/key}
      {/if}
    </div>
    {#if tray}
      <aside class="tray">
        <h2>To schedule</h2>
        <p class="hint">Drag a task onto the calendar to book time for it.</p>
        {#each unscheduled as t (t.id)}
          <div class="tt" draggable="true" ondragstart={(e) => startDrag(e, "task", t.id)} ondragend={endDrag} role="listitem">
            <TaskCheck done={!!t.done} priority={t.priority} size={15} onclick={() => toggleTodo(t.id)} />
            <button type="button" class="ttl" onclick={() => (app.detail = t.id)}>
              <span>{t.title}</span>
              <small>
                {#if t.due_date}<span class="due {D.tone(t.due_date)}">{D.label(t.due_date)}</span>{/if}
                {#if t.class_name}<span style:color={hex(t.class_color)}>#{t.class_name}</span>{/if}
              </small>
            </button>
          </div>
        {:else}
          <p class="hint">Nothing waiting. Nice.</p>
        {/each}
      </aside>
    {/if}
  </div>
</div>

<style>
  .cal { height: 100%; display: flex; flex-direction: column; padding: 18px 20px 14px 28px; }
  .ch { display: flex; align-items: center; gap: 6px; margin-bottom: 14px; flex-wrap: wrap; }
  h1 { margin: 0 12px 0 0; font-size: 30px; font-weight: 850; letter-spacing: -0.03em; }
  h1 span { color: var(--muted); font-weight: 600; letter-spacing: -0.01em; font-size: 22px; }
  .cds { display: flex; gap: 6px; }
  .cd { border: 1px solid color-mix(in srgb, var(--c) 35%, transparent); background: color-mix(in srgb, var(--c) 12%, transparent); color: var(--text-2); border-radius: 999px; padding: 3px 10px; font-size: 12.5px; }
  .cd b { color: var(--c); font-weight: 800; }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; margin: 0 4px; }
  .seg button { border: 0; background: transparent; padding: 5px 12px; color: var(--muted); font-size: 12.5px; font-weight: 650; }
  .seg button.on { background: var(--active); color: var(--text); }
  .icon-btn.on { color: var(--text); background: var(--hover); }
  .main { flex: 1; display: flex; gap: 14px; min-height: 0; }
  .grid { flex: 1; min-width: 0; min-height: 0; }
  .tray { width: 250px; flex: none; overflow-y: auto; padding: 0 2px 0 14px; border-left: 1px solid var(--border); }
  .tray h2 { font-size: 13px; font-weight: 750; margin: 2px 0 2px; }
  .hint { font-size: 12px; color: var(--muted); margin: 0 0 10px; }
  .tt { display: flex; align-items: flex-start; gap: 8px; padding: 7px 6px; border-radius: 8px; cursor: grab; border: 1px solid transparent; }
  .tt:hover { background: var(--surface); border-color: var(--border); }
  .ttl { flex: 1; min-width: 0; border: 0; background: none; padding: 0; text-align: left; display: flex; flex-direction: column; gap: 1px; font-size: 13px; color: var(--text); }
  .ttl > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  small { display: flex; gap: 8px; font-size: 11.5px; color: var(--muted); }
  .due.overdue { color: var(--overdue); }
  .due.today { color: var(--today); }
  .due.tomorrow { color: var(--tomorrow); }
  .due.week { color: var(--week); }
</style>
