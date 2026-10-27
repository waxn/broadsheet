<script>
  // A collapsible group of tasks inside a class or the Inbox (a Todoist section).
  import { ChevronDown, Ellipsis, Pencil, Trash2, Plus, GripVertical } from "@lucide/svelte";
  import TaskList from "./TaskList.svelte";
  import { deleteSection, updateSection } from "../lib/store.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { drag, endDrag, startDrag } from "../lib/drag.svelte.js";
  import { focusOnMount } from "../lib/place.js";

  let { section = null, tasks, classId = null, showDone = false, ondropSection } = $props();
  let renaming = $state(false);
  let name = $state("");
  let over = $state(false);
  const collapsed = $derived(!!section?.collapsed);

  function rename() {
    name = section.name;
    renaming = true;
  }

  function saveName(e) {
    e?.preventDefault();
    if (name.trim() && name !== section.name) updateSection(section.id, { name: name.trim() });
    renaming = false;
  }

  function menu(e) {
    openMenu(e, [
      { label: "Rename section", icon: Pencil, action: rename },
      { sep: true },
      { label: "Delete section", icon: Trash2, danger: true, action: () => confirm(`Delete “${section.name}” and its ${tasks.length} task(s)?`) && deleteSection(section.id) },
    ]);
  }
</script>

<section class="sec" class:over>
  {#if section}
    <div
      class="head"
      draggable="true"
      ondragstart={(e) => startDrag(e, "section", section.id)}
      ondragend={endDrag}
      ondragover={(e) => { if (drag.kind === "section" && drag.id !== section.id) { e.preventDefault(); over = true; } }}
      ondragleave={() => (over = false)}
      ondrop={(e) => { e.preventDefault(); over = false; ondropSection?.(drag.id, section.id); endDrag(); }}
      role="heading"
      aria-level="2"
    >
      <span class="grip"><GripVertical size={14} /></span>
      <button type="button" class="twist" class:closed={collapsed} aria-label="Toggle section" onclick={() => updateSection(section.id, { collapsed: !collapsed })}><ChevronDown size={16} /></button>
      {#if renaming}
        <form onsubmit={saveName} class="rename"><input class="input" bind:value={name} onblur={saveName} use:focusOnMount={true} onkeydown={(e) => e.key === "Escape" && (renaming = false)} /></form>
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span class="name" ondblclick={rename}>{section.name}</span>
        <span class="count">{tasks.length || ""}</span>
        <button type="button" class="icon-btn more" aria-label="Section actions" onclick={menu}><Ellipsis size={16} /></button>
      {/if}
    </div>
  {/if}
  {#if !collapsed}
    <TaskList {tasks} {showDone} place={{ class_id: classId, section_id: section?.id ?? null }} addDefaults={{ class_id: classId, section_id: section?.id ?? null }} />
  {/if}
</section>

<style>
  .sec { margin-bottom: 18px; }
  .sec.over { box-shadow: 0 -2px 0 var(--link); }
  .head { position: relative; display: flex; align-items: center; gap: 4px; height: 40px; border-bottom: 1px solid var(--border); margin-left: -40px; padding-left: 0; }
  .grip { width: 18px; color: var(--faint); opacity: 0; cursor: grab; display: grid; place-items: center; }
  .head:hover .grip { opacity: 1; }
  .twist { width: 22px; height: 22px; display: grid; place-items: center; border: 0; background: none; color: var(--muted); border-radius: 4px; padding: 0; }
  .twist.closed :global(svg) { transform: rotate(-90deg); }
  .name { font-weight: 750; font-size: 14.5px; margin-left: 6px; }
  .count { color: var(--muted); font-size: 12.5px; }
  .more { margin-left: auto; opacity: 0; }
  .head:hover .more { opacity: 1; }
  .rename { flex: 1; margin-left: 6px; }
</style>
