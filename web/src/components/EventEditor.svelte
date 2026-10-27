<script>
  import { untrack } from "svelte";
  // Create or edit a calendar event.
  import { Trash2, MapPin, Hourglass, AlignLeft, Hash } from "@lucide/svelte";
  import Modal from "./Modal.svelte";
  import ColorPicker from "./ColorPicker.svelte";
  import { app, createEvent, deleteEvent, updateEvent } from "../lib/store.svelte.js";
  import { autosize, focusOnMount } from "../lib/place.js";
  import * as D from "../lib/dates.js";

  let { event } = $props();
  // A form starts from the event it was opened with (the dialog is re-created for another one).
  const initial = untrack(() => $state.snapshot(event));
  const isNew = !initial.id;
  const allDay0 = !!initial.all_day;

  let title = $state(initial.title ?? "");
  let allDay = $state(allDay0);
  let startDate = $state(D.datePart(initial.start_at));
  let endDate = $state(D.datePart(initial.end_at || initial.start_at));
  let startTime = $state(allDay0 ? "09:00" : D.timePart(initial.start_at));
  let endTime = $state(allDay0 ? "10:00" : D.timePart(initial.end_at || initial.start_at));
  let color = $state(initial.color ?? "");
  let classId = $state(initial.class_id ?? "");
  let location = $state(initial.location ?? "");
  let notes = $state(initial.notes ?? "");
  let countdown = $state(!!initial.countdown);
  let error = $state("");

  const close = () => (app.eventEditor = null);

  function changeStartDate(v) {
    const shift = D.diffDays(v, startDate);
    startDate = v;
    endDate = D.addDays(endDate, shift);
  }

  function changeStartTime(v) {
    const len = D.minutes(endTime) - D.minutes(startTime);
    startTime = v;
    if (startDate === endDate) endTime = D.hhmm(Math.min(D.minutes(v) + Math.max(len, 15), 24 * 60 - 1));
  }

  async function save(e) {
    e?.preventDefault();
    if (!title.trim()) {
      error = "Give it a name";
      return;
    }
    const fields = {
      title: title.trim(),
      all_day: allDay,
      start_at: allDay ? startDate : D.join(startDate, startTime),
      end_at: allDay ? (endDate < startDate ? startDate : endDate) : D.join(endDate < startDate ? startDate : endDate, endTime),
      color,
      class_id: classId === "" ? null : Number(classId),
      location,
      notes,
      countdown,
    };
    if (!allDay && fields.end_at <= fields.start_at) {
      error = "The end has to be after the start";
      return;
    }
    const ok = isNew ? await createEvent(fields) : await updateEvent(event.id, fields);
    if (ok) close();
  }

  function remove() {
    deleteEvent(event.id);
    close();
  }
</script>

<Modal onclose={close} width={480} top="10vh" label={isNew ? "New event" : "Edit event"}>
  <form class="ee" onsubmit={save}>
    <input class="title" placeholder="Event name" bind:value={title} use:focusOnMount={isNew} />
    <label class="toggle"><input type="checkbox" bind:checked={allDay} /> All day</label>
    <div class="when">
      <span class="to">Starts</span>
      <input type="date" class="input" value={startDate} onchange={(e) => changeStartDate(e.currentTarget.value)} aria-label="Start date" />
      {#if !allDay}<input type="time" class="input t" value={startTime} onchange={(e) => changeStartTime(e.currentTarget.value)} aria-label="Start time" />{/if}
      <span class="to">Ends</span>
      <input type="date" class="input" bind:value={endDate} min={startDate} aria-label="End date" />
      {#if !allDay}<input type="time" class="input t" bind:value={endTime} aria-label="End time" />{/if}
    </div>
    <div class="row"><MapPin size={16} /><input class="input" placeholder="Location" bind:value={location} /></div>
    <div class="row"><Hash size={16} />
      <select class="select" bind:value={classId} aria-label="Class">
        <option value="">No class</option>
        {#each app.classes.filter((c) => !c.archived) as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
      </select>
    </div>
    <div class="row top"><AlignLeft size={16} /><textarea class="textarea" rows="2" placeholder="Notes" bind:value={notes} use:autosize></textarea></div>
    <label class="toggle"><input type="checkbox" bind:checked={countdown} /><Hourglass size={15} /> Count down to this (shows on Today and the calendar)</label>
    <div class="colors"><span class="muted">Colour</span><ColorPicker bind:value={color} allowNone /></div>
    {#if error}<p class="err">{error}</p>{/if}
    <footer>
      {#if !isNew}<button type="button" class="btn danger" onclick={remove}><Trash2 size={15} /> Delete</button>{/if}
      <span class="spacer"></span>
      <button type="button" class="btn ghost" onclick={close}>Cancel</button>
      <button class="btn primary">{isNew ? "Add event" : "Save"}</button>
    </footer>
  </form>
</Modal>

<style>
  .ee { padding: 18px 20px; display: flex; flex-direction: column; gap: 12px; }
  .title { border: 0; outline: none; background: none; font-size: 22px; font-weight: 800; letter-spacing: -0.02em; padding: 0; }
  .title::placeholder { color: var(--faint); }
  .when { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 6px 8px; }
  .when .input { width: auto; }
  .when .t { width: 110px; }
  .to { color: var(--muted); font-size: 13px; width: 44px; }
  .row { display: flex; align-items: center; gap: 10px; color: var(--muted); }
  .row.top { align-items: flex-start; }
  .row.top :global(svg) { margin-top: 9px; }
  .toggle { display: flex; align-items: center; gap: 8px; font-size: 13.5px; color: var(--text-2); }
  .colors { display: flex; flex-direction: column; gap: 6px; font-size: 13px; }
  .err { color: var(--p1); margin: 0; font-size: 13px; }
  footer { display: flex; gap: 8px; align-items: center; margin-top: 4px; }
</style>
