<script>
  import { ChevronRight, FileText, Plus, Ellipsis } from "@lucide/svelte";
  import Self from "./PageTreeItem.svelte";
  import { app, createPage, pageChildren, reorder, trashPage, updatePage } from "../lib/store.svelte.js";
  import { go, route } from "../lib/router.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { drag, endDrag, half, moved, startDrag } from "../lib/drag.svelte.js";
  import { Star, Trash2, Copy } from "@lucide/svelte";

  let { page, depth = 0 } = $props();
  const kids = $derived(pageChildren(page.id));
  let open = $state(false);
  let over = $state(null);

  // Opening a page reveals it in the tree.
  $effect(() => {
    if (route.name === "page" && route.id !== page.id && containsCurrent()) open = true;
  });
  function containsCurrent() {
    let p = app.pages.find((x) => x.id === route.id);
    for (let i = 0; p && i < 50; i++) {
      if (p.parent_id === page.id) return true;
      p = app.pages.find((x) => x.id === p.parent_id);
    }
    return false;
  }

  async function addChild(e) {
    e?.stopPropagation();
    const p = await createPage({ parent_id: page.id });
    if (p) {
      open = true;
      go(`/page/${p.id}`);
    }
  }

  function more(e) {
    openMenu(e, [
      { label: page.favorite ? "Remove from favorites" : "Add to favorites", icon: Star, action: () => updatePage(page.id, { favorite: !page.favorite }) },
      { label: "Add sub-page", icon: Plus, action: addChild },
      { label: "Copy link", icon: Copy, action: () => navigator.clipboard?.writeText(`${location.origin}/page/${page.id}`) },
      { sep: true },
      { label: "Move to trash", icon: Trash2, danger: true, action: () => trashPage(page.id) },
    ]);
  }

  function dragover(e) {
    if (drag.kind !== "page" || drag.id === page.id) return;
    e.preventDefault();
    const r = e.currentTarget.getBoundingClientRect();
    const y = (e.clientY - r.top) / r.height;
    over = y < 0.25 ? "before" : y > 0.75 ? "after" : "inside";
  }

  function drop(e) {
    e.preventDefault();
    const id = drag.id;
    const where = over;
    over = null;
    endDrag();
    if (id == null || id === page.id) return;
    if (where === "inside") {
      reorder("pages", [{ id, parent_id: page.id, position: kids.length }]);
      open = true;
    } else {
      const siblings = page.parent_id == null ? pageChildren(null) : pageChildren(page.parent_id);
      const items = moved(siblings, id, page.id, where).map((it) => (it.id === id ? { ...it, parent_id: page.parent_id ?? null } : it));
      reorder("pages", items);
    }
  }
</script>

<div
  class="item"
  class:current={route.name === "page" && route.id === page.id}
  class:over-before={over === "before"}
  class:over-after={over === "after"}
  class:over-inside={over === "inside"}
  style:padding-left="{8 + depth * 14}px"
  draggable="true"
  ondragstart={(e) => startDrag(e, "page", page.id)}
  ondragend={endDrag}
  ondragover={dragover}
  ondragleave={() => (over = null)}
  ondrop={drop}
  role="treeitem"
  aria-selected={route.id === page.id}
  aria-expanded={open}
  tabindex="-1"
>
  <button type="button" class="twist" class:open aria-label={open ? "Collapse" : "Expand"} onclick={() => (open = !open)}>
    {#if kids.length}<ChevronRight size={14} />{:else}<span class="leaf"></span>{/if}
  </button>
  <a href="/page/{page.id}" class="label">
    {#if page.icon}<span class="emoji">{page.icon}</span>{:else}<FileText size={15} strokeWidth={1.75} />{/if}
    <span class="name">{page.title || "Untitled"}</span>
  </a>
  <span class="actions">
    <button type="button" class="icon-btn mini" aria-label="Page actions" onclick={more}><Ellipsis size={15} /></button>
    <button type="button" class="icon-btn mini" aria-label="Add sub-page" onclick={addChild}><Plus size={15} /></button>
  </span>
</div>
{#if open}
  {#each kids as child (child.id)}
    <Self page={child} depth={depth + 1} />
  {/each}
{/if}

<style>
  .item { display: flex; align-items: center; gap: 2px; height: 30px; padding-right: 4px; border-radius: 6px; color: var(--text-2); position: relative; }
  .item:hover { background: var(--hover); }
  .item.current { background: var(--active); color: var(--text); }
  .over-before { box-shadow: inset 0 2px 0 var(--link); }
  .over-after { box-shadow: inset 0 -2px 0 var(--link); }
  .over-inside { background: rgba(122, 167, 255, 0.18); }
  .twist { width: 20px; height: 20px; display: grid; place-items: center; border: 0; background: none; color: var(--muted); padding: 0; border-radius: 4px; flex: none; }
  .twist:hover { background: var(--active); }
  .twist :global(svg) { transition: transform 120ms; }
  .twist.open :global(svg) { transform: rotate(90deg); }
  .leaf { width: 4px; height: 4px; border-radius: 50%; background: var(--faint); }
  .label { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; color: inherit; text-decoration: none; height: 100%; }
  .emoji { width: 16px; text-align: center; font-size: 14px; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .actions { display: none; }
  .item:hover .actions { display: flex; }
  .mini { width: 22px; height: 22px; }
</style>
