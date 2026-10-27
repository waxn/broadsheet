<script>
  // A Notion-style block editor: slash menu, drag handles, a formatting bubble, mentions,
  // tables, toggles, callouts and uploads.
  import { onDestroy, onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import { NodeSelection, TextSelection } from "@tiptap/pm/state";
  import { GripVertical, Plus, Bold, Italic, Underline, Strikethrough, Code, Link, ChevronDown, Baseline, ExternalLink, Trash2, Copy, Palette } from "@lucide/svelte";
  import { buildExtensions, insertFiles } from "./extensions.js";
  import "./editor.css";
  import { collectLinks, mentionTarget } from "./nodes.js";
  import { upload } from "../lib/api.js";
  import { app } from "../lib/store.svelte.js";
  import { go } from "../lib/router.svelte.js";
  import { openMenu } from "../lib/menu.svelte.js";
  import { fail } from "../lib/toast.svelte.js";

  let { pageId, content = null, markdown = "", onchange, onheadings, editable = true, autofocus = false } = $props();

  const NOTE_COLORS = [
    ["Default", null],
    ["Gray", "gray"],
    ["Brown", "brown"],
    ["Orange", "orange"],
    ["Yellow", "yellow"],
    ["Green", "green"],
    ["Blue", "blue"],
    ["Purple", "purple"],
    ["Pink", "pink"],
    ["Red", "red"],
  ];
  const INK = { gray: "#9b9a97", brown: "#ba856f", orange: "#d9824a", yellow: "#d4a84a", green: "#5aa97a", blue: "#6b93d6", purple: "#a77ad8", pink: "#d6699f", red: "#e0605c" };

  let host = $state();
  let wrap = $state();
  let editor = $state(null);
  let tick = $state(0); // bumps on every transaction so the toolbars re-read editor state
  let handle = $state(null); // { top, pos, node }
  let bubble = $state(null); // { top, left }
  let linkMode = $state(false);
  let linkValue = $state("");
  let tableBar = $state(null);

  onMount(() => {
    const ed = new Editor({
      element: host,
      extensions: buildExtensions({ pageId }),
      content: content ?? (markdown || ""),
      contentType: content ? "json" : markdown ? "markdown" : "json",
      editable,
      autofocus: autofocus ? "end" : false,
      editorProps: {
        attributes: { class: "doc", spellcheck: "true" },
        handlePaste(view, event) {
          const files = [...(event.clipboardData?.files || [])];
          if (!files.length) return false;
          event.preventDefault();
          uploadAll(files).then((done) => insertFiles(ed, done));
          return true;
        },
        handleDrop(view, event, _slice, moved) {
          const files = [...(event.dataTransfer?.files || [])];
          if (moved || !files.length) return false;
          event.preventDefault();
          const at = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos;
          uploadAll(files).then((done) => insertFiles(ed, done, at));
          return true;
        },
        handleKeyDown(view, event) {
          if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
            event.preventDefault();
            if (!view.state.selection.empty) openLink();
            return true;
          }
          return false;
        },
      },
      onTransaction: () => {
        tick++;
        updateBubble();
      },
      onUpdate: ({ editor: e }) => {
        onchange?.(snapshot(e));
        emitHeadings(e);
      },
      onBlur: () => setTimeout(() => {
        if (!wrap?.contains(document.activeElement)) bubble = null;
      }, 120),
    });
    editor = ed;
    emitHeadings(ed);
    // Old pages were markdown: save them once in the new format.
    if (!content && markdown) onchange?.(snapshot(ed));
  });

  onDestroy(() => editor?.destroy());

  export function getMarkdown() {
    return editor?.getMarkdown?.() ?? editor?.getText() ?? "";
  }

  export function focus(where = "start") {
    if (!editor) return;
    // Focus right away (TipTap's focus command waits a frame, and fast typing would be lost).
    editor.view.focus();
    const sel = where === "start" ? TextSelection.atStart(editor.state.doc) : TextSelection.atEnd(editor.state.doc);
    editor.view.dispatch(editor.state.tr.setSelection(sel).scrollIntoView());
  }

  function snapshot(e) {
    return { content: e.getJSON(), text: e.getText({ blockSeparator: "\n" }), links: collectLinks(e.state.doc) };
  }

  function emitHeadings(e) {
    if (!onheadings) return;
    const list = [];
    e.state.doc.forEach((node, offset) => {
      if (node.type.name === "heading" && node.textContent.trim()) list.push({ level: node.attrs.level, text: node.textContent, pos: offset });
    });
    onheadings(list);
  }

  export function scrollToPos(pos) {
    const dom = editor?.view.nodeDOM(pos);
    dom?.scrollIntoView?.({ behavior: "smooth", block: "start" });
    editor?.commands.setTextSelection(pos + 1);
  }

  async function uploadAll(files) {
    const out = [];
    for (const f of files) {
      try {
        out.push(await upload(f));
      } catch (err) {
        fail(err);
      }
    }
    return out;
  }

  // ----- drag handle -----

  function blockAt(clientY) {
    const view = editor.view;
    const rect = view.dom.getBoundingClientRect();
    const hit = view.posAtCoords({ left: rect.left + Math.min(60, rect.width / 2), top: clientY });
    if (!hit) return null;
    const doc = view.state.doc;
    const pos = hit.inside >= 0 ? hit.inside : hit.pos;
    const rp = doc.resolve(Math.min(pos, doc.content.size));
    let start;
    if (rp.depth === 0) {
      start = rp.nodeAfter ? pos : rp.nodeBefore ? pos - rp.nodeBefore.nodeSize : null;
    } else {
      let depth = 1;
      for (let d = rp.depth; d > 0; d--) {
        if (["listItem", "taskItem"].includes(rp.node(d).type.name)) {
          depth = d;
          break;
        }
      }
      start = rp.before(depth);
    }
    if (start == null) return null;
    const node = doc.nodeAt(start);
    const dom = view.nodeDOM(start);
    if (!node || !(dom instanceof HTMLElement)) return null;
    return { pos: start, node, dom };
  }

  function mousemove(e) {
    if (!editor || !editable || e.target.closest?.(".handle")) return;
    const b = blockAt(e.clientY);
    if (!b) return;
    const wr = wrap.getBoundingClientRect();
    const r = b.dom.getBoundingClientRect();
    const style = getComputedStyle(b.dom);
    const line = parseFloat(style.lineHeight) || 24;
    const pad = parseFloat(style.paddingTop) || 0;
    handle = { top: r.top - wr.top + pad + Math.max(0, (Math.min(line, 32) - 24) / 2), pos: b.pos, node: b.node };
  }

  function dragStart(e) {
    if (!handle) return;
    const view = editor.view;
    const sel = NodeSelection.create(view.state.doc, handle.pos);
    view.dispatch(view.state.tr.setSelection(sel));
    view.dragging = { slice: sel.content(), move: true };
    const dom = view.nodeDOM(handle.pos);
    e.dataTransfer.effectAllowed = "copyMove";
    e.dataTransfer.setData("text/plain", handle.node.textContent);
    if (dom instanceof HTMLElement) e.dataTransfer.setDragImage(dom, 0, 0);
  }

  function addBelow() {
    if (!handle) return;
    const end = handle.pos + handle.node.nodeSize;
    const isItem = ["listItem", "taskItem"].includes(handle.node.type.name);
    editor
      .chain()
      .insertContentAt(end, isItem ? { type: handle.node.type.name, content: [{ type: "paragraph" }] } : { type: "paragraph" })
      .setTextSelection(end + (isItem ? 2 : 1))
      .insertContent(isItem ? "" : "/")
      .focus()
      .run();
  }

  function selectInside(pos) {
    const tr = editor.state.tr.setSelection(TextSelection.near(editor.state.doc.resolve(pos + 1)));
    editor.view.dispatch(tr);
  }

  function blockMenu(e) {
    if (!handle) return;
    const { pos, node } = handle;
    const turn = (fn) => () => {
      selectInside(pos);
      fn(editor.chain().focus()).run();
    };
    const colorable = node.type.spec.attrs?.color !== undefined || ["paragraph", "heading", "blockquote", "callout", "listItem", "taskItem"].includes(node.type.name);
    openMenu(e, [
      { heading: "Turn into" },
      { label: "Text", action: turn((c) => c.setParagraph()) },
      { label: "Heading 1", action: turn((c) => c.setNode("heading", { level: 1 })) },
      { label: "Heading 2", action: turn((c) => c.setNode("heading", { level: 2 })) },
      { label: "Heading 3", action: turn((c) => c.setNode("heading", { level: 3 })) },
      { label: "Bulleted list", action: turn((c) => c.toggleBulletList()) },
      { label: "Numbered list", action: turn((c) => c.toggleOrderedList()) },
      { label: "To-do list", action: turn((c) => c.toggleTaskList()) },
      { label: "Quote", action: turn((c) => c.toggleBlockquote()) },
      { label: "Callout", action: turn((c) => c.setCallout()) },
      { label: "Code", action: turn((c) => c.toggleCodeBlock()) },
      { sep: true },
      ...(colorable ? [{ label: "Colour…", icon: Palette, action: () => colorMenu(e, pos) }] : []),
      { label: "Duplicate", icon: Copy, hint: "Ctrl D", action: () => editor.chain().insertContentAt(pos + node.nodeSize, node.toJSON()).run() },
      { label: "Delete", icon: Trash2, danger: true, hint: "Del", action: () => editor.view.dispatch(editor.state.tr.delete(pos, pos + node.nodeSize)) },
    ]);
  }

  function colorMenu(e, pos) {
    const set = (color) => () => {
      const node = editor.state.doc.nodeAt(pos);
      if (!node) return;
      editor.view.dispatch(editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, color }));
    };
    openMenu({ x: e.clientX ?? 100, y: e.clientY ?? 100 }, [
      { heading: "Text" },
      ...NOTE_COLORS.map(([label, c]) => ({ label, color: c ? INK[c] : undefined, icon: Baseline, action: set(c) })),
      { heading: "Background" },
      ...NOTE_COLORS.slice(1).map(([label, c]) => ({ label: `${label} background`, color: INK[c], icon: Palette, action: set(`${c}_bg`) })),
    ]);
  }

  // ----- formatting bubble -----

  function updateBubble() {
    const e = editor;
    if (!e || !wrap) return;
    const { selection } = e.state;
    const inCode = e.isActive("codeBlock");
    if (selection.empty || selection instanceof NodeSelection || inCode || !e.isFocused) {
      if (!linkMode) bubble = null;
    } else {
      const start = e.view.coordsAtPos(selection.from);
      const end = e.view.coordsAtPos(selection.to);
      const wr = wrap.getBoundingClientRect();
      bubble = { top: Math.min(start.top, end.top) - wr.top - 46, left: Math.max(0, (start.left + end.left) / 2 - wr.left) };
    }
    // Table controls sit above the table the cursor is in.
    if (e.isActive("table")) {
      const dom = e.view.domAtPos(selection.from).node;
      const table = (dom.nodeType === 1 ? dom : dom.parentElement)?.closest("table");
      if (table) {
        const r = table.getBoundingClientRect();
        const wr = wrap.getBoundingClientRect();
        tableBar = { top: r.top - wr.top - 38, left: r.left - wr.left };
      }
    } else tableBar = null;
  }

  function openLink() {
    linkValue = editor.getAttributes("link").href || "";
    linkMode = true;
    updateBubble();
  }

  function applyLink(e) {
    e?.preventDefault();
    const href = linkValue.trim();
    const chain = editor.chain().focus().extendMarkRange("link");
    if (href) chain.setLink({ href: /^(https?:|mailto:|\/)/i.test(href) ? href : `https://${href}` }).run();
    else chain.unsetLink().run();
    linkMode = false;
  }

  function turnIntoMenu(e) {
    const c = () => editor.chain().focus();
    openMenu(e, [
      { label: "Text", checked: editor.isActive("paragraph") && !editor.isActive("bulletList") && !editor.isActive("orderedList") && !editor.isActive("taskList"), action: () => c().setParagraph().run() },
      { label: "Heading 1", checked: editor.isActive("heading", { level: 1 }), action: () => c().setNode("heading", { level: 1 }).run() },
      { label: "Heading 2", checked: editor.isActive("heading", { level: 2 }), action: () => c().setNode("heading", { level: 2 }).run() },
      { label: "Heading 3", checked: editor.isActive("heading", { level: 3 }), action: () => c().setNode("heading", { level: 3 }).run() },
      { label: "Bulleted list", checked: editor.isActive("bulletList"), action: () => c().toggleBulletList().run() },
      { label: "Numbered list", checked: editor.isActive("orderedList"), action: () => c().toggleOrderedList().run() },
      { label: "To-do list", checked: editor.isActive("taskList"), action: () => c().toggleTaskList().run() },
      { label: "Quote", checked: editor.isActive("blockquote"), action: () => c().toggleBlockquote().run() },
      { label: "Code", action: () => c().toggleCodeBlock().run() },
    ]);
  }

  function inlineColorMenu(e) {
    const c = () => editor.chain().focus();
    openMenu(e, [
      { heading: "Text colour" },
      { label: "Default", icon: Baseline, action: () => c().unsetColor().run() },
      ...NOTE_COLORS.slice(1).map(([label, k]) => ({ label, icon: Baseline, color: INK[k], action: () => c().setColor(INK[k]).run() })),
      { heading: "Highlight" },
      { label: "None", icon: Palette, action: () => c().unsetBackgroundColor().run() },
      ...NOTE_COLORS.slice(1).map(([label, k]) => ({ label: `${label} highlight`, icon: Palette, color: INK[k], action: () => c().setBackgroundColor(`${INK[k]}38`).run() })),
    ]);
  }

  const blockLabel = $derived.by(() => {
    tick;
    if (!editor) return "Text";
    if (editor.isActive("heading", { level: 1 })) return "Heading 1";
    if (editor.isActive("heading", { level: 2 })) return "Heading 2";
    if (editor.isActive("heading", { level: 3 })) return "Heading 3";
    if (editor.isActive("taskList")) return "To-do";
    if (editor.isActive("bulletList")) return "Bulleted";
    if (editor.isActive("orderedList")) return "Numbered";
    if (editor.isActive("blockquote")) return "Quote";
    return "Text";
  });
  const active = (name, attrs) => {
    tick;
    return editor?.isActive(name, attrs);
  };

  // ----- clicks on mentions, sub-pages and links -----

  function click(e) {
    const m = e.target.closest("[data-mention]");
    if (m) {
      e.preventDefault();
      const { kind, value } = mentionTarget(m.dataset.mention);
      if (kind === "page") go(`/page/${value}`);
      if (kind === "class") go(`/class/${value}`);
      if (kind === "date") go(`/calendar?date=${value}&view=day`);
      if (kind === "todo") {
        if (app.todos.some((t) => t.id === Number(value))) app.detail = Number(value);
      }
      return;
    }
    const pb = e.target.closest(".page-block[data-href]");
    if (pb) {
      e.preventDefault();
      go(pb.dataset.href);
      return;
    }
    const a = e.target.closest(".ProseMirror a[href]");
    if (a) {
      e.preventDefault();
      if (e.ctrlKey || e.metaKey || !editable) window.open(a.href, "_blank", "noopener");
    }
  }

  const linkHref = $derived.by(() => {
    tick;
    return editor?.isActive("link") ? editor.getAttributes("link").href : null;
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="editor-wrap" bind:this={wrap} onmousemove={mousemove} onmouseleave={() => (handle = null)} onclick={click}>
  {#if handle && editable}
    <div class="handle" style:top="{handle.top}px">
      <button type="button" class="hbtn" aria-label="Add a block below" title="Click to add below" onclick={addBelow}><Plus size={17} /></button>
      <button type="button" class="hbtn grip" aria-label="Drag to move, click for options" title="Drag to move · Click for options" draggable="true" ondragstart={dragStart} onclick={blockMenu}><GripVertical size={17} /></button>
    </div>
  {/if}

  {#if bubble && editor}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="bubble" style:top="{bubble.top}px" style:left="{bubble.left}px" onmousedown={(e) => e.target.tagName !== "INPUT" && e.preventDefault()}>
      {#if linkMode}
        <form class="linkform" onsubmit={applyLink}>
          <!-- svelte-ignore a11y_autofocus -->
          <input bind:value={linkValue} placeholder="Paste a link…" autofocus onkeydown={(e) => e.key === "Escape" && ((linkMode = false), editor.commands.focus())} />
          <button class="bb">Apply</button>
        </form>
      {:else}
        <button type="button" class="bb wide" onclick={turnIntoMenu}>{blockLabel} <ChevronDown size={13} /></button>
        <span class="sep"></span>
        <button type="button" class="bb" class:on={active("bold")} aria-label="Bold" onclick={() => editor.chain().focus().toggleBold().run()}><Bold size={15} /></button>
        <button type="button" class="bb" class:on={active("italic")} aria-label="Italic" onclick={() => editor.chain().focus().toggleItalic().run()}><Italic size={15} /></button>
        <button type="button" class="bb" class:on={active("underline")} aria-label="Underline" onclick={() => editor.chain().focus().toggleUnderline().run()}><Underline size={15} /></button>
        <button type="button" class="bb" class:on={active("strike")} aria-label="Strikethrough" onclick={() => editor.chain().focus().toggleStrike().run()}><Strikethrough size={15} /></button>
        <button type="button" class="bb" class:on={active("code")} aria-label="Code" onclick={() => editor.chain().focus().toggleCode().run()}><Code size={15} /></button>
        <button type="button" class="bb" class:on={active("link")} aria-label="Link" onclick={openLink}><Link size={15} /></button>
        {#if linkHref}<button type="button" class="bb" aria-label="Open link" onclick={() => window.open(linkHref, "_blank", "noopener")}><ExternalLink size={15} /></button>{/if}
        <span class="sep"></span>
        <button type="button" class="bb" aria-label="Colour" onclick={inlineColorMenu}><Baseline size={15} /><ChevronDown size={12} /></button>
      {/if}
    </div>
  {/if}

  {#if tableBar && editor && editable}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="tablebar" style:top="{tableBar.top}px" style:left="{tableBar.left}px" onmousedown={(e) => e.preventDefault()}>
      <button type="button" class="bb" onclick={() => editor.chain().focus().addRowAfter().run()}>+ Row</button>
      <button type="button" class="bb" onclick={() => editor.chain().focus().addColumnAfter().run()}>+ Column</button>
      <button type="button" class="bb" onclick={() => editor.chain().focus().deleteRow().run()}>− Row</button>
      <button type="button" class="bb" onclick={() => editor.chain().focus().deleteColumn().run()}>− Column</button>
      <button type="button" class="bb" onclick={() => editor.chain().focus().toggleHeaderRow().run()}>Header</button>
      <span class="sep"></span>
      <button type="button" class="bb danger" aria-label="Delete table" onclick={() => editor.chain().focus().deleteTable().run()}><Trash2 size={14} /></button>
    </div>
  {/if}

  <div bind:this={host} class="host"></div>
</div>

<style>
  .editor-wrap { position: relative; padding-left: 56px; margin-left: -56px; }
  .handle { position: absolute; left: 6px; display: flex; gap: 0; z-index: 3; animation: fade 120ms; }
  .hbtn { width: 24px; height: 24px; display: grid; place-items: center; border: 0; padding: 0; background: none; color: var(--faint); border-radius: 4px; }
  .hbtn:hover { background: var(--hover); color: var(--text-2); }
  .grip { cursor: grab; }
  .bubble, .tablebar {
    position: absolute; z-index: 30; display: flex; align-items: center; gap: 1px; padding: 3px; transform: translateX(-50%);
    background: var(--raised); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: var(--shadow); white-space: nowrap;
  }
  .tablebar { transform: none; }
  .bb { display: inline-flex; align-items: center; gap: 3px; height: 28px; min-width: 28px; padding: 0 6px; border: 0; background: none; border-radius: 5px; color: var(--text-2); font-size: 13px; font-weight: 550; justify-content: center; }
  .bb:hover { background: var(--active); color: var(--text); }
  .bb.on { color: var(--link); }
  .bb.danger:hover { color: var(--p1); }
  .bb.wide { padding: 0 8px; }
  .sep { width: 1px; height: 18px; background: var(--border-strong); margin: 0 3px; }
  .linkform { display: flex; gap: 4px; }
  .linkform input { width: 260px; height: 28px; border: 0; background: var(--bg); border-radius: 5px; padding: 0 8px; outline: none; }
</style>
