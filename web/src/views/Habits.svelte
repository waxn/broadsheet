<script>
  // Habits: tick off each day, keep a streak, and see the last half year at a glance.
  import { Plus, Ellipsis, Pencil, Archive, Trash2, Flame } from "@lucide/svelte";
  import ViewHeader from "../components/ViewHeader.svelte";
  import EmojiPicker from "../components/EmojiPicker.svelte";
  import { app, createHabit, deleteHabit, isChecked, loadHabits, toggleHabit, updateHabit } from "../lib/store.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { hex, COLORS } from "../lib/colors.js";
  import { focusOnMount } from "../lib/place.js";
  import * as D from "../lib/dates.js";

  loadHabits();

  const today = $derived(D.today());
  const week = $derived(D.range(D.addDays(today, -6), 7));
  const WEEKS = 26;
  const heatStart = $derived(D.addDays(D.startOfWeek(today), -7 * (WEEKS - 1)));
  const heatDays = $derived(D.range(heatStart, WEEKS * 7));
  const active = $derived(app.habits.filter((h) => !h.archived));
  const archived = $derived(app.habits.filter((h) => h.archived));

  let name = $state("");
  let emoji = $state("✅");
  let emojiAnchor = $state(null);
  let editEmojiFor = $state(null);
  let renaming = $state(null);
  let renameValue = $state("");

  function streak(id) {
    let n = 0;
    let d = isChecked(id, today) ? today : D.addDays(today, -1);
    while (isChecked(id, d)) {
      n++;
      d = D.addDays(d, -1);
    }
    return n;
  }

  const total = (id) => app.checks.filter((c) => c.habit_id === id && c.day >= heatStart).length;

  async function add(e) {
    e.preventDefault();
    if (!name.trim()) return;
    const palette = Object.keys(COLORS);
    await createHabit({ name: name.trim(), emoji, color: palette[(app.habits.length * 5 + 6) % palette.length] });
    name = "";
  }

  function menu(e, h) {
    openMenu(e, [
      { label: "Rename", icon: Pencil, action: () => ((renaming = h.id), (renameValue = h.name)) },
      { label: "Change emoji", action: () => (editEmojiFor = { id: h.id, anchor: e.target }) },
      { heading: "Colour" },
      ...["green", "teal", "blue", "grape", "magenta", "red", "orange", "yellow"].map((c) => ({ label: COLORS[c][0], color: hex(c), icon: Flame, checked: h.color === c, action: () => updateHabit(h.id, { color: c }) })),
      { sep: true },
      { label: h.archived ? "Unarchive" : "Archive", icon: Archive, action: () => updateHabit(h.id, { archived: !h.archived }) },
      { label: "Delete", icon: Trash2, danger: true, action: () => confirm(`Delete “${h.name}” and its history?`) && deleteHabit(h.id) },
    ]);
  }

  function saveName(e, h) {
    e?.preventDefault();
    if (renameValue.trim()) updateHabit(h.id, { name: renameValue.trim() });
    renaming = null;
  }
</script>

<div class="wrap">
  <ViewHeader title="Habits" subtitle="Small things, every day. Tick them here or right on the calendar." />

  <form class="add" onsubmit={add}>
    <button type="button" class="emoji" aria-label="Pick an emoji" onclick={(e) => (emojiAnchor = e.currentTarget)}>{emoji}</button>
    <input class="input" placeholder="New habit, like “Read 20 minutes”" bind:value={name} />
    <button class="btn primary" disabled={!name.trim()}><Plus size={15} /> Add</button>
  </form>

  {#if active.length}
    <div class="weekhead">
      <span></span>
      {#each week as d}<span class:today={d === today}>{D.DAY_SHORT[D.weekday(d)][0]}<b>{Number(d.slice(8))}</b></span>{/each}
      <span></span>
    </div>
  {/if}

  {#each active as h (h.id)}
    {@const s = streak(h.id)}
    <section class="habit" style:--c={hex(h.color)}>
      <div class="row">
        <div class="name">
          <span class="he">{h.emoji || "•"}</span>
          {#if renaming === h.id}
            <form onsubmit={(e) => saveName(e, h)}><input class="input" bind:value={renameValue} use:focusOnMount={true} onblur={(e) => saveName(e, h)} /></form>
          {:else}
            <div>
              <div class="hn">{h.name}</div>
              <div class="meta">{#if s}<span class="streak"><Flame size={12} /> {s} day{s === 1 ? "" : "s"}</span>{/if}<span>{total(h.id)} in 6 months</span></div>
            </div>
          {/if}
        </div>
        {#each week as d}
          <button type="button" class="tick" class:on={isChecked(h.id, d)} class:today={d === today} aria-label="{h.name} on {D.long(d)}" aria-pressed={isChecked(h.id, d)} onclick={() => toggleHabit(h.id, d)}>
            {#if isChecked(h.id, d)}✓{/if}
          </button>
        {/each}
        <button type="button" class="icon-btn" aria-label="Habit options" onclick={(e) => menu(e, h)}><Ellipsis size={17} /></button>
      </div>
      <div class="heat" aria-label="Last six months">
        {#each heatDays as d}
          <button type="button" class="hd" class:on={isChecked(h.id, d)} class:future={d > today} disabled={d > today} title="{D.long(d)}{isChecked(h.id, d) ? ' ✓' : ''}" onclick={() => toggleHabit(h.id, d)} aria-label={D.long(d)}></button>
        {/each}
      </div>
    </section>
  {:else}
    <div class="empty">
      <div class="art">🔥</div>
      <p><b>Build a streak</b></p>
      <p class="muted">Add a habit above. Check it off here, or tap its emoji on any day in the calendar.</p>
    </div>
  {/each}

  {#if archived.length}
    <h2>Archived</h2>
    {#each archived as h (h.id)}
      <div class="arch"><span>{h.emoji} {h.name}</span><button type="button" class="icon-btn" aria-label="Options" onclick={(e) => menu(e, h)}><Ellipsis size={16} /></button></div>
    {/each}
  {/if}
</div>

{#if emojiAnchor}
  <EmojiPicker anchor={emojiAnchor} allowRemove={false} onpick={(e) => (emoji = e)} onclose={() => (emojiAnchor = null)} />
{/if}
{#if editEmojiFor}
  <EmojiPicker anchor={editEmojiFor.anchor} allowRemove={false} onpick={(e) => updateHabit(editEmojiFor.id, { emoji: e })} onclose={() => (editEmojiFor = null)} />
{/if}

<style>
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 56px 120px; }
  .add { display: flex; gap: 8px; margin-bottom: 22px; }
  .emoji { width: 40px; height: 34px; flex: none; border: 1px solid var(--border); background: var(--bg); border-radius: 6px; font-size: 18px; padding: 0; }
  .weekhead, .row { display: grid; grid-template-columns: 1fr repeat(7, 40px) 32px; align-items: center; gap: 6px; }
  .weekhead { font-size: 11px; color: var(--muted); text-align: center; font-weight: 650; margin-bottom: 6px; }
  .weekhead span { display: flex; flex-direction: column; }
  .weekhead b { font-size: 13px; color: var(--text-2); }
  .weekhead .today b { color: var(--accent); }
  .habit { padding: 14px 16px; border-radius: 14px; background: var(--surface); border: 1px solid var(--border); margin-bottom: 10px; }
  .name { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .he { font-size: 26px; width: 40px; height: 40px; display: grid; place-items: center; border-radius: 12px; background: color-mix(in srgb, var(--c) 18%, transparent); flex: none; }
  .hn { font-weight: 700; font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { display: flex; gap: 10px; font-size: 12px; color: var(--muted); }
  .streak { display: inline-flex; align-items: center; gap: 3px; color: var(--c); font-weight: 700; }
  .tick { width: 34px; height: 34px; margin: 0 auto; border-radius: 50%; border: 2px solid var(--border-strong); background: none; color: #fff; font-weight: 800; padding: 0; transition: transform 100ms; }
  .tick:hover { border-color: var(--c); }
  .tick.today { border-color: color-mix(in srgb, var(--c) 60%, var(--border-strong)); }
  .tick.on { background: var(--c); border-color: var(--c); animation: pop 220ms ease-out; }
  .heat { display: grid; grid-template-rows: repeat(7, 10px); grid-auto-flow: column; grid-auto-columns: 10px; gap: 3px; margin-top: 14px; overflow-x: auto; }
  .hd { width: 10px; height: 10px; border-radius: 2px; border: 0; padding: 0; background: #2f2e2a; }
  .hd.on { background: var(--c); }
  .hd.future { opacity: 0.3; cursor: default; }
  h2 { font-size: 13px; color: var(--muted); margin: 24px 0 8px; }
  .arch { display: flex; align-items: center; justify-content: space-between; padding: 6px 0; color: var(--muted); border-bottom: 1px solid var(--border); }
  .empty { text-align: center; padding: 40px 0; }
  .art { width: 72px; height: 72px; margin: 0 auto 14px; border-radius: 50%; display: grid; place-items: center; font-size: 34px; background: rgba(255, 154, 31, 0.12); }
  .empty p { margin: 4px 0; }
</style>
