// Custom blocks: callouts, links to sub-pages, mentions that stay in sync with titles, block
// colours, and Notion's Backspace-at-start behaviour.

import { Extension, Node, mergeAttributes } from "@tiptap/core";
import Mention from "@tiptap/extension-mention";
import { app } from "../lib/store.svelte.js";
import * as D from "../lib/dates.js";

export const Callout = Node.create({
  name: "callout",
  group: "block",
  content: "block+",
  defining: true,
  addAttributes() {
    return { emoji: { default: "💡" } };
  },
  parseHTML() {
    return [{ tag: "div[data-callout]", getAttrs: (el) => ({ emoji: el.getAttribute("data-emoji") || "💡" }) }];
  },
  renderHTML({ node, HTMLAttributes }) {
    return [
      "div",
      mergeAttributes(HTMLAttributes, { "data-callout": "", "data-emoji": node.attrs.emoji, class: "callout" }),
      ["span", { class: "callout-emoji", contenteditable: "false" }, node.attrs.emoji],
      ["div", { class: "callout-body" }, 0],
    ];
  },
  addCommands() {
    return {
      setCallout:
        (attrs = {}) =>
        ({ commands }) =>
          commands.wrapIn(this.name, attrs),
    };
  },
});

/** A block that links to a sub-page, showing its current icon and title. */
export const PageBlock = Node.create({
  name: "pageBlock",
  group: "block",
  atom: true,
  draggable: true,
  selectable: true,
  addAttributes() {
    return { id: { default: null, parseHTML: (el) => Number(el.getAttribute("data-page")) || null } };
  },
  parseHTML() {
    return [{ tag: "div[data-page]" }];
  },
  renderHTML({ node }) {
    const p = app.pages.find((x) => x.id === node.attrs.id);
    return [
      "div",
      { "data-page": node.attrs.id, class: `page-block${p ? "" : " missing"}`, "data-href": `/page/${node.attrs.id}` },
      ["span", { class: "pb-icon" }, p?.icon || "📄"],
      ["span", { class: "pb-title" }, p ? p.title || "Untitled" : "Deleted page"],
    ];
  },
});

/** Text and background colours on blocks, set from the block menu. */
export const BlockColor = Extension.create({
  name: "blockColor",
  addGlobalAttributes() {
    return [
      {
        types: ["paragraph", "heading", "blockquote", "callout", "listItem", "taskItem"],
        attributes: {
          color: {
            default: null,
            parseHTML: (el) => el.getAttribute("data-color"),
            renderHTML: (attrs) => (attrs.color ? { "data-color": attrs.color } : {}),
          },
        },
      },
    ];
  },
});

/** Backspace at the start of a heading, quote or callout turns it back into plain text. */
export const BlockKeys = Extension.create({
  name: "blockKeys",
  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { $from, empty } = editor.state.selection;
        if (!empty || $from.parentOffset !== 0) return false;
        if ($from.parent.type.name === "heading") return editor.commands.setParagraph();
        if ($from.parent.type.name === "paragraph" && $from.depth >= 2) {
          const wrapper = $from.node($from.depth - 1);
          if (wrapper.type.name === "blockquote" && $from.index($from.depth - 1) === 0) return editor.commands.lift("blockquote");
          if (wrapper.type.name === "callout" && $from.index($from.depth - 1) === 0) return editor.commands.lift("callout");
        }
        return false;
      },
      "Mod-Alt-0": ({ editor }) => editor.commands.setParagraph(),
    };
  },
});

// ----- mentions -----

/** Ids are "kind:value": page:12, todo:5, class:2, date:2026-10-30. */
export function mentionTarget(id) {
  const [kind, ...rest] = String(id).split(":");
  return { kind, value: rest.join(":") };
}

function mentionLabel(id, fallback) {
  const { kind, value } = mentionTarget(id);
  const n = Number(value);
  if (kind === "page") {
    const p = app.pages.find((x) => x.id === n);
    return p ? `${p.icon ? `${p.icon} ` : ""}${p.title || "Untitled"}` : fallback;
  }
  if (kind === "todo") return app.todos.find((x) => x.id === n)?.title ?? fallback;
  if (kind === "class") return app.classes.find((x) => x.id === n)?.name ?? fallback;
  if (kind === "date") return `${D.label(value)}${D.label(value).includes(" ") ? "" : `, ${D.MONTH_SHORT[D.parse(value).getMonth()]} ${Number(value.slice(8))}`}`;
  return fallback;
}

export function makeMention(suggestions) {
  return Mention.extend({
    renderHTML({ node, HTMLAttributes }) {
      const { kind } = mentionTarget(node.attrs.id);
      const label = mentionLabel(node.attrs.id, node.attrs.label);
      const prefix = kind === "page" || kind === "date" ? "" : kind === "class" ? "#" : "";
      return ["span", mergeAttributes(HTMLAttributes, { class: `mention mention-${kind}`, "data-mention": node.attrs.id }), `${prefix}${label}`];
    },
    renderText({ node }) {
      return mentionLabel(node.attrs.id, node.attrs.label);
    },
  }).configure({ deleteTriggerWithBackspace: true, suggestions });
}

/** Every page, task and class a document points at, for backlinks. */
export function collectLinks(doc) {
  const out = new Map();
  doc.descendants((node) => {
    if (node.type.name === "mention") {
      const { kind, value } = mentionTarget(node.attrs.id);
      if (kind === "page" || kind === "todo" || kind === "class") out.set(`${kind}:${value}`, { kind, id: Number(value) });
    } else if (node.type.name === "pageBlock" && node.attrs.id) {
      out.set(`page:${node.attrs.id}`, { kind: "page", id: node.attrs.id });
    }
  });
  return [...out.values()];
}
