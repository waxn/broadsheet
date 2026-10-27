<script>
  // The Inbox (classId = null) or one class: its tasks as a list with sections or as a board,
  // and for classes, the class's notes.
  import { List, SquareKanban, Ellipsis, Pencil, Plus, CircleCheck, NotebookPen, ListChecks, Archive, Trash2 } from "@lucide/svelte";
  import ViewHeader from "../components/ViewHeader.svelte";
  import Section from "../components/Section.svelte";
  import TaskList from "../components/TaskList.svelte";
  import ClassDialog from "../components/ClassDialog.svelte";
  import PageView from "./PageView.svelte";
  import { app, createSection, deleteClass, loadCompleted, localGet, localSet, reorder, updateClass } from "../lib/store.svelte.js";
  import { go, route } from "../lib/router.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { hex } from "../lib/colors.js";
  import { moved } from "../lib/drag.svelte.js";
  import { focusOnMount } from "../lib/place.js";

  let { classId = null } = $props();

  const cls = $derived(classId == null ? null : app.classes.find((c) => c.id === classId));
  const view = $derived(cls ? cls.view : localGet("inboxView", "list"));
  let inboxView = $state(localGet("inboxView", "list"));
  const layout = $derived(cls ? cls.view : inboxView);
  let showDone = $state(false);
  let editing = $state(false);
  let addingSection = $state(false);
  let sectionName = $state("");
  const tab = $derived(route.query.get("tab") === "notes" && cls ? "notes" : "tasks");

  const sections = $derived(app.sections.filter((s) => (s.class_id ?? null) === classId).sort((a, b) => a.position - b.position || a.id - b.id));
  const mine = $derived(
    app.todos
      .filter((t) => (t.class_id ?? null) === classId && t.parent_id == null && (showDone || !t.done))
      .sort((a, b) => a.done - b.done || a.position - b.position || a.id - b.id),
  );
  const loose = $derived(mine.filter((t) => t.section_id == null || !sections.some((s) => s.id === t.section_id)));
  const bySection = (id) => mine.filter((t) => t.section_id === id);
  const openCount = $derived(app.todos.filter((t) => (t.class_id ?? null) === classId && !t.done).length);

  function setLayout(v) {
    if (cls) updateClass(cls.id, { view: v });
    else {
      inboxView = v;
      localSet("inboxView", v);
    }
  }

  async function toggleDone() {
    showDone = !showDone;
    if (showDone) await loadCompleted();
  }

  async function addSection(e) {
    e.preventDefault();
    if (!sectionName.trim()) return;
    await createSection({ name: sectionName.trim(), class_id: classId });
    sectionName = "";
    addingSection = false;
  }

  function dropSection(id, targetId) {
    reorder("sections", moved(sections, id, targetId, "before"));
  }

  function menu(e) {
    const items = [
      { label: "Add section", icon: Plus, action: () => (addingSection = true) },
      { label: showDone ? "Hide completed" : "Show completed", icon: CircleCheck, action: toggleDone },
    ];
    if (cls) {
      items.push(
        { sep: true },
        { label: "Edit class", icon: Pencil, action: () => (editing = true) },
        { label: cls.archived ? "Unarchive" : "Archive", icon: Archive, action: () => updateClass(cls.id, { archived: !cls.archived }) },
        { label: "Delete class", icon: Trash2, danger: true, action: () => confirm(`Delete “${cls.name}”? Its tasks move to the Inbox; its notes are deleted.`) && (deleteClass(cls.id), go("/inbox")) },
      );
    }
    openMenu(e, items);
  }

  const setTab = (t) => go(`/class/${classId}${t === "notes" ? "?tab=notes" : ""}`, { replace: true });
</script>

{#if classId != null && !cls}
  <div class="wrap"><ViewHeader title="Class not found" subtitle="It may have been deleted." /></div>
{:else}
  <div class="wrap" class:wide={layout === "board" && tab === "tasks"}>
    <ViewHeader title={cls ? cls.name : "Inbox"} icon={cls?.emoji || null} subtitle={cls ? [cls.section, `${openCount} open`].filter(Boolean).join(" · ") : `${openCount} open`}>
      {#if tab === "tasks"}
        <div class="seg" role="group" aria-label="Layout">
          <button type="button" class:on={layout === "list"} onclick={() => setLayout("list")} title="List"><List size={16} /> List</button>
          <button type="button" class:on={layout === "board"} onclick={() => setLayout("board")} title="Board"><SquareKanban size={16} /> Board</button>
        </div>
      {/if}
      <button type="button" class="icon-btn" aria-label="More" onclick={menu}><Ellipsis size={18} /></button>
    </ViewHeader>

    {#if cls}
      <nav class="tabs">
        <button type="button" class:on={tab === "tasks"} onclick={() => setTab("tasks")}><ListChecks size={15} /> Tasks</button>
        <button type="button" class:on={tab === "notes"} onclick={() => setTab("notes")}><NotebookPen size={15} /> Notes</button>
        <span class="bar" style:background={hex(cls.color)}></span>
      </nav>
    {/if}

    {#if tab === "notes" && cls}
      {#if cls.home_page_id}
        {#key cls.home_page_id}<PageView pageId={cls.home_page_id} embedded />{/key}
      {:else}
        <p class="muted">This class has no notes page yet.</p>
      {/if}
    {:else if layout === "board"}
      <div class="board">
        <div class="col">
          <div class="colhead"><span>(No section)</span><span class="count">{loose.length || ""}</span></div>
          <TaskList tasks={loose} {showDone} compact place={{ class_id: classId, section_id: null }} addDefaults={{ class_id: classId, section_id: null }} />
        </div>
        {#each sections as s (s.id)}
          <div class="col">
            <Section section={s} tasks={bySection(s.id)} {classId} {showDone} ondropSection={dropSection} />
          </div>
        {/each}
        <div class="col ghost">
          {#if addingSection}
            <form onsubmit={addSection} class="newsec"><input class="input" placeholder="Name this section" bind:value={sectionName} use:focusOnMount onkeydown={(e) => e.key === "Escape" && (addingSection = false)} /><div class="row"><button class="btn primary sm">Add section</button><button type="button" class="btn ghost sm" onclick={() => (addingSection = false)}>Cancel</button></div></form>
          {:else}
            <button type="button" class="addsec" onclick={() => (addingSection = true)}><Plus size={16} /> Add section</button>
          {/if}
        </div>
      </div>
    {:else}
      <TaskList tasks={loose} {showDone} place={{ class_id: classId, section_id: null }} addDefaults={{ class_id: classId, section_id: null }} />
      {#each sections as s (s.id)}
        <Section section={s} tasks={bySection(s.id)} {classId} {showDone} ondropSection={dropSection} />
      {/each}
      {#if addingSection}
        <form onsubmit={addSection} class="newsec"><input class="input" placeholder="Name this section" bind:value={sectionName} use:focusOnMount onkeydown={(e) => e.key === "Escape" && (addingSection = false)} /><div class="row"><button class="btn primary sm">Add section</button><button type="button" class="btn ghost sm" onclick={() => (addingSection = false)}>Cancel</button></div></form>
      {:else}
        <button type="button" class="divider-add" onclick={() => (addingSection = true)}><span>Add section</span></button>
      {/if}
      {#if !mine.length && !sections.length}
        <div class="empty">
          <div class="art">✓</div>
          <p><b>{cls ? "Nothing here yet" : "Your Inbox is clear"}</b></p>
          <p class="muted">Press <kbd>Q</kbd> to add a task from anywhere. Try “Read ch. 4 fri p2{cls ? ` #${cls.name.replace(/\s+/g, "")}` : ""}”.</p>
        </div>
      {/if}
    {/if}
  </div>
{/if}

{#if editing && cls}
  <ClassDialog {cls} onclose={() => (editing = false)} />
{/if}

<style>
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 56px 120px; }
  .wrap.wide { max-width: none; }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; margin-right: 4px; }
  .seg button { display: inline-flex; align-items: center; gap: 6px; border: 0; background: transparent; padding: 5px 10px; color: var(--muted); font-size: 12.5px; font-weight: 600; }
  .seg button.on { background: var(--active); color: var(--text); }
  .tabs { position: relative; display: flex; gap: 4px; margin: 0 0 14px; border-bottom: 1px solid var(--border); }
  .tabs button { display: inline-flex; align-items: center; gap: 6px; border: 0; background: none; color: var(--muted); padding: 8px 10px; font-weight: 600; border-bottom: 2px solid transparent; margin-bottom: -1px; }
  .tabs button.on { color: var(--text); border-bottom-color: var(--text); }
  .bar { position: absolute; right: 0; top: 14px; width: 10px; height: 10px; border-radius: 50%; }
  .board { display: flex; align-items: flex-start; gap: 16px; overflow-x: auto; padding-bottom: 24px; padding-left: 40px; margin-left: -40px; }
  .col { width: 290px; flex: none; }
  .col :global(.row) { margin-left: 0; padding: 10px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface); margin-bottom: 8px; }
  .col :global(.row .grip), .col :global(.row .twist), .col :global(.row .twist-space) { display: none; }
  .col :global(.row.selected) { background: var(--raised); }
  .col :global(.row.selected::before) { display: none; }
  .col :global(.sec .head) { margin-left: 0; border-bottom: 0; }
  .col :global(.sec .head .grip) { display: none; }
  .colhead { display: flex; align-items: center; gap: 8px; height: 40px; font-weight: 750; font-size: 14.5px; }
  .count { color: var(--muted); font-size: 12.5px; font-weight: 500; }
  .addsec { display: flex; align-items: center; gap: 8px; height: 40px; border: 0; background: none; color: var(--muted); font-weight: 600; }
  .addsec:hover { color: var(--accent); }
  .newsec { display: flex; flex-direction: column; gap: 8px; margin: 10px 0; }
  .newsec .row { display: flex; gap: 6px; }
  .divider-add { position: relative; width: 100%; height: 30px; border: 0; background: none; color: var(--accent); font-weight: 650; font-size: 13px; opacity: 0; transition: opacity 120ms; }
  .divider-add:hover { opacity: 1; }
  .divider-add::before { content: ""; position: absolute; left: 0; right: 0; top: 50%; height: 1px; background: var(--accent); }
  .divider-add span { position: relative; background: var(--bg); padding: 0 10px; }
  .empty { text-align: center; padding: 60px 0; }
  .art { width: 72px; height: 72px; margin: 0 auto 14px; border-radius: 50%; display: grid; place-items: center; font-size: 34px; color: var(--today); background: rgba(79, 191, 106, 0.12); }
  .empty p { margin: 4px 0; }
</style>
