<script>
  // The add-task box. Words like "fri 3pm p1 #English every week" are picked out and
  // highlighted as you type; the chips below show (and override) what was understood.
  import { CalendarDays, Flag, Hash, Inbox, Repeat, Clock, X } from "@lucide/svelte";
  import DatePicker from "./DatePicker.svelte";
  import { app, createTodo } from "../lib/store.svelte.js";
  import { parse, describeRule } from "../lib/nlp.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { placeItems, placeLabel, priorityItems } from "../lib/pickers.js";
  import { priorityColor, hex } from "../lib/colors.js";
  import { autosize } from "../lib/place.js";
  import * as D from "../lib/dates.js";

  let { defaults = {}, oncancel, oncreated, keepOpen = true, autofocus = true, variant = "inline", submitLabel = "Add task" } = $props();

  let text = $state("");
  let description = $state("");
  let overrides = $state({});
  let dateAnchor = $state(null);
  let input = $state();
  let mirror = $state();
  let busy = $state(false);

  const parsed = $derived(parse(text, { classes: app.classes, sections: app.sections, today: D.today() }));
  // Values chosen with the chips win over the words; defaults (the view you are in) come last.
  const fields = $derived.by(() => {
    const f = { ...defaults };
    for (const k of ["due_date", "due_time", "priority", "class_id", "section_id", "recurrence"]) {
      if (parsed[k] !== undefined) f[k] = parsed[k];
    }
    if (parsed.class_id !== undefined && parsed.section_id === undefined) f.section_id = null;
    return { ...f, ...overrides, duration: parsed.duration };
  });

  const segments = $derived.by(() => {
    const out = [];
    let at = 0;
    for (const t of parsed.tokens) {
      if (t.start > at) out.push({ text: text.slice(at, t.start) });
      out.push({ text: text.slice(t.start, t.end), kind: t.kind });
      at = t.end;
    }
    out.push({ text: text.slice(at) + " " });
    return out;
  });

  // Keep the highlights lined up when a long title scrolls sideways.
  const sync = () => mirror && input && (mirror.scrollLeft = input.scrollLeft);
  $effect(() => {
    text;
    requestAnimationFrame(sync);
  });

  $effect(() => {
    if (autofocus) requestAnimationFrame(() => input?.focus());
  });

  async function submit(e) {
    e?.preventDefault();
    const title = parsed.title.trim();
    if (!title || busy) return;
    busy = true;
    const f = fields;
    const body = { title, description, priority: f.priority ?? 0 };
    for (const k of ["due_date", "due_time", "class_id", "section_id", "parent_id", "recurrence"]) if (f[k] != null) body[k] = f[k];
    // "3pm for 1h" also books the time on the calendar.
    if (f.due_date && f.due_time && f.duration) {
      const start = D.minutes(f.due_time);
      body.start_at = D.join(f.due_date, f.due_time);
      body.end_at = D.join(f.due_date, D.hhmm(Math.min(start + f.duration, 24 * 60 - 1)));
    }
    const t = await createTodo(body);
    busy = false;
    if (!t) return;
    oncreated?.(t);
    text = "";
    description = "";
    overrides = {};
    if (!keepOpen) oncancel?.();
    else input?.focus();
  }

  function key(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      oncancel?.();
    }
  }

  const dateText = $derived(fields.due_date ? `${D.label(fields.due_date)}${fields.due_time ? ` ${D.fmtTime(fields.due_time)}` : ""}` : "Date");
  const cls = $derived(app.classes.find((c) => c.id === fields.class_id));
</script>

<form class="composer {variant}" onsubmit={submit}>
  <div class="fieldwrap">
    <div class="mirror" aria-hidden="true" bind:this={mirror}>{#each segments as s}{#if s.kind}<mark class={s.kind}>{s.text}</mark>{:else}{s.text}{/if}{/each}</div>
    <input
      bind:this={input}
      bind:value={text}
      class="title"
      placeholder={variant === "modal" ? "e.g. Lab report fri 3pm p1 #Chemistry" : "Task name"}
      onkeydown={key}
      onkeyup={sync}
      onscroll={sync}
      aria-label="Task name"
      spellcheck="false"
      autocomplete="off"
    />
  </div>
  <textarea class="desc" rows="1" placeholder="Description" bind:value={description} use:autosize onkeydown={(e) => e.key === "Escape" && (e.stopPropagation(), oncancel?.())}></textarea>

  <div class="chips">
    <button type="button" class="chip" class:set={fields.due_date} style:color={fields.due_date ? `var(--${D.tone(fields.due_date)})` : undefined} onclick={(e) => (dateAnchor = e.currentTarget)}>
      <CalendarDays size={14} /> {dateText}
      {#if fields.recurrence}<Repeat size={12} />{/if}
      {#if fields.duration}<Clock size={12} /> {fields.duration}m{/if}
    </button>
    <button type="button" class="chip" class:set={fields.priority} onclick={(e) => openMenu(e, priorityItems(fields.priority ?? 0, (p) => (overrides.priority = p)))}>
      <Flag size={14} color={fields.priority ? priorityColor(fields.priority) : undefined} fill={fields.priority ? priorityColor(fields.priority) : "none"} />
      {fields.priority ? `P${4 - fields.priority}` : "Priority"}
    </button>
    {#if fields.recurrence}
      <span class="chip set"><Repeat size={13} /> {describeRule(fields.recurrence)}
        <button type="button" class="x" aria-label="Don't repeat" onclick={() => (overrides.recurrence = null)}><X size={12} /></button>
      </span>
    {/if}
  </div>

  <footer>
    <button type="button" class="chip place" onclick={(e) => openMenu(e, placeItems(fields, (p) => Object.assign(overrides, p)))}>
      {#if cls}<Hash size={14} color={hex(cls.color)} />{:else}<Inbox size={14} />{/if}
      {placeLabel(fields.class_id, fields.section_id)}
    </button>
    <span class="spacer"></span>
    {#if oncancel}<button type="button" class="btn ghost sm" onclick={oncancel}>Cancel</button>{/if}
    <button class="btn primary sm" disabled={!parsed.title.trim() || busy}>{submitLabel}</button>
  </footer>
</form>

{#if dateAnchor}
  <DatePicker anchor={dateAnchor} value={fields} onchange={(v) => Object.assign(overrides, { due_date: v.due_date, due_time: v.due_time, recurrence: v.recurrence })} onclose={() => { dateAnchor = null; input?.focus(); }} />
{/if}

<style>
  .composer { border: 1px solid var(--border-strong); border-radius: var(--radius); background: var(--surface); }
  .composer.inline { margin: 6px 0 10px; }
  .composer.modal { border: 0; background: transparent; }
  .composer:focus-within { border-color: #5a5650; }
  .fieldwrap { position: relative; margin: 10px 12px 0; }
  .mirror, .title {
    font: 600 15px/22px var(--font); letter-spacing: normal; padding: 0; margin: 0; white-space: pre; overflow: hidden;
  }
  .mirror { position: absolute; inset: 0; color: transparent; pointer-events: none; }
  .title { position: relative; width: 100%; border: 0; outline: none; background: transparent; color: var(--text); }
  .title::placeholder { color: var(--faint); font-weight: 500; }
  .modal .title, .modal .mirror { font-size: 16px; line-height: 24px; }
  mark { color: transparent; border-radius: 4px; background: rgba(122, 167, 255, 0.22); box-shadow: 0 0 0 1px rgba(122, 167, 255, 0.22); }
  mark.priority { background: rgba(255, 107, 94, 0.22); box-shadow: 0 0 0 1px rgba(255, 107, 94, 0.22); }
  mark.class, mark.section { background: rgba(79, 191, 106, 0.22); box-shadow: 0 0 0 1px rgba(79, 191, 106, 0.22); }
  mark.recurrence { background: rgba(180, 140, 255, 0.24); box-shadow: 0 0 0 1px rgba(180, 140, 255, 0.24); }
  .desc { display: block; width: calc(100% - 24px); margin: 2px 12px 0; border: 0; outline: none; background: transparent; resize: none; font-size: 13px; color: var(--text-2); padding: 0; min-height: 20px; max-height: 200px; }
  .desc::placeholder { color: var(--faint); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; padding: 10px 12px; }
  .x { border: 0; background: none; color: var(--muted); padding: 0; display: grid; place-items: center; }
  footer { display: flex; align-items: center; gap: 8px; padding: 8px 10px 8px 12px; border-top: 1px solid var(--border); }
  .place { border: 0; padding: 0 8px; }
</style>
