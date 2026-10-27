<script>
  // Week and day view: an all-day row, then hours. Drag on empty space to make an event, drag
  // events and task blocks to move them, pull their bottom edge to resize, and drop tasks from
  // the tray to book time for them.
  import { onMount } from "svelte";
  import { Repeat } from "@lucide/svelte";
  import TaskCheck from "./TaskCheck.svelte";
  import { app, isChecked, toggleHabit, toggleTodo, updateEvent, updateTodo } from "../lib/store.svelte.js";
  import { drag as dnd, endDrag, startDrag } from "../lib/drag.svelte.js";
  import { HOUR, covers, endOn, eventColor, fmtRange, layoutDay, snap, startOn, taskColor } from "../lib/cal.js";
  import * as D from "../lib/dates.js";

  let { days, onpickday } = $props();

  const today = $derived(D.today());
  let scroller = $state();
  let cols = $state();
  let now = $state(new Date());
  let op = $state(null); // the pointer gesture in progress
  let dropHint = $state(null); // { day, start } while dragging a task in from the tray

  onMount(() => {
    const first = Math.min(...[...app.events, ...app.todos].filter((x) => x.start_at && days.includes(D.datePart(x.start_at))).map((x) => D.minutes(D.timePart(x.start_at))), 8 * 60);
    scroller.scrollTop = Math.max(0, ((Math.min(first, 8 * 60) - 30) / 60) * HOUR);
    const t = setInterval(() => (now = new Date()), 60_000);
    return () => clearInterval(t);
  });

  const habits = $derived(app.habits.filter((h) => !h.archived));

  function timedFor(day) {
    const entries = [];
    for (const e of app.events) {
      if (e.all_day || !covers(e, day)) continue;
      const start = startOn(e, day);
      const end = endOn(e, day);
      if (end > start) entries.push({ kind: "event", item: e, start, end });
    }
    for (const t of app.todos) {
      if (t.start_at && t.end_at && covers(t, day)) entries.push({ kind: "task", item: t, start: startOn(t, day), end: endOn(t, day) });
      else if (!t.start_at && t.due_date === day && t.due_time) {
        const s = D.minutes(t.due_time);
        entries.push({ kind: "task", item: t, start: s, end: Math.min(s + 30, 1440), point: true });
      }
    }
    return layoutDay(entries);
  }

  function allDayFor(day) {
    const events = app.events.filter((e) => e.all_day && covers(e, day));
    const tasks = app.todos.filter((t) => t.due_date === day && !t.due_time && !t.start_at && t.parent_id == null).sort((a, b) => a.done - b.done || b.priority - a.priority);
    return { events, tasks };
  }

  // ----- pointer gestures -----

  function minsAt(clientY) {
    const r = cols.getBoundingClientRect();
    return Math.max(0, Math.min(24 * 60, ((clientY - r.top) / HOUR) * 60));
  }

  function dayAt(clientX) {
    const colEls = [...cols.querySelectorAll(".col")];
    for (let i = 0; i < colEls.length; i++) {
      const r = colEls[i].getBoundingClientRect();
      if (clientX >= r.left && clientX < r.right) return i;
    }
    return clientX < colEls[0].getBoundingClientRect().left ? 0 : colEls.length - 1;
  }

  function downEmpty(e, i) {
    if (e.button !== 0 || e.target.closest(".item")) return;
    const m = snap(minsAt(e.clientY) - 7);
    op = { mode: "create", day: i, anchor: m, start: m, end: m + 30, moved: false };
    listen();
  }

  function downItem(e, entry, i, mode = "move") {
    if (e.button !== 0 || e.target.closest(".check")) return;
    e.stopPropagation();
    const m = minsAt(e.clientY);
    op = { mode, kind: entry.kind, item: entry.item, point: entry.point, day: i, fromDay: i, start: entry.start, end: entry.end, origStart: entry.start, origEnd: entry.end, grab: m - entry.start, moved: false, x: e.clientX, y: e.clientY };
    listen();
  }

  function listen() {
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up, { once: true });
  }

  function move(e) {
    if (!op) return;
    const m = minsAt(e.clientY);
    if (op.mode === "create") {
      const s = snap(m);
      op.start = Math.min(op.anchor, s);
      op.end = Math.max(op.anchor + 15, s);
      op.moved = true;
    } else if (op.mode === "move") {
      if (!op.moved && Math.hypot(e.clientX - op.x, e.clientY - op.y) < 4) return;
      op.moved = true;
      const len = op.origEnd - op.origStart;
      const s = Math.max(0, Math.min(24 * 60 - len, snap(m - op.grab)));
      op.start = s;
      op.end = s + len;
      op.day = dayAt(e.clientX);
    } else if (op.mode === "resize") {
      op.moved = true;
      op.end = Math.max(op.start + 15, Math.min(24 * 60, snap(m)));
    }
  }

  function up() {
    window.removeEventListener("pointermove", move);
    const g = op;
    op = null;
    if (!g) return;
    if (g.mode === "create") {
      const end = g.moved ? g.end : g.start + 60;
      const day = days[g.day];
      app.eventEditor = { event: { title: "", all_day: 0, start_at: D.join(day, D.hhmm(g.start)), end_at: D.join(day, D.hhmm(Math.min(end, 24 * 60 - 1))) } };
      return;
    }
    if (!g.moved) {
      if (g.kind === "event") app.eventEditor = { event: g.item };
      else app.detail = g.item.id;
      return;
    }
    const day = days[g.day];
    const start = D.join(day, D.hhmm(g.start));
    const end = D.join(g.end >= 24 * 60 ? D.addDays(day, 1) : day, D.hhmm(g.end >= 24 * 60 ? 0 : g.end));
    if (g.kind === "event") {
      updateEvent(g.item.id, { start_at: start, end_at: end });
    } else if (g.point && g.mode === "move") {
      updateTodo(g.item.id, { due_date: day, due_time: D.hhmm(g.start) });
    } else {
      const fields = { start_at: start, end_at: end };
      if (g.item.due_date && g.fromDay !== g.day) fields.due_date = day;
      if (g.point) fields.due_date = day;
      updateTodo(g.item.id, fields);
    }
  }

  // ----- tasks dropped in from the tray -----

  function dragOverCol(e, i) {
    if (dnd.kind !== "task") return;
    e.preventDefault();
    dropHint = { day: i, start: snap(minsAt(e.clientY) - 15) };
  }

  function dropCol(e, i) {
    if (dnd.kind !== "task") return;
    e.preventDefault();
    const day = days[i];
    const s = Math.max(0, snap(minsAt(e.clientY) - 15));
    const t = app.todos.find((x) => x.id === dnd.id);
    const len = t?.start_at && t?.end_at ? D.minutes(D.timePart(t.end_at)) - D.minutes(D.timePart(t.start_at)) : 60;
    updateTodo(dnd.id, { due_date: day, start_at: D.join(day, D.hhmm(s)), end_at: D.join(day, D.hhmm(Math.min(s + Math.max(len, 15), 24 * 60 - 1))) });
    dropHint = null;
    endDrag();
  }

  function dropAllDay(e, day) {
    if (dnd.kind !== "task") return;
    e.preventDefault();
    updateTodo(dnd.id, { due_date: day, due_time: null, start_at: null, end_at: null });
    endDrag();
  }

  const nowMins = $derived(now.getHours() * 60 + now.getMinutes());
  const hours = Array.from({ length: 24 }, (_, h) => h);
  const label = (h) => (h === 0 ? "" : h < 12 ? `${h} AM` : h === 12 ? "12 PM" : `${h - 12} PM`);
</script>

<div class="tg" style:--days={days.length}>
  <div class="head">
    <div class="gutter"></div>
    {#each days as d}
      <button type="button" class="dayhead" class:today={d === today} onclick={() => onpickday?.(d)}>
        <span class="wd">{D.DAY_SHORT[D.weekday(d)]}</span>
        <span class="num">{Number(d.slice(8))}</span>
      </button>
    {/each}
  </div>
  <div class="allday">
    <div class="gutter small">all-day</div>
    {#each days as d}
      {@const ad = allDayFor(d)}
      <div class="adcell" ondragover={(e) => dnd.kind === "task" && e.preventDefault()} ondrop={(e) => dropAllDay(e, d)} role="list">
        {#each ad.events as ev (ev.id)}
          <button type="button" class="chip-ev" style:--c={eventColor(ev)} onclick={() => (app.eventEditor = { event: ev })}>{ev.countdown ? "⏳ " : ""}{ev.title}</button>
        {/each}
        {#each ad.tasks as t (t.id)}
          <div class="chip-task" class:done={t.done} draggable="true" ondragstart={(e) => startDrag(e, "task", t.id)} ondragend={endDrag} role="listitem">
            <TaskCheck done={!!t.done} priority={t.priority} size={14} onclick={() => toggleTodo(t.id)} />
            <button type="button" class="tt" onclick={() => (app.detail = t.id)}>{t.title}</button>
          </div>
        {/each}
        {#if habits.length && d <= today}
          <div class="habits">
            {#each habits as h (h.id)}
              <button type="button" class="hb" class:on={isChecked(h.id, d)} title="{h.name}{isChecked(h.id, d) ? ' ✓' : ''}" onclick={() => toggleHabit(h.id, d)}>{h.emoji || h.name[0]}</button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="scroll" bind:this={scroller}>
    <div class="body">
      <div class="gutter hours">
        {#each hours as h}<div class="hl" style:height="{HOUR}px"><span>{label(h)}</span></div>{/each}
      </div>
      <div class="cols" bind:this={cols}>
        {#each days as d, i}
          {@const items = timedFor(d)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="col" class:today={d === today} onpointerdown={(e) => downEmpty(e, i)} ondragover={(e) => dragOverCol(e, i)} ondragleave={() => (dropHint = null)} ondrop={(e) => dropCol(e, i)}>
            {#each hours as h}<div class="slot" style:height="{HOUR}px"></div>{/each}
            {#each items as en (en.kind + en.item.id)}
              {@const hidden = op?.moved && op.item?.id === en.item.id && op.kind === en.kind}
              {@const s = en.item.start_at ? D.timePart(en.item.start_at) : en.item.due_time}
              {@const eTime = en.item.end_at ? D.timePart(en.item.end_at) : D.hhmm(en.end)}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="item {en.kind}"
                class:ghosted={hidden}
                class:done={en.kind === "task" && en.item.done}
                class:short={en.height < 34}
                style:top="{en.top}px"
                style:height="{en.height - 2}px"
                style:left="calc({(en.col / en.cols) * 100}% + 2px)"
                style:width="calc({100 / en.cols}% - 4px)"
                style:--c={en.kind === "event" ? eventColor(en.item) : taskColor(en.item)}
                onpointerdown={(e) => downItem(e, en, i)}
              >
                {#if en.kind === "task"}
                  <span class="tcheck"><TaskCheck done={!!en.item.done} priority={en.item.priority} size={14} onclick={() => toggleTodo(en.item.id)} /></span>
                {/if}
                <span class="it">{en.item.title}{#if en.item.recurrence}<Repeat size={10} />{/if}</span>
                {#if !en.point || en.kind === "event"}<span class="time">{fmtRange(s, eTime)}</span>{:else}<span class="time">{D.fmtTime(s)}</span>{/if}
                {#if !en.point}<span class="resize" onpointerdown={(e) => downItem(e, en, i, "resize")}></span>{/if}
              </div>
            {/each}
            {#if op?.moved && op.day === i && op.mode !== "create"}
              <div class="item ghost {op.kind}" style:top="{(op.start / 60) * HOUR}px" style:height="{((op.end - op.start) / 60) * HOUR - 2}px" style:--c={op.kind === "event" ? eventColor(op.item) : taskColor(op.item)}>
                <span class="it">{op.item.title}</span><span class="time">{fmtRange(D.hhmm(op.start), D.hhmm(op.end))}</span>
              </div>
            {/if}
            {#if op?.mode === "create" && op.day === i}
              <div class="item ghost event new" style:top="{(op.start / 60) * HOUR}px" style:height="{(((op.moved ? op.end : op.start + 60) - op.start) / 60) * HOUR - 2}px" style:--c="#7aa7ff">
                <span class="it">New event</span><span class="time">{fmtRange(D.hhmm(op.start), D.hhmm(op.moved ? op.end : op.start + 60))}</span>
              </div>
            {/if}
            {#if dropHint?.day === i}
              <div class="drophint" style:top="{(dropHint.start / 60) * HOUR}px" style:height="{HOUR - 2}px">{D.fmtTime(D.hhmm(Math.max(0, dropHint.start)))}</div>
            {/if}
            {#if d === today}<div class="now" style:top="{(nowMins / 60) * HOUR}px"></div>{/if}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .tg { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .head, .allday, .body { display: grid; grid-template-columns: 56px minmax(0, 1fr); }
  /* The same gutter on every band keeps the day columns lined up with the scrolling hours. */
  .head, .allday, .scroll { scrollbar-gutter: stable; }
  .head { grid-template-columns: 56px repeat(var(--days), minmax(0, 1fr)); border-bottom: 1px solid var(--border); overflow: hidden; }
  .allday { grid-template-columns: 56px repeat(var(--days), minmax(0, 1fr)); border-bottom: 1px solid var(--border-strong); max-height: 30vh; overflow-y: auto; }
  .gutter { font-size: 11px; color: var(--muted); text-align: right; padding-right: 8px; }
  .gutter.small { padding-top: 8px; }
  .dayhead { display: flex; align-items: baseline; justify-content: center; gap: 6px; border: 0; background: none; padding: 8px 0; color: var(--text-2); border-left: 1px solid transparent; }
  .dayhead:hover .num { background: var(--hover); }
  .wd { font-size: 12px; font-weight: 650; text-transform: uppercase; letter-spacing: .05em; color: var(--muted); }
  .num { font-size: 22px; font-weight: 800; width: 38px; height: 38px; display: grid; place-items: center; border-radius: 50%; font-variant-numeric: tabular-nums; }
  .dayhead.today .num { background: var(--accent); color: #fff; }
  .dayhead.today .wd { color: var(--accent); }
  .adcell { min-width: 0; min-height: 34px; padding: 3px; border-left: 1px solid var(--border); display: flex; flex-direction: column; gap: 2px; }
  .chip-ev { text-align: left; border: 0; border-radius: 4px; padding: 2px 6px; font-size: 12px; font-weight: 600; background: color-mix(in srgb, var(--c) 30%, transparent); color: color-mix(in srgb, var(--c) 40%, white); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .chip-task { display: flex; align-items: center; gap: 5px; padding: 1px 4px; border-radius: 4px; cursor: grab; }
  .chip-task:hover { background: var(--hover); }
  .chip-task .tt { border: 0; background: none; padding: 0; font-size: 12px; color: var(--text-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-align: left; flex: 1; min-width: 0; }
  .chip-task.done .tt { text-decoration: line-through; color: var(--muted); }
  .habits { display: flex; flex-wrap: wrap; gap: 2px; margin-top: 2px; }
  .hb { width: 20px; height: 20px; border-radius: 50%; border: 1px dashed var(--border-strong); background: none; font-size: 11px; padding: 0; opacity: 0.45; filter: grayscale(1); }
  .hb.on { opacity: 1; filter: none; border-style: solid; border-color: var(--today); background: rgba(79, 191, 106, 0.15); }

  .scroll { flex: 1; overflow-y: auto; min-height: 0; }
  .body { position: relative; }
  .hours .hl { position: relative; }
  .hours .hl span { position: absolute; top: -7px; right: 8px; font-size: 11px; color: var(--muted); white-space: nowrap; }
  .cols { display: grid; grid-template-columns: repeat(var(--days), minmax(0, 1fr)); position: relative; }
  .col { position: relative; border-left: 1px solid var(--border); cursor: crosshair; }
  .col.today { background: rgba(229, 72, 59, 0.035); }
  .slot { border-bottom: 1px solid #2a2926; }
  .slot:nth-child(odd) { border-bottom-color: #2f2e2a; }
  .item {
    position: absolute; z-index: 2; overflow: hidden; border-radius: 6px; padding: 3px 6px 2px 8px; cursor: grab; font-size: 12px; line-height: 1.3;
    background: color-mix(in srgb, var(--c) 28%, #1b1a18); border-left: 3px solid var(--c); color: color-mix(in srgb, var(--c) 30%, white);
    display: flex; flex-direction: column; user-select: none; touch-action: none; box-shadow: 0 0 0 1px var(--bg);
  }
  .item.task { background: color-mix(in srgb, var(--c) 12%, #1b1a18); border: 1px dashed color-mix(in srgb, var(--c) 60%, transparent); border-left: 3px solid var(--c); padding-left: 24px; }
  .item.task.done { opacity: 0.55; }
  .item.task.done .it { text-decoration: line-through; }
  .item.short { flex-direction: row; gap: 6px; align-items: center; padding-top: 0; padding-bottom: 0; }
  .item.short .time { font-size: 11px; }
  .tcheck { position: absolute; left: 5px; top: 4px; }
  .short .tcheck { top: 50%; transform: translateY(-50%); }
  .it { font-weight: 650; display: flex; align-items: center; gap: 4px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--text); }
  .time { opacity: 0.8; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .resize { position: absolute; left: 0; right: 0; bottom: 0; height: 7px; cursor: ns-resize; }
  .ghosted { opacity: 0.3; }
  .ghost { z-index: 5; opacity: 0.92; box-shadow: var(--shadow); pointer-events: none; }
  .ghost.new { background: rgba(122, 167, 255, 0.35); }
  .drophint { position: absolute; left: 2px; right: 2px; z-index: 4; border: 2px dashed var(--link); border-radius: 6px; background: rgba(122, 167, 255, 0.12); font-size: 11px; padding: 2px 6px; color: var(--link); pointer-events: none; }
  .now { position: absolute; left: -1px; right: 0; height: 2px; background: var(--accent); z-index: 6; pointer-events: none; }
  .now::before { content: ""; position: absolute; left: -5px; top: -4px; width: 10px; height: 10px; border-radius: 50%; background: var(--accent); }
</style>
