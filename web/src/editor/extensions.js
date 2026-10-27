// The editor's building blocks, assembled for one page.

import { Extension } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { Placeholder } from "@tiptap/extensions";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import { TableKit } from "@tiptap/extension-table";
import Image from "@tiptap/extension-image";
import { BackgroundColor, Color, TextStyle } from "@tiptap/extension-text-style";
import { Details, DetailsContent, DetailsSummary } from "@tiptap/extension-details";
import { Markdown } from "@tiptap/markdown";
import Suggestion from "@tiptap/suggestion";
import { PluginKey } from "@tiptap/pm/state";
import { BlockColor, BlockKeys, Callout, PageBlock, makeMention } from "./nodes.js";
import { popupRenderer } from "./popup.js";
import { app, createPage } from "../lib/store.svelte.js";
import { upload } from "../lib/api.js";
import { fail } from "../lib/toast.svelte.js";
import { parseWhen } from "../lib/nlp.js";
import * as D from "../lib/dates.js";

/** Asks for files and uploads them; resolves to the upload results. */
export function pickFiles(accept = "") {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    if (accept) input.accept = accept;
    input.onchange = async () => {
      const out = [];
      for (const f of input.files) {
        try {
          out.push(await upload(f));
        } catch (err) {
          fail(err);
        }
      }
      resolve(out);
    };
    input.click();
  });
}

/** Inserts uploaded files: images as images, anything else as a link. */
export function insertFiles(editor, files, pos) {
  const content = files.flatMap((f) =>
    f.mime.startsWith("image/") && f.mime !== "image/svg+xml"
      ? [{ type: "image", attrs: { src: f.url, alt: f.name } }]
      : [{ type: "paragraph", content: [{ type: "text", text: `📎 ${f.name}`, marks: [{ type: "link", attrs: { href: f.url } }] }] }],
  );
  if (!content.length) return;
  if (pos == null) editor.chain().focus().insertContent(content).run();
  else editor.chain().focus().insertContentAt(pos, content).run();
}

export function slashItems(pageId) {
  const block = (fn) => (editor, range) => fn(editor.chain().focus().deleteRange(range)).run();
  return [
    { title: "Text", subtitle: "Plain writing", icon: "Aa", keys: "paragraph p", group: "Basic blocks", run: block((c) => c.setParagraph()) },
    { title: "Heading 1", subtitle: "Big section heading", icon: "H1", hint: "#", keys: "h1 title", group: "Basic blocks", run: block((c) => c.setNode("heading", { level: 1 })) },
    { title: "Heading 2", subtitle: "Medium section heading", icon: "H2", hint: "##", keys: "h2 subtitle", group: "Basic blocks", run: block((c) => c.setNode("heading", { level: 2 })) },
    { title: "Heading 3", subtitle: "Small section heading", icon: "H3", hint: "###", keys: "h3", group: "Basic blocks", run: block((c) => c.setNode("heading", { level: 3 })) },
    { title: "Bulleted list", subtitle: "A simple list", icon: "•", hint: "-", keys: "ul bullet unordered", group: "Basic blocks", run: block((c) => c.toggleBulletList()) },
    { title: "Numbered list", subtitle: "A list with numbers", icon: "1.", hint: "1.", keys: "ol ordered number", group: "Basic blocks", run: block((c) => c.toggleOrderedList()) },
    { title: "To-do list", subtitle: "Check things off", icon: "☐", hint: "[]", keys: "todo checkbox task check", group: "Basic blocks", run: block((c) => c.toggleTaskList()) },
    { title: "Toggle list", subtitle: "Hide content inside", icon: "▸", hint: ">", keys: "toggle details collapse", group: "Basic blocks", run: block((c) => c.setDetails()) },
    { title: "Quote", subtitle: "Capture a quote", icon: "❝", hint: '"', keys: "blockquote", group: "Basic blocks", run: block((c) => c.toggleBlockquote()) },
    { title: "Callout", subtitle: "Make writing stand out", icon: "💡", keys: "note info tip warning", group: "Basic blocks", run: block((c) => c.setCallout()) },
    { title: "Divider", subtitle: "Separate sections", icon: "—", hint: "---", keys: "hr rule line separator", group: "Basic blocks", run: block((c) => c.setHorizontalRule()) },
    { title: "Code", subtitle: "A code snippet", icon: "</>", hint: "```", keys: "codeblock pre", group: "Basic blocks", run: block((c) => c.toggleCodeBlock()) },
    { title: "Table", subtitle: "Rows and columns", icon: "⊞", keys: "grid spreadsheet", group: "Basic blocks", run: block((c) => c.insertTable({ rows: 3, cols: 3, withHeaderRow: true })) },
    {
      title: "Page",
      subtitle: "Embed a sub-page",
      icon: "📄",
      keys: "subpage new page",
      group: "Pages",
      run: async (editor, range) => {
        editor.chain().focus().deleteRange(range).run();
        const p = await createPage({ parent_id: pageId });
        if (p) editor.chain().focus().insertContent({ type: "pageBlock", attrs: { id: p.id } }).run();
      },
    },
    { title: "Link to page", subtitle: "Mention a page", icon: "↗", hint: "[[", keys: "mention link", group: "Pages", run: block((c) => c.insertContent("@")) },
    {
      title: "Image",
      subtitle: "Upload or paste one",
      icon: "🖼",
      keys: "picture photo upload",
      group: "Media",
      run: async (editor, range) => {
        editor.chain().focus().deleteRange(range).run();
        insertFiles(editor, await pickFiles("image/*"));
      },
    },
    {
      title: "File",
      subtitle: "Attach any file",
      icon: "📎",
      keys: "attachment upload pdf",
      group: "Media",
      run: async (editor, range) => {
        editor.chain().focus().deleteRange(range).run();
        insertFiles(editor, await pickFiles());
      },
    },
    { title: "Mention", subtitle: "A page, task, class or date", icon: "@", hint: "@", keys: "person date reminder", group: "Inline", run: block((c) => c.insertContent("@")) },
    { title: "Today's date", subtitle: D.long(D.today()), icon: "📅", keys: "date now today", group: "Inline", run: block((c) => c.insertContent([{ type: "mention", attrs: { id: `date:${D.today()}`, label: D.today() } }, { type: "text", text: " " }])) },
  ];
}

function filter(items, query) {
  const q = query.toLowerCase().trim();
  if (!q) return items;
  return items.filter((it) => `${it.title} ${it.keys || ""}`.toLowerCase().includes(q));
}

const SlashCommand = Extension.create({
  name: "slashCommand",
  addOptions() {
    return { pageId: null };
  },
  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        pluginKey: new PluginKey("slash"),
        char: "/",
        startOfLine: false,
        allow: ({ state, range }) => {
          // Not inside code, and only where a new word starts.
          const $from = state.doc.resolve(range.from);
          return $from.parent.type.name !== "codeBlock";
        },
        items: ({ query }) => filter(slashItems(this.options.pageId), query).slice(0, 30),
        command: ({ editor, range, props }) => props.run(editor, range),
        render: popupRenderer("No matching blocks"),
      }),
    ];
  },
});

function mentionItems(query, { pagesOnly = false, pageId = null } = {}) {
  const q = query.toLowerCase().trim();
  const out = [];
  if (!pagesOnly) {
    const when = q ? parseWhen(q) : null;
    const dates = when?.due_date ? [when.due_date] : q ? [] : [D.today(), D.addDays(D.today(), 1)];
    for (const d of dates) out.push({ id: `date:${d}`, label: d, title: D.long(d), icon: "📅", group: "Date" });
  }
  const pages = app.pages.filter((p) => p.id !== pageId && (p.title || "untitled").toLowerCase().includes(q)).slice(0, pagesOnly ? 10 : 6);
  for (const p of pages) out.push({ id: `page:${p.id}`, label: p.title || "Untitled", title: p.title || "Untitled", icon: p.icon || "📄", group: "Pages" });
  if (pagesOnly && q) out.push({ create: query.trim(), title: `New page “${query.trim()}”`, icon: "+", group: "Pages" });
  if (!pagesOnly) {
    const tasks = app.todos.filter((t) => !t.done && t.title.toLowerCase().includes(q)).slice(0, 5);
    for (const t of tasks) out.push({ id: `todo:${t.id}`, label: t.title, title: t.title, subtitle: t.class_name || "Inbox", icon: "◯", group: "Tasks" });
    const classes = app.classes.filter((c) => c.name.toLowerCase().includes(q)).slice(0, 4);
    for (const c of classes) out.push({ id: `class:${c.id}`, label: c.name, title: c.name, icon: c.emoji || "#", group: "Classes" });
  }
  return out;
}

async function insertMention(editor, range, item) {
  let { id, label } = item;
  if (item.create) {
    const p = await createPage({ title: item.create });
    if (!p) return;
    id = `page:${p.id}`;
    label = p.title;
  }
  editor
    .chain()
    .focus()
    .insertContentAt(range, [{ type: "mention", attrs: { id, label } }, { type: "text", text: " " }])
    .run();
}

export function buildExtensions({ pageId }) {
  return [
    StarterKit.configure({
      heading: { levels: [1, 2, 3] },
      link: { openOnClick: false, autolink: true, defaultProtocol: "https", HTMLAttributes: { rel: "noopener noreferrer", target: null } },
      dropcursor: { color: "#7aa7ff", width: 2 },
      trailingNode: true,
    }),
    Placeholder.configure({
      showOnlyCurrent: true,
      includeChildren: true,
      placeholder: ({ node, editor }) => {
        if (node.type.name === "heading") return `Heading ${node.attrs.level}`;
        if (node.type.name === "detailsSummary") return "Toggle";
        if (node.type.name !== "paragraph") return "";
        const doc = editor.state.doc;
        if (doc.childCount === 1 && doc.firstChild.childCount === 0) return "Write something, or press ‘/’ for commands…";
        return "Press ‘/’ for commands";
      },
    }),
    TaskList,
    TaskItem.configure({ nested: true }),
    TableKit.configure({ table: { resizable: true, cellMinWidth: 80 } }),
    Image.configure({ allowBase64: false }),
    Details.configure({ persist: true, HTMLAttributes: { class: "toggle" } }),
    DetailsSummary,
    DetailsContent,
    Markdown,
    TextStyle,
    Color,
    BackgroundColor,
    Callout,
    PageBlock,
    BlockColor,
    BlockKeys,
    SlashCommand.configure({ pageId }),
    makeMention([
      {
        char: "@",
        pluginKey: new PluginKey("mention-at"),
        items: ({ query }) => mentionItems(query, { pageId }),
        command: ({ editor, range, props }) => insertMention(editor, range, props),
        render: popupRenderer("Nothing found"),
      },
      {
        char: "[[",
        pluginKey: new PluginKey("mention-link"),
        items: ({ query }) => mentionItems(query, { pagesOnly: true, pageId }),
        command: ({ editor, range, props }) => insertMention(editor, range, props),
        render: popupRenderer("Type a page title"),
      },
    ]),
  ];
}
