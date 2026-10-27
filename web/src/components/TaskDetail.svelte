<script>
  // Todoist's task view: title, description, subtasks and notes on the left; where, when and
  // how important on the right.
  import { ChevronUp, ChevronDown, X, Ellipsis, Hash, Inbox, CalendarDays, Flag, Repeat, CalendarClock, FileText, Plus, Trash2, Copy, GitBranch } from "@lucide/svelte";
  import Modal from "./Modal.svelte";
  import TaskCheck from "./TaskCheck.svelte";
  import TaskList from "./TaskList.svelte";
  import DatePicker from "./DatePicker.svelte";
  import { app, childrenOf, createPage, deleteTodo, todoById, toggleTodo, updateTodo } from "../lib/store.svelte.js";
  import { get } from "../lib/api.js";
  import { go } from "../lib/router.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { placeItems, placeLabel, priorityItems } from "../lib/pickers.js";
  import { hex, priorityColor } from "../lib/colors.js";
  import { describeRule } from "../lib/nlp.js";
  import { autosize } from "../lib/place.js";
  import * as D from "../lib/dates.js";

  let { id } = $props();
  const task = $derived(todoById(id));
  const kids = $derived(childrenOf(id));
  const parent = $derived(task?.parent_id ? todoById(task.parent_id) : null);
  let title = $state("");
  let description = $state("");
  let pages = $state([]);
  let dateAnchor = $state(null);

  $effect(() => {
    // Reload the editable copies when switching tasks.
    const t = todoById(id);
    title = t?.title ?? "";
    description = t?.description ?? "";
    pages = [];
    get(`/api/todos/${id}`).then((full) => (pages = full.pages)).catch(() => {});
  });

  $effect(() => {
    if (!task) app.detail = null;
  });

  const close = () => (app.detail = null);

  function saveTitle() {
    const v = title.trim();
    if (v && v !== task.title) updateTodo(id, { title: v });
    else title = task.title;
  }

  function saveDescription() {
    if (description !== (task.description ?? "")) updateTodo(id, { description });
  }

  function step(dir) {
    const ids = [...document.querySelectorAll("main [data-task-id]")].map((el) => Number(el.dataset.taskId));
    const i = ids.indexOf(id);
    const next = ids[i + dir];
    if (next) {
      app.selected = next;
      app.detail = next;
    }
  }

  async function newNotes() {
    const p = await createPage({ todo_id: id, title: task.title });
    if (p) {
      close();
      go(`/page/${p.id}`);
    }
  }

  function scheduleBlock() {
    const day = task.due_date || D.today();
    const start = task.due_time || "09:00";
    updateTodo(id, { start_at: D.join(day, start), end_at: D.join(day, D.hhmm(D.minutes(start) + 60)) });
  }

  function setBlock(part, value) {
    if (!value) return;
    const s = task.start_at;
    const e = task.end_at;
    const day = part === "date" ? value : D.datePart(s);
    let start = part === "start" ? value : D.timePart(s);
    let end = part === "end" ? value : D.timePart(e);
    if (end <= start) end = D.hhmm(D.minutes(start) + 30);
    updateTodo(id, { start_at: D.join(day, start), end_at: D.join(day, end) });
  }

  function more(e) {
    openMenu(e, [
      { label: "Copy title", icon: Copy, action: () => navigator.clipboard?.writeText(task.title) },
      { sep: true },
      { label: "Delete task", icon: Trash2, danger: true, action: () => deleteTodo(id) },
    ]);
  }

  function key(e) {
    if (e.target.closest("input, textarea, [contenteditable]")) return;
    if (e.key === "ArrowUp" || e.key === "k") (e.preventDefault(), step(-1));
    if (e.key === "ArrowDown" || e.key === "j") (e.preventDefault(), step(1));
  }

  const cls = $derived(task && app.classes.find((c) => c.id === task.class_id));
  // The server stores UTC ("2026-10-27 01:02:03"); show the local day.
  const localDay = (ts) => (ts ? D.label(D.iso(new Date(ts.includes("T") ? ts : `${ts.replace(" ", "T")}Z`))) : "");
</script>

<svelte:window onkeydown={key} />

{#if task}
  <Modal onclose={close} width={880} top="6vh" label="Task">
    <header class="top">
      <button type="button" class="crumb" onclick={() => { close(); go(task.class_id ? `/class/${task.class_id}` : "/inbox"); }}>
        {#if cls}<Hash size={14} color={hex(cls.color)} />{:else}<Inbox size={14} />{/if}
        {placeLabel(task.class_id, task.section_id)}
      </button>
      {#if parent}<span class="muted">›</span><button type="button" class="crumb" onclick={() => (app.detail = parent.id)}><GitBranch size={13} /> {parent.title}</button>{/if}
      <span class="spacer"></span>
      <button type="button" class="icon-btn" aria-label="Previous task" onclick={() => step(-1)}><ChevronUp size={18} /></button>
      <button type="button" class="icon-btn" aria-label="Next task" onclick={() => step(1)}><ChevronDown size={18} /></button>
      <button type="button" class="icon-btn" aria-label="More" onclick={more}><Ellipsis size={18} /></button>
      <button type="button" class="icon-btn" aria-label="Close" onclick={close}><X size={18} /></button>
    </header>

    <div class="cols">
      <div class="main">
        <div class="titlerow">
          <span class="chk"><TaskCheck done={!!task.done} priority={task.priority} size={20} onclick={() => toggleTodo(id)} /></span>
          <textarea class="title" rows="1" bind:value={title} use:autosize onblur={saveTitle} onkeydown={(e) => e.key === "Enter" && (e.preventDefault(), e.currentTarget.blur())} aria-label="Task name"></textarea>
        </div>
        <textarea class="desc" rows="2" placeholder="Description" bind:value={description} use:autosize onblur={saveDescription} aria-label="Description"></textarea>

        <section class="block">
          <h3>Sub-tasks {#if kids.length}<span class="muted">{kids.filter((k) => k.done).length}/{kids.length}</span>{/if}</h3>
          <div class="subs">
            <TaskList tasks={kids} showDone addDefaults={{ parent_id: id, class_id: task.class_id, section_id: task.section_id }} addLabel="Add sub-task" compact />
          </div>
        </section>

        <section class="block">
          <h3>Notes</h3>
          {#each pages as p (p.id)}
            <a class="page" href="/page/{p.id}" onclick={close}>
              {#if p.icon}<span>{p.icon}</span>{:else}<FileText size={15} />{/if}
              {p.title || "Untitled"}
            </a>
          {/each}
          <button type="button" class="addnote" onclick={newNotes}><Plus size={15} /> {pages.length ? "Another notes page" : "Write notes for this task"}</button>
        </section>
      </div>

      <aside class="side">
        <div class="field">
          <span class="label">Class</span>
          <button type="button" class="value" onclick={(e) => openMenu(e, placeItems(task, (place) => updateTodo(id, place)))}>
            {#if cls}<Hash size={14} color={hex(cls.color)} />{:else}<Inbox size={14} />{/if}
            {placeLabel(task.class_id, task.section_id)}
          </button>
        </div>
        <div class="field">
          <span class="label">Date</span>
          <button type="button" class="value {task.due_date ? D.tone(task.due_date) : ''}" onclick={(e) => (dateAnchor = e.currentTarget)}>
            <CalendarDays size={14} />
            {task.due_date ? `${D.label(task.due_date)}${task.due_time ? ` ${D.fmtTime(task.due_time)}` : ""}` : "No date"}
          </button>
          {#if task.recurrence}<div class="rec"><Repeat size={12} /> {describeRule(task.recurrence)}</div>{/if}
        </div>
        <div class="field">
          <span class="label">Time block</span>
          {#if task.start_at && task.end_at}
            <div class="block-edit">
              <input type="date" class="input sm" value={D.datePart(task.start_at)} onchange={(e) => setBlock("date", e.currentTarget.value)} aria-label="Block date" />
              <div class="times">
                <input type="time" class="input sm" value={D.timePart(task.start_at)} onchange={(e) => setBlock("start", e.currentTarget.value)} aria-label="Start" />
                <span>–</span>
                <input type="time" class="input sm" value={D.timePart(task.end_at)} onchange={(e) => setBlock("end", e.currentTarget.value)} aria-label="End" />
              </div>
              <button type="button" class="link" onclick={() => updateTodo(id, { start_at: null, end_at: null })}>Unschedule</button>
            </div>
          {:else}
            <button type="button" class="value" onclick={scheduleBlock}><CalendarClock size={14} /> Schedule on calendar</button>
          {/if}
        </div>
        <div class="field">
          <span class="label">Priority</span>
          <button type="button" class="value" onclick={(e) => openMenu(e, priorityItems(task.priority, (p) => updateTodo(id, { priority: p })))}>
            <Flag size={14} color={priorityColor(task.priority)} fill={task.priority ? priorityColor(task.priority) : "none"} /> P{4 - task.priority}
          </button>
        </div>
        <p class="created">Added {localDay(task.created_at)}{task.completed_at && task.done ? ` · done ${localDay(task.completed_at)}` : ""}</p>
      </aside>
    </div>
  </Modal>

  {#if dateAnchor}
    <DatePicker anchor={dateAnchor} value={task} onchange={(v) => updateTodo(id, { due_date: v.due_date ?? null, due_time: v.due_time ?? null, recurrence: v.recurrence ?? null })} onclose={() => (dateAnchor = null)} />
  {/if}
{/if}

<style>
  .top { display: flex; align-items: center; gap: 6px; padding: 10px 12px 10px 18px; border-bottom: 1px solid var(--border); }
  .crumb { display: inline-flex; align-items: center; gap: 6px; border: 0; background: none; color: var(--text-2); font-size: 13px; padding: 4px 6px; border-radius: 6px; }
  .crumb:hover { background: var(--hover); }
  .cols { display: flex; min-height: 420px; }
  .main { flex: 1; min-width: 0; padding: 18px 24px 30px 18px; }
  .titlerow { display: flex; gap: 10px; align-items: flex-start; }
  .chk { padding-top: 5px; }
  .title { flex: 1; border: 0; outline: none; background: none; resize: none; font-size: 20px; font-weight: 750; line-height: 1.35; padding: 0; letter-spacing: -0.01em; }
  .desc { display: block; width: 100%; margin: 6px 0 0 30px; width: calc(100% - 30px); border: 0; outline: none; background: none; resize: none; color: var(--text-2); font-size: 14px; padding: 0; min-height: 40px; }
  .desc::placeholder { color: var(--faint); }
  .block { margin: 22px 0 0 30px; }
  h3 { font-size: 13px; font-weight: 700; margin: 0 0 4px; display: flex; gap: 8px; }
  .subs { padding-left: 40px; margin-left: -40px; }
  .page { display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: 6px; color: var(--text); text-decoration: none; margin-left: -8px; }
  .page:hover { background: var(--hover); text-decoration: none; }
  .addnote { display: inline-flex; align-items: center; gap: 6px; border: 0; background: none; color: var(--muted); padding: 7px 0; font-size: 13.5px; }
  .addnote:hover { color: var(--accent); }
  .side { width: 260px; flex: none; background: var(--sidebar); border-left: 1px solid var(--border); padding: 14px 14px; border-bottom-right-radius: 14px; }
  .field { padding: 6px 0 10px; border-bottom: 1px solid var(--border); margin-bottom: 6px; }
  .label { display: block; font-size: 12px; font-weight: 650; color: var(--muted); margin-bottom: 4px; padding-left: 6px; }
  .value { display: flex; align-items: center; gap: 8px; width: 100%; border: 0; background: none; padding: 5px 6px; border-radius: 6px; text-align: left; font-size: 13.5px; color: var(--text); }
  .value:hover { background: var(--hover); }
  .value.overdue { color: var(--overdue); }
  .value.today { color: var(--today); }
  .value.tomorrow { color: var(--tomorrow); }
  .value.week { color: var(--week); }
  .rec { display: flex; align-items: center; gap: 5px; padding: 0 6px; font-size: 12px; color: var(--muted); }
  .block-edit { display: flex; flex-direction: column; gap: 6px; padding: 0 6px; }
  .times { display: flex; align-items: center; gap: 4px; }
  .input.sm { height: 30px; padding: 0 6px; font-size: 13px; }
  .link { border: 0; background: none; color: var(--accent); padding: 0; font-size: 12.5px; text-align: left; }
  .created { font-size: 12px; color: var(--faint); padding: 0 6px; }
</style>
