<script>
  import { untrack } from "svelte";
  // Todoist-style scheduler: type a date in words, pick a shortcut or a day, add a time and repeat.
  import { CalendarDays, Sun, Sofa, CalendarArrowUp, Ban, ChevronLeft, ChevronRight, Clock, Repeat, X } from "@lucide/svelte";
  import Popover from "./Popover.svelte";
  import * as D from "../lib/dates.js";
  import { describeRule, parseWhen } from "../lib/nlp.js";
  import { focusOnMount } from "../lib/place.js";

  let { anchor, value = {}, onchange, onclose } = $props();
  // A form starts from the value it was opened with (the dialog is re-created for another one).
  const initial = untrack(() => $state.snapshot(value));

  const today = D.today();
  let text = $state("");
  let month = $state(D.startOfMonth(initial.due_date || today));
  let showTime = $state(!!initial.due_time);
  let time = $state(initial.due_time || "");
  let showRepeat = $state(false);

  const parsed = $derived(text.trim() ? parseWhen(text, today) : null);
  const weeks = $derived.by(() => {
    const start = D.startOfWeek(month);
    return Array.from({ length: 6 }, (_, w) => D.range(D.addDays(start, w * 7), 7));
  });

  function set(fields) {
    onchange({ ...value, ...fields });
  }

  function pick(date) {
    set({ due_date: date, due_time: date ? time || null : null, ...(date ? {} : { recurrence: null }) });
    onclose();
  }

  function applyText(e) {
    if (e.key === "Enter" && parsed?.matched) {
      e.preventDefault();
      const f = { due_date: parsed.due_date ?? value.due_date ?? today };
      if (parsed.due_time) f.due_time = parsed.due_time;
      if (parsed.recurrence) f.recurrence = parsed.recurrence;
      set(f);
      onclose();
    }
  }

  function setTime(t) {
    time = t;
    set({ due_time: t || null, due_date: value.due_date || today });
  }

  const presets = $derived.by(() => {
    const base = value.due_date || today;
    const wd = D.weekday(base);
    const dom = Number(base.slice(8));
    return [
      ["Every day", { freq: "day", interval: 1 }],
      ["Every weekday (Mon – Fri)", { freq: "week", interval: 1, days: [0, 1, 2, 3, 4] }],
      [`Every week on ${D.DAY_NAMES[wd]}`, { freq: "week", interval: 1, days: [wd] }],
      [`Every other week on ${D.DAY_NAMES[wd]}`, { freq: "week", interval: 2, days: [wd] }],
      [`Every month on the ${dom}${dom === 1 ? "st" : dom === 2 ? "nd" : dom === 3 ? "rd" : "th"}`, { freq: "month", interval: 1, monthday: dom }],
      ["Every year", { freq: "year", interval: 1 }],
    ];
  });

  const quick = [
    { label: "Today", icon: CalendarDays, color: "var(--today)", date: today },
    { label: "Tomorrow", icon: Sun, color: "var(--tomorrow)", date: D.addDays(today, 1) },
    { label: "This weekend", icon: Sofa, color: "var(--p3)", date: D.addDays(D.startOfWeek(today), 5) < today ? today : D.addDays(D.startOfWeek(today), 5) },
    { label: "Next week", icon: CalendarArrowUp, color: "var(--week)", date: D.addDays(D.startOfWeek(today), 7) },
  ];
</script>

<Popover {anchor} {onclose} width={272}>
  <div class="dp">
    <div class="head">
      <input class="type" placeholder="Type a date…  (fri 3pm, every mon)" bind:value={text} onkeydown={applyText} use:focusOnMount />
      {#if parsed?.matched}
        <div class="preview">
          {parsed.due_date ? D.long(parsed.due_date) : ""}{parsed.due_time ? ` · ${D.fmtTime(parsed.due_time)}` : ""}
          {#if parsed.recurrence}<span class="rec"><Repeat size={12} /> {describeRule(parsed.recurrence)}</span>{/if}
          <kbd>↵</kbd>
        </div>
      {:else if value.due_date}
        <div class="preview current">
          {D.long(value.due_date)}{value.due_time ? ` · ${D.fmtTime(value.due_time)}` : ""}
          {#if value.recurrence}<span class="rec"><Repeat size={12} /> {describeRule(value.recurrence)}</span>{/if}
        </div>
      {/if}
    </div>
    <div class="quick">
      {#each quick as q}
        <button type="button" class="menu-item" onclick={() => pick(q.date)}>
          <q.icon size={16} color={q.color} strokeWidth={1.75} /> {q.label}<span class="hint">{D.DAY_SHORT[D.weekday(q.date)]}{q.label === "Next week" || q.label === "This weekend" ? ` ${D.MONTH_SHORT[D.parse(q.date).getMonth()]} ${Number(q.date.slice(8))}` : ""}</span>
        </button>
      {/each}
      {#if value.due_date}
        <button type="button" class="menu-item" onclick={() => pick(null)}><Ban size={16} strokeWidth={1.75} /> No date</button>
      {/if}
    </div>

    <div class="cal">
      <div class="calhead">
        <b>{D.monthTitle(month)}</b>
        <span class="spacer"></span>
        <button type="button" class="icon-btn sm" aria-label="Previous month" onclick={() => (month = D.addMonths(month, -1))}><ChevronLeft size={16} /></button>
        <button type="button" class="icon-btn sm" aria-label="This month" onclick={() => (month = D.startOfMonth(today))}><span class="ring"></span></button>
        <button type="button" class="icon-btn sm" aria-label="Next month" onclick={() => (month = D.addMonths(month, 1))}><ChevronRight size={16} /></button>
      </div>
      <div class="grid">
        {#each D.DAY_SHORT as d}<span class="wd">{d[0]}</span>{/each}
        {#each weeks.flat() as day}
          <button
            type="button"
            class="day"
            class:other={day.slice(0, 7) !== month.slice(0, 7)}
            class:past={day < today}
            class:today={day === today}
            class:sel={day === value.due_date}
            onclick={() => pick(day)}
          >{Number(day.slice(8))}</button>
        {/each}
      </div>
    </div>

    <div class="foot">
      {#if showTime}
        <label class="time"><Clock size={15} /><input type="time" class="input sm" value={time} onchange={(e) => setTime(e.currentTarget.value)} /></label>
        <button type="button" class="icon-btn sm" aria-label="Remove time" onclick={() => { showTime = false; setTime(""); }}><X size={14} /></button>
      {:else}
        <button type="button" class="btn ghost sm" onclick={() => (showTime = true)}><Clock size={15} /> Time</button>
      {/if}
      <span class="spacer"></span>
      <button type="button" class="btn ghost sm" class:on={!!value.recurrence} onclick={() => (showRepeat = !showRepeat)}><Repeat size={15} /> Repeat</button>
    </div>
    {#if showRepeat}
      <div class="repeat">
        {#each presets as [label, rule]}
          <button type="button" class="menu-item" onclick={() => { set({ recurrence: rule, due_date: value.due_date || today }); onclose(); }}>{label}</button>
        {/each}
        {#if value.recurrence}
          <button type="button" class="menu-item danger" onclick={() => { set({ recurrence: null }); showRepeat = false; }}>Don't repeat</button>
        {/if}
        <p class="tip">Or type it: “every 3 days”, “every tue, thu”.</p>
      </div>
    {/if}
  </div>
</Popover>

<style>
  .dp { display: flex; flex-direction: column; }
  .head { padding: 8px; border-bottom: 1px solid var(--border); }
  .type { width: 100%; border: 0; background: transparent; outline: none; font-size: 13.5px; padding: 4px; }
  .type::placeholder { color: var(--faint); }
  .preview { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 6px 4px 0; font-size: 12.5px; font-weight: 600; color: var(--today); }
  .preview.current { color: var(--text-2); font-weight: 500; }
  .preview kbd { margin-left: auto; }
  .rec { display: inline-flex; align-items: center; gap: 3px; color: var(--muted); font-weight: 500; }
  .quick { padding: 4px; border-bottom: 1px solid var(--border); }
  .cal { padding: 8px 10px; border-bottom: 1px solid var(--border); }
  .calhead { display: flex; align-items: center; margin-bottom: 4px; font-size: 13px; }
  .icon-btn.sm { width: 24px; height: 24px; }
  .ring { width: 8px; height: 8px; border: 2px solid currentColor; border-radius: 50%; }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px; text-align: center; }
  .wd { font-size: 11px; color: var(--muted); padding: 2px 0; }
  .day { height: 30px; border: 0; background: none; border-radius: 50%; font-size: 12.5px; padding: 0; color: var(--text); font-variant-numeric: tabular-nums; }
  .day:hover { background: var(--active); }
  .day.other { color: var(--faint); }
  .day.past { color: var(--faint); }
  .day.today { color: var(--accent); font-weight: 750; }
  .day.sel { background: var(--accent); color: #fff; font-weight: 700; }
  .foot { display: flex; align-items: center; gap: 4px; padding: 6px 8px; }
  .time { display: flex; align-items: center; gap: 6px; color: var(--muted); }
  .input.sm { height: 28px; width: 120px; }
  .btn.on { color: var(--accent); }
  .repeat { padding: 4px; border-top: 1px solid var(--border); }
  .tip { margin: 4px 9px 6px; font-size: 12px; color: var(--muted); }
</style>
