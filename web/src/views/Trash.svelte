<script>
  import { RotateCcw, Trash2, FileText } from "@lucide/svelte";
  import ViewHeader from "../components/ViewHeader.svelte";
  import { get } from "../lib/api.js";
  import { deletePage, restorePage } from "../lib/store.svelte.js";
  import { fail } from "../lib/toast.svelte.js";

  let pages = $state([]);
  async function load() {
    try {
      pages = await get("/api/pages?archived=1");
    } catch (err) {
      fail(err);
    }
  }
  $effect(() => {
    load();
  });

  async function restore(id) {
    await restorePage(id);
    load();
  }
  async function remove(p) {
    if (!confirm(`Delete “${p.title || "Untitled"}” forever?`)) return;
    await deletePage(p.id);
    load();
  }
</script>

<div class="wrap">
  <ViewHeader title="Trash" subtitle="Pages you deleted. Restore them or delete them for good." />
  {#each pages as p (p.id)}
    <div class="row">
      <span class="icon">{#if p.icon}{p.icon}{:else}<FileText size={16} />{/if}</span>
      <span class="title">{p.title || "Untitled"}</span>
      <span class="muted when">{p.updated_at?.slice(0, 10)}</span>
      <button type="button" class="btn ghost sm" onclick={() => restore(p.id)}><RotateCcw size={14} /> Restore</button>
      <button type="button" class="btn danger sm" onclick={() => remove(p)}><Trash2 size={14} /> Delete</button>
    </div>
  {:else}
    <p class="muted">Trash is empty.</p>
  {/each}
</div>

<style>
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 56px 120px; }
  .row { display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .icon { width: 20px; display: grid; place-items: center; color: var(--muted); }
  .title { flex: 1; }
  .when { font-size: 12px; }
</style>
