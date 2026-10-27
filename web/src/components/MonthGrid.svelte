<script>
  // Hey-style month: big day tiles with events, due tasks and habit check-ins. Click a day to add
  // an event; drag events and tasks between days.
  import TaskCheck from "./TaskCheck.svelte";
  import { app, isChecked, toggleHabit, toggleTodo, updateEvent, updateTodo } from "../lib/store.svelte.js";
  import { drag, endDrag, startDrag } from "../lib/drag.svelte.js";
  import { covers, eventColor, taskColor } from "../lib/cal.js";
  import * as D from "../lib/dates.js";

  let { month, onpickday } = $props();

  const today = $derived(D.today());
  const weeks = $derived.by(() => {
    const start = D.startOfWeek(month);
    const last = D.addDays(D.addMonths(month, 1), -1);
    const n = Math.ceil((D.diffDays(last, start) + 1) / 7);
    return Array.from({ length: n }, (_, w) => D.range(D.addDays(start, w * 7), 7));
  });
  const habits = $derived(app.habits.filter((h) => !h.archived));
  let over = $state(null);
  const MAX = 4;

  function itemsFor(day) {
    const events = app.events.filter((e) => covers(e, day)).sort((a, b) => b.all_day - a.all_day || a.start_at.localeCompare(b.start_at));
    const tasks = app.todos
      .filter((t) => t.parent_id == null && (t.due_date === day || (t.start_at && D.datePart(t.start_at) === day && t.due_date !== day)))
      .sort((a, b) => a.done - b.done || (a.due_time || "99").localeCompare(b.due_time || "99") || b.priority - a.priority);
    return [...events.map((item) => ({ kind: "event", item })), ...tasks.map((item) => ({ kind: "task", item }))];
  }

  function newEvent(day, e) {
    if (e.target.closest(".it, .more, .hb, .daynum")) return;
    app.eventEditor = { event: { title: "", all_day: 1, start_at: day, end_at: day } };
  }

  function dragOver(e, day) {
    if (drag.kind !== "task" && drag.kind !== "event") return;
    e.preventDefault();
    over = day;
  }

  function drop(e, day) {
    e.preventDefault();
    over = null;
    if (drag.kind === "task") {
      updateTodo(drag.id, { due_date: day, ...(app.todos.find((t) => t.id === drag.id)?.start_at ? { start_at: null, end_at: null } : {}) });
    } else if (drag.kind === "event") {
      const ev = app.events.find((x) => x.id === drag.id);
      if (ev) {
        const shift = D.diffDays(day, D.datePart(ev.start_at));
        const move = (s) => (ev.all_day ? D.addDays(s, shift) : D.join(D.addDays(D.datePart(s), shift), D.timePart(s)));
        updateEvent(ev.id, { start_at: move(ev.start_at), end_at: move(ev.end_at) });
      }
    }
    endDrag();
  }
</script>

<div class="mg">
  <div class="wds">{#each D.DAY_SHORT as d}<span>{d}</span>{/each}</div>
  <div class="grid" style:grid-template-rows="repeat({weeks.length}, 1fr)">
    {#each weeks.flat() as day (day)}
      {@const items = itemsFor(day)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div
        class="cell"
        class:other={day.slice(0, 7) !== month.slice(0, 7)}
        class:today={day === today}
        class:past={day < today}
        class:weekend={D.weekday(day) >= 5}
        class:over={over === day}
        onclick={(e) => newEvent(day, e)}
        ondragover={(e) => dragOver(e, day)}
        ondragleave={() => (over = null)}
        ondrop={(e) => drop(e, day)}
      >
        <button type="button" class="daynum" onclick={() => onpickday?.(day)} aria-label="Open {D.long(day)}">{Number(day.slice(8)) === 1 ? `${D.MONTH_SHORT[D.parse(day).getMonth()]} 1` : Number(day.slice(8))}</button>
        <div class="items">
          {#each items.slice(0, MAX) as { kind, item } (kind + item.id)}
            {#if kind === "event"}
              <button
                type="button"
                class="it ev"
                class:allday={item.all_day}
                style:--c={eventColor(item)}
                draggable="true"
                ondragstart={(e) => startDrag(e, "event", item.id)}
                ondragend={endDrag}
                onclick={() => (app.eventEditor = { event: item })}
              >
                {#if !item.all_day}<span class="dot"></span><b>{D.fmtTime(D.timePart(item.start_at))}</b>{/if}
                {item.countdown ? "⏳ " : ""}{item.title}
              </button>
            {:else}
              <div class="it task" class:done={item.done} style:--c={taskColor(item)} draggable="true" ondragstart={(e) => startDrag(e, "task", item.id)} ondragend={endDrag} role="button" tabindex="-1" onclick={() => (app.detail = item.id)}>
                <TaskCheck done={!!item.done} priority={item.priority} size={13} onclick={() => toggleTodo(item.id)} />
                {#if item.due_time}<b>{D.fmtTime(item.due_time)}</b>{/if}
                <span class="tt">{item.title}</span>
              </div>
            {/if}
          {/each}
          {#if items.length > MAX}
            <button type="button" class="more" onclick={() => onpickday?.(day)}>+{items.length - MAX} more</button>
          {/if}
        </div>
        {#if habits.length && day <= today && day.slice(0, 7) === month.slice(0, 7)}
          <div class="habits">
            {#each habits as h (h.id)}
              <button type="button" class="hb" class:on={isChecked(h.id, day)} title={h.name} onclick={() => toggleHabit(h.id, day)}>{h.emoji || h.name[0]}</button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .mg { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .wds { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); padding: 0 0 6px; }
  .wds span { font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: .06em; color: var(--muted); padding-left: 10px; }
  .grid { flex: 1; display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 4px; min-height: 0; }
  .cell {
    position: relative; min-height: 0; overflow: hidden; padding: 6px 6px 4px; border-radius: 10px; background: var(--surface);
    display: flex; flex-direction: column; gap: 3px; cursor: pointer; transition: background 100ms;
  }
  .cell:hover { background: var(--raised); }
  .cell.weekend { background: #23221f; }
  .cell.other { opacity: 0.45; }
  .cell.over { box-shadow: inset 0 0 0 2px var(--link); }
  .cell.today { background: color-mix(in srgb, var(--yellow) 9%, var(--surface)); box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--yellow) 55%, transparent); }
  .daynum { align-self: flex-start; border: 0; background: none; padding: 0 4px; border-radius: 6px; font-size: 15px; font-weight: 800; color: var(--text-2); font-variant-numeric: tabular-nums; }
  .daynum:hover { background: var(--active); color: var(--text); }
  .cell.past .daynum { color: var(--muted); }
  .cell.today .daynum { color: #1b1a18; background: var(--yellow); }
  .items { display: flex; flex-direction: column; gap: 2px; min-height: 0; }
  .it { display: flex; align-items: center; gap: 4px; min-width: 0; border: 0; text-align: left; font-size: 12px; padding: 1px 5px; border-radius: 4px; background: none; color: var(--text-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; cursor: pointer; }
  .it:hover { background: var(--active); }
  .ev.allday { background: color-mix(in srgb, var(--c) 30%, transparent); color: color-mix(in srgb, var(--c) 35%, white); font-weight: 650; }
  .ev .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--c); flex: none; }
  .it b { font-weight: 650; color: var(--muted); font-variant-numeric: tabular-nums; }
  .task .tt { overflow: hidden; text-overflow: ellipsis; }
  .task.done .tt { text-decoration: line-through; color: var(--muted); }
  .more { border: 0; background: none; text-align: left; padding: 0 5px; font-size: 11.5px; color: var(--muted); font-weight: 600; }
  .more:hover { color: var(--text); }
  .habits { margin-top: auto; display: flex; flex-wrap: wrap; gap: 2px; }
  .hb { width: 18px; height: 18px; border-radius: 50%; border: 1px dashed var(--border-strong); background: none; font-size: 10px; padding: 0; opacity: 0.35; filter: grayscale(1); }
  .hb.on { opacity: 1; filter: none; border-style: solid; border-color: var(--today); background: rgba(79, 191, 106, 0.15); }
</style>
