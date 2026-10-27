<script>
  import { untrack } from "svelte";
  import Modal from "./Modal.svelte";
  import ColorPicker from "./ColorPicker.svelte";
  import { createClass, updateClass } from "../lib/store.svelte.js";
  import { go } from "../lib/router.svelte.js";
  import { focusOnMount } from "../lib/place.js";

  let { cls = null, onclose } = $props();
  // A form starts from the cls it was opened with (the dialog is re-created for another one).
  const initial = untrack(() => $state.snapshot(cls));
  let name = $state(initial?.name ?? "");
  let section = $state(initial?.section ?? "");
  let color = $state(initial?.color ?? "charcoal");
  let emoji = $state(initial?.emoji ?? "");
  let view = $state(initial?.view ?? "list");

  async function save(e) {
    e.preventDefault();
    if (!name.trim()) return;
    const fields = { name, section, color, emoji, view };
    if (cls) await updateClass(cls.id, fields);
    else {
      const c = await createClass(fields);
      if (c) go(`/class/${c.id}`);
    }
    onclose();
  }
</script>

<Modal {onclose} width={460} label={cls ? "Edit class" : "Add class"}>
  <form class="form" onsubmit={save}>
    <h2>{cls ? "Edit class" : "Add class"}</h2>
    <label>Name
      <div class="name">
        <input class="input emoji" bind:value={emoji} maxlength="4" placeholder="📘" aria-label="Emoji" />
        <input class="input" bind:value={name} maxlength="120" required use:focusOnMount placeholder="Biology" />
      </div>
    </label>
    <label>Teacher, room or period <input class="input" bind:value={section} maxlength="120" placeholder="Mr. Lee · Room 204" /></label>
    <div class="field"><span>Colour</span><ColorPicker bind:value={color} /></div>
    <div class="field"><span>Layout</span>
      <div class="seg">
        <button type="button" class:on={view === "list"} onclick={() => (view = "list")}>List</button>
        <button type="button" class:on={view === "board"} onclick={() => (view = "board")}>Board</button>
      </div>
    </div>
    <footer>
      <button type="button" class="btn ghost" onclick={onclose}>Cancel</button>
      <button class="btn primary" disabled={!name.trim()}>{cls ? "Save" : "Add"}</button>
    </footer>
  </form>
</Modal>

<style>
  .form { padding: 20px 22px; display: flex; flex-direction: column; gap: 14px; }
  h2 { margin: 0; font-size: 18px; font-weight: 750; }
  label, .field { display: flex; flex-direction: column; gap: 6px; font-size: 13px; font-weight: 600; color: var(--text-2); }
  .name { display: flex; gap: 8px; }
  .emoji { width: 52px; text-align: center; flex: none; }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; width: max-content; }
  .seg button { border: 0; background: transparent; padding: 6px 14px; color: var(--text-2); font-weight: 550; }
  .seg button.on { background: var(--active); color: var(--text); }
  footer { display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px; }
</style>
