<script>
  // Notes home: favorites and recent pages as cards, then every notebook.
  import { Plus, FileText, Hash, CircleCheck, Star } from "@lucide/svelte";
  import ViewHeader from "../components/ViewHeader.svelte";
  import PageTreeItem from "../components/PageTreeItem.svelte";
  import { app, createPage, pageChildren } from "../lib/store.svelte.js";
  import { go } from "../lib/router.svelte.js";
  import { hex } from "../lib/colors.js";

  const recent = $derived([...app.pages].sort((a, b) => (b.updated_at || "").localeCompare(a.updated_at || "")).slice(0, 8));
  const favorites = $derived(app.pages.filter((p) => p.favorite));
  const roots = $derived(pageChildren(null));
  const classBooks = $derived(
    app.classes
      .filter((c) => !c.archived)
      .map((c) => ({ c, pages: app.pages.filter((p) => p.class_id === c.id && p.parent_id == null).sort((a, b) => a.position - b.position || a.id - b.id) }))
      .filter((x) => x.pages.length),
  );
  const taskNotes = $derived(app.pages.filter((p) => p.todo_id != null && p.parent_id == null));

  async function newPage(fields = {}) {
    const p = await createPage(fields);
    if (p) go(`/page/${p.id}`);
  }

  function ago(ts) {
    if (!ts) return "";
    const then = new Date(ts.replace(" ", "T") + "Z");
    const mins = Math.round((Date.now() - then) / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const h = Math.round(mins / 60);
    if (h < 24) return `${h}h ago`;
    const d = Math.round(h / 24);
    return d < 30 ? `${d}d ago` : then.toLocaleDateString();
  }
</script>

<div class="wrap">
  <ViewHeader title="Notes" subtitle={`${app.pages.length} page${app.pages.length === 1 ? "" : "s"}`}>
    <button type="button" class="btn primary" onclick={() => newPage()}><Plus size={16} /> New page</button>
  </ViewHeader>

  {#if favorites.length}
    <h2><Star size={14} /> Favorites</h2>
    <div class="cards">
      {#each favorites as p (p.id)}
        <a class="card" href="/page/{p.id}"><span class="ci">{p.icon || "📄"}</span><span class="ct">{p.title || "Untitled"}</span><span class="cm">{ago(p.updated_at)}</span></a>
      {/each}
    </div>
  {/if}

  {#if recent.length}
    <h2>Recently edited</h2>
    <div class="cards">
      {#each recent as p (p.id)}
        <a class="card" href="/page/{p.id}"><span class="ci">{p.icon || "📄"}</span><span class="ct">{p.title || "Untitled"}</span><span class="cm">{ago(p.updated_at)}</span></a>
      {/each}
    </div>
  {/if}

  <div class="cols">
    <section>
      <h2>Pages <button type="button" class="icon-btn mini" aria-label="New page" onclick={() => newPage()}><Plus size={15} /></button></h2>
      <div role="tree" class="tree">
        {#each roots as p (p.id)}<PageTreeItem page={p} />{:else}<p class="muted small">No pages yet. Make one for anything: lecture notes, a reading list, plans.</p>{/each}
      </div>
    </section>
    <section>
      <h2>Class notebooks</h2>
      {#each classBooks as { c, pages } (c.id)}
        <div class="book">
          <a class="bookhead" href="/class/{c.id}?tab=notes">{#if c.emoji}<span>{c.emoji}</span>{:else}<Hash size={15} color={hex(c.color)} />{/if}{c.name}</a>
          <div class="tree">{#each pages as p (p.id)}<PageTreeItem page={p} depth={1} />{/each}</div>
          <button type="button" class="addp" onclick={() => newPage({ class_id: c.id })}><Plus size={13} /> Page in {c.name}</button>
        </div>
      {:else}
        <p class="muted small">Each class gets its own notebook.</p>
      {/each}
      {#if taskNotes.length}
        <h2>Task notes</h2>
        <div class="tree">
          {#each taskNotes as p (p.id)}
            {@const t = app.todos.find((x) => x.id === p.todo_id)}
            <a class="tn" href="/page/{p.id}"><CircleCheck size={14} /><span>{p.title || t?.title || "Untitled"}</span></a>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .wrap { max-width: 1000px; margin: 0 auto; padding: 0 56px 120px; }
  h2 { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; color: var(--muted); margin: 26px 0 10px; text-transform: uppercase; letter-spacing: .05em; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 12px; }
  .card {
    display: flex; flex-direction: column; gap: 6px; padding: 14px; height: 120px; border-radius: 12px; text-decoration: none;
    background: var(--surface); border: 1px solid var(--border); color: var(--text); transition: transform 120ms, border-color 120ms;
  }
  .card:hover { transform: translateY(-2px); border-color: var(--border-strong); text-decoration: none; }
  .ci { font-size: 26px; }
  .ct { font-weight: 650; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .cm { margin-top: auto; font-size: 12px; color: var(--muted); }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 40px; }
  .tree { margin-left: -8px; }
  .mini { width: 22px; height: 22px; }
  .small { font-size: 13px; }
  .book { margin-bottom: 14px; }
  .bookhead { display: flex; align-items: center; gap: 8px; font-weight: 650; color: var(--text); padding: 4px 0; text-decoration: none; }
  .addp { display: inline-flex; align-items: center; gap: 4px; border: 0; background: none; color: var(--faint); font-size: 12.5px; padding: 4px 6px; margin-left: 14px; }
  .addp:hover { color: var(--accent); }
  .tn { display: flex; align-items: center; gap: 8px; padding: 5px 8px; border-radius: 6px; color: var(--text-2); text-decoration: none; }
  .tn:hover { background: var(--hover); text-decoration: none; }
  @media (max-width: 900px) { .cols { grid-template-columns: 1fr; } }
</style>
