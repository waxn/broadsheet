<script>
  // One page: icon, title, the editor, sub-pages and backlinks. Saves itself as you type.
  import { onDestroy } from "svelte";
  import { Star, Maximize2, Ellipsis, Copy, Download, Trash2, FileText, MoveHorizontal, CaseSensitive, SmilePlus, ArrowUpRight, FolderInput, CircleCheck, Hash, Link2 } from "@lucide/svelte";
  import EmojiPicker from "../components/EmojiPicker.svelte";
  import { get } from "../lib/api.js";
  import { app, pageById, trashPage, updatePage, reorder } from "../lib/store.svelte.js";
  import { go } from "../lib/router.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { fail, toast } from "../lib/toast.svelte.js";
  import { autosize } from "../lib/place.js";

  let { pageId, embedded = false } = $props();

  // The editor is the heaviest part of the app, so it loads only when a page is opened.
  const editorModule = import("../editor/Editor.svelte");

  let page = $state(null);
  let error = $state("");
  let title = $state("");
  let status = $state("saved"); // saved | saving | error
  let headings = $state([]);
  let editorRef = $state();
  let iconAnchor = $state(null);
  let pending = null;
  let titleTimer;
  let saveTimer;

  const meta = $derived(pageById(pageId));
  const children = $derived(app.pages.filter((p) => p.parent_id === pageId).sort((a, b) => a.position - b.position || a.id - b.id));
  const owner = $derived.by(() => {
    const root = page?.path?.[0] ?? page;
    if (!root) return null;
    if (root.todo_id) return { kind: "todo", id: root.todo_id, label: app.todos.find((t) => t.id === root.todo_id)?.title ?? "Task" };
    if (root.class_id) return { kind: "class", id: root.class_id, label: app.classes.find((c) => c.id === root.class_id)?.name ?? "Class" };
    return null;
  });

  async function load() {
    try {
      page = await get(`/api/pages/${pageId}`);
      title = page.title;
    } catch (err) {
      error = err.message;
    }
  }
  load();

  async function save() {
    if (!pending) return;
    const body = pending;
    pending = null;
    status = "saving";
    try {
      await updatePage(pageId, body);
      status = pending ? "saving" : "saved";
    } catch {
      status = "error";
      pending = { ...body, ...(pending || {}) };
    }
  }

  function changed(snap) {
    pending = { ...(pending || {}), ...snap, body: "" };
    status = "saving";
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 600);
  }

  function titleInput() {
    const m = pageById(pageId);
    if (m) m.title = title;
    clearTimeout(titleTimer);
    titleTimer = setTimeout(() => updatePage(pageId, { title: title.trim() }).catch(() => {}), 400);
  }

  function titleKey(e) {
    if (e.key === "Enter" || e.key === "ArrowDown") {
      e.preventDefault();
      editorRef?.focus("start");
    }
  }

  // Save anything unsaved when leaving the page or closing the tab.
  function flush() {
    clearTimeout(saveTimer);
    if (pending) {
      fetch(`/api/pages/${pageId}`, { method: "PATCH", keepalive: true, credentials: "same-origin", headers: { "Content-Type": "application/json" }, body: JSON.stringify(pending) });
      pending = null;
    }
    clearTimeout(titleTimer);
    if (page && title !== page.title) updatePage(pageId, { title: title.trim() }).catch(() => {});
  }
  onDestroy(flush);

  const fontClass = $derived(`font-${meta?.font ?? page?.font ?? "default"}`);
  const fullWidth = $derived(!!(meta?.full_width ?? page?.full_width));
  const favorite = $derived(!!meta?.favorite);

  function setMeta(fields) {
    if (page) Object.assign(page, fields);
    updatePage(pageId, fields).catch(() => {});
  }

  function fontMenu(e) {
    const cur = page?.font ?? "default";
    openMenu(e, [
      { heading: "Page font" },
      { label: "Default (sans)", checked: cur === "default", action: () => setMeta({ font: "default" }) },
      { label: "Serif", checked: cur === "serif", action: () => setMeta({ font: "serif" }) },
      { label: "Mono", checked: cur === "mono", action: () => setMeta({ font: "mono" }) },
    ]);
  }

  function exportMarkdown() {
    const md = `# ${title || "Untitled"}\n\n${editorRef?.getMarkdown() ?? ""}`;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([md], { type: "text/markdown" }));
    a.download = `${(title || "Untitled").replace(/[\\/:*?"<>|]+/g, "-")}.md`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  }

  function moveMenu(e) {
    const blocked = new Set([pageId]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const p of app.pages) if (p.parent_id && blocked.has(p.parent_id) && !blocked.has(p.id)) (blocked.add(p.id), (grew = true));
    }
    const targets = app.pages.filter((p) => !blocked.has(p.id)).slice(0, 40);
    openMenu({ x: e.clientX, y: e.clientY }, [
      { heading: "Move to" },
      { label: "Top level (Pages)", icon: FileText, action: () => move(null) },
      ...targets.map((p) => ({ label: `${p.icon ? `${p.icon} ` : ""}${p.title || "Untitled"}`, action: () => move(p.id) })),
    ]);
  }

  async function move(parentId) {
    await reorder("pages", [{ id: pageId, parent_id: parentId, position: 9999 }]);
    load();
  }

  function more(e) {
    openMenu(e, [
      { label: "Copy link", icon: Link2, action: () => navigator.clipboard?.writeText(`${location.origin}/page/${pageId}`).then(() => toast("Link copied")) },
      { label: "Move to…", icon: FolderInput, action: () => moveMenu(e) },
      { label: "Export as Markdown", icon: Download, action: exportMarkdown },
      { sep: true },
      { label: "Move to trash", icon: Trash2, danger: true, action: async () => { await trashPage(pageId); go(page?.parent_id ? `/page/${page.parent_id}` : "/notes"); } },
    ]);
  }

  function jump(h) {
    editorRef?.scrollToPos(h.pos);
  }
</script>

<svelte:window onbeforeunload={flush} />

{#if error}
  <div class="missing"><h1>Page not found</h1><p class="muted">{error}</p><a href="/notes">Back to Notes</a></div>
{:else if page}
  <div class="pageview {fontClass}" class:embedded class:focus={app.focus}>
    {#if !embedded && !app.focus}
      <header class="bar">
        <nav class="crumbs" aria-label="Breadcrumb">
          {#if owner}
            {#if owner.kind === "todo"}
              <button type="button" class="crumb" onclick={() => (app.detail = owner.id)}><CircleCheck size={14} /> {owner.label}</button>
            {:else}
              <a class="crumb" href="/class/{owner.id}?tab=notes"><Hash size={14} /> {owner.label}</a>
            {/if}
            <span class="slash">/</span>
          {/if}
          {#each page.path as p (p.id)}
            <a class="crumb" href="/page/{p.id}">{#if p.icon}<span>{p.icon}</span>{/if}{pageById(p.id)?.title || p.title || "Untitled"}</a>
            <span class="slash">/</span>
          {/each}
          <span class="crumb current">{#if meta?.icon}<span>{meta.icon}</span>{/if}{title || "Untitled"}</span>
        </nav>
        <span class="status" class:err={status === "error"}>{status === "saving" ? "Saving…" : status === "error" ? "Not saved, retrying" : "Saved"}</span>
        <button type="button" class="icon-btn" class:on={favorite} aria-label={favorite ? "Remove from favorites" : "Add to favorites"} title="Favorite" onclick={() => updatePage(pageId, { favorite: !favorite })}><Star size={17} fill={favorite ? "var(--yellow)" : "none"} color={favorite ? "var(--yellow)" : undefined} /></button>
        <button type="button" class="icon-btn" aria-label="Page font" title="Font" onclick={fontMenu}><CaseSensitive size={18} /></button>
        <button type="button" class="icon-btn" class:on={fullWidth} aria-label="Full width" title="Full width" onclick={() => setMeta({ full_width: !fullWidth })}><MoveHorizontal size={17} /></button>
        <button type="button" class="icon-btn" aria-label="Focus mode" title="Focus mode  Ctrl+Shift+F" onclick={() => (app.focus = true)}><Maximize2 size={16} /></button>
        <button type="button" class="icon-btn" aria-label="More" onclick={more}><Ellipsis size={18} /></button>
      </header>
    {/if}

    <div class="layout" class:wide={fullWidth}>
      <article class="page">
        {#if !embedded}
          <div class="iconrow">
            {#if meta?.icon}
              <button type="button" class="bigicon" aria-label="Change icon" onclick={(e) => (iconAnchor = e.currentTarget)}>{meta.icon}</button>
            {:else}
              <button type="button" class="addicon" onclick={(e) => (iconAnchor = e.currentTarget)}><SmilePlus size={15} /> Add icon</button>
            {/if}
          </div>
          <textarea class="title" rows="1" placeholder="Untitled" bind:value={title} use:autosize oninput={titleInput} onkeydown={titleKey} aria-label="Page title"></textarea>
        {:else}
          <div class="embedbar">
            <span class="status" class:err={status === "error"}>{status === "saving" ? "Saving…" : "Saved"}</span>
            <a href="/page/{pageId}" class="btn ghost sm"><ArrowUpRight size={14} /> Open as page</a>
          </div>
        {/if}

        {#await editorModule then { default: Editor }}
          <Editor bind:this={editorRef} {pageId} content={page.content} markdown={page.content ? "" : page.body} onchange={changed} onheadings={(h) => (headings = h)} autofocus={!page.content && !page.body && !!title} />
        {/await}

        {#if children.length || page.backlinks.length}
          <footer class="refs">
            {#if children.length}
              <h3>Pages inside</h3>
              {#each children as c (c.id)}
                <a class="ref" href="/page/{c.id}">{#if c.icon}<span>{c.icon}</span>{:else}<FileText size={15} />{/if}{c.title || "Untitled"}</a>
              {/each}
            {/if}
            {#if page.backlinks.length}
              <h3>{page.backlinks.length} backlink{page.backlinks.length === 1 ? "" : "s"}</h3>
              {#each page.backlinks as b (b.id)}
                <a class="ref" href="/page/{b.id}">{#if b.icon}<span>{b.icon}</span>{:else}<ArrowUpRight size={15} />{/if}{b.title || "Untitled"}</a>
              {/each}
            {/if}
          </footer>
        {/if}
      </article>

      {#if headings.length >= 3 && !embedded}
        <aside class="toc" aria-label="Contents">
          {#each headings as h}
            <button type="button" class="toc-item l{h.level}" onclick={() => jump(h)} title={h.text}>{h.text}</button>
          {/each}
        </aside>
      {/if}
    </div>
  </div>
{:else}
  <div class="loading"></div>
{/if}

{#if iconAnchor}
  <EmojiPicker anchor={iconAnchor} onpick={(icon) => setMeta({ icon })} onclose={() => (iconAnchor = null)} allowRemove={!!meta?.icon} />
{/if}

<style>
  .pageview { min-height: 100%; }
  .bar { position: sticky; top: 0; z-index: 10; display: flex; align-items: center; gap: 2px; height: 46px; padding: 0 12px 0 16px; background: var(--bg); }
  .crumbs { display: flex; align-items: center; gap: 2px; flex: 1; min-width: 0; overflow: hidden; }
  .crumb { display: inline-flex; align-items: center; gap: 5px; max-width: 200px; padding: 3px 6px; border-radius: 5px; border: 0; background: none; color: var(--text-2); font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; text-decoration: none; }
  a.crumb:hover, button.crumb:hover { background: var(--hover); text-decoration: none; color: var(--text); }
  .crumb.current { color: var(--text); }
  .slash { color: var(--faint); }
  .status { font-size: 12.5px; color: var(--faint); margin-right: 8px; white-space: nowrap; }
  .status.err { color: var(--p1); }
  .icon-btn.on { color: var(--text); background: var(--hover); }

  .layout { display: flex; justify-content: center; gap: 24px; padding: 0 96px; }
  .page { width: 100%; max-width: 720px; padding: 6vh 0 0; }
  .layout.wide .page { max-width: none; }
  .embedded .layout { padding: 0; justify-content: flex-start; }
  .embedded .page { padding-top: 0; max-width: none; }
  .focus .layout { padding-top: 6vh; }
  .focus .page { max-width: 760px; }
  .focus.font-default :global(.doc) { font-size: 17px; }

  .iconrow { min-height: 34px; display: flex; align-items: flex-end; margin-bottom: 4px; }
  .bigicon { font-size: 64px; line-height: 1; border: 0; background: none; padding: 4px; border-radius: 8px; margin-left: -4px; }
  .bigicon:hover { background: var(--hover); }
  .addicon { display: inline-flex; align-items: center; gap: 6px; border: 0; background: none; color: var(--faint); padding: 4px 6px; border-radius: 5px; font-size: 13.5px; opacity: 0; transition: opacity 120ms; margin-left: -6px; }
  .page:hover .addicon { opacity: 1; }
  .addicon:hover { background: var(--hover); color: var(--text-2); }
  .title {
    display: block; width: 100%; border: 0; outline: none; background: none; resize: none; padding: 0; margin: 0 0 14px;
    font-size: 40px; line-height: 1.2; font-weight: 800; letter-spacing: -0.025em; color: var(--text); overflow: hidden;
  }
  .font-serif .title { font-family: var(--serif); letter-spacing: 0; font-weight: 700; }
  .font-mono .title { font-family: var(--mono); font-size: 34px; letter-spacing: 0; }
  .title::placeholder { color: #45423d; }
  .embedbar { display: flex; align-items: center; justify-content: flex-end; gap: 6px; margin: -6px 0 6px; }

  .refs { margin-top: -24vh; padding: 20px 0 80px; border-top: 1px solid var(--border); }
  .refs h3 { font-size: 12.5px; color: var(--muted); font-weight: 650; margin: 14px 0 4px; }
  .ref { display: flex; align-items: center; gap: 8px; padding: 5px 6px; margin-left: -6px; border-radius: 5px; color: var(--text-2); text-decoration: none; }
  .ref:hover { background: var(--hover); color: var(--text); text-decoration: none; }

  .toc { position: sticky; top: 70px; align-self: flex-start; width: 200px; flex: none; margin-top: 18vh; display: flex; flex-direction: column; gap: 1px; max-height: 70vh; overflow: auto; opacity: 0.6; transition: opacity 150ms; }
  .toc:hover { opacity: 1; }
  .toc-item { border: 0; background: none; text-align: left; padding: 3px 6px; border-radius: 4px; font-size: 12.5px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .toc-item:hover { background: var(--hover); color: var(--text); }
  .toc-item.l2 { padding-left: 16px; }
  .toc-item.l3 { padding-left: 26px; }
  @media (max-width: 1250px) { .toc { display: none; } }
  @media (max-width: 900px) { .layout { padding: 0 32px; } }
  .missing { padding: 80px; }
  .loading { height: 100%; }
</style>
