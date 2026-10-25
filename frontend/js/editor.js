// Block editor. The DOM is the source of truth while editing; markdown is
// produced from it on save and rebuilt from markdown on load.

import { blocksToMarkdown, inlineFromNode, inlineToHtml, markdownToBlocks } from "./md.js";
import { esc, showMenu } from "./ui.js";

const PLACEHOLDER = {
  p: "Type something, or start with “## ” and a space",
  h1: "Heading 1",
  h2: "Heading 2",
  h3: "Heading 3",
  ul: "List item",
  ol: "List item",
  todo: "To-do",
  quote: "Quote",
};

const MENU = [
  ["p", "Text"],
  ["h1", "Heading 1"],
  ["h2", "Heading 2"],
  ["h3", "Heading 3"],
  ["ul", "Bullet list"],
  ["ol", "Numbered list"],
  ["todo", "To-do"],
  ["quote", "Quote"],
  ["code", "Code block"],
  ["hr", "Divider"],
];

// Markdown typed at the start of a paragraph, followed by a space, turns it into that block.
const SHORTCUTS = [
  [/^#$/, "h1"],
  [/^##$/, "h2"],
  [/^###$/, "h3"],
  [/^[-*+]$/, "ul"],
  [/^\d+\.$/, "ol"],
  [/^>$/, "quote"],
  [/^\[ ?\]$/, "todo", false],
  [/^\[[xX]\]$/, "todo", true],
];

const isSafeLink = (url) => /^(https?:\/\/|mailto:)/i.test(url);

export class Editor {
  constructor(root, { onChange } = {}) {
    this.root = root;
    this.onChange = onChange || (() => {});
    root.classList.add("doc");
    root.setAttribute("role", "textbox");
    root.setAttribute("aria-multiline", "true");
    root.addEventListener("keydown", (e) => this.onKeydown(e));
    root.addEventListener("input", (e) => this.onInput(e));
    root.addEventListener("paste", (e) => this.onPaste(e));
    root.addEventListener("contextmenu", (e) => this.onContextMenu(e));
    root.addEventListener("change", (e) => this.onCheck(e));
  }

  // ----- load / save -----

  setMarkdown(md) {
    this.root.replaceChildren();
    const blocks = markdownToBlocks(md || "");
    for (const b of blocks) this.root.append(this.make(b));
    if (!blocks.length) this.root.append(this.make({ type: "p" }));
    this.renumber();
  }

  getMarkdown() {
    return blocksToMarkdown(
      [...this.root.children].map((blk) => {
        const type = blk.dataset.type;
        if (type === "code") return { type, text: blk.querySelector("textarea").value };
        if (type === "hr") return { type };
        return { type, md: inlineFromNode(blk.querySelector(".txt")), checked: blk.dataset.checked === "1" };
      }),
    );
  }

  // Headings in document order, for the outline: [{ level, text, blk }]
  headings() {
    const out = [];
    for (const blk of this.root.children) {
      const m = /^h([123])$/.exec(blk.dataset.type);
      if (m) out.push({ level: Number(m[1]), text: blk.querySelector(".txt").textContent.trim(), blk });
    }
    return out;
  }

  focusStart() {
    const first = this.root.firstElementChild;
    if (first) this.focusBlock(first, "start");
  }

  // ----- block construction -----

  make({ type = "p", md, html, text = "", checked = false }) {
    const blk = document.createElement("div");
    blk.className = "blk";
    blk.dataset.type = type;

    if (type === "code") {
      blk.innerHTML = `<textarea class="code-text" rows="1" spellcheck="false" aria-label="Code block"></textarea>`;
      const ta = blk.querySelector("textarea");
      ta.value = text;
      this.autosize(ta);
      return blk;
    }
    if (type === "hr") {
      blk.tabIndex = 0;
      blk.innerHTML = "<hr>";
      return blk;
    }

    if (type === "todo") {
      blk.dataset.checked = checked ? "1" : "0";
      blk.classList.toggle("done", checked);
    }
    blk.innerHTML =
      this.marker(type, checked) +
      `<div class="txt" contenteditable="true" spellcheck="true" data-ph="${esc(PLACEHOLDER[type] || "")}"></div>`;
    const txt = blk.querySelector(".txt");
    txt.innerHTML = html ?? inlineToHtml(md ?? "");
    this.markEmpty(txt);
    return blk;
  }

  marker(type, checked) {
    if (type === "ul") return `<span class="mark bullet" contenteditable="false">•</span>`;
    if (type === "ol") return `<span class="mark num" contenteditable="false">1.</span>`;
    if (type === "todo") {
      return `<span class="mark check" contenteditable="false"><input type="checkbox" aria-label="Done"${checked ? " checked" : ""}></span>`;
    }
    return "";
  }

  renumber() {
    let n = 0;
    for (const blk of this.root.children) {
      if (blk.dataset.type === "ol") {
        n++;
        const mark = blk.querySelector(".mark");
        if (mark) mark.textContent = `${n}.`;
      } else {
        n = 0;
      }
    }
  }

  // Replace a block with another type, keeping its text where it makes sense.
  convert(blk, type, caret = "end", opts = {}) {
    let html, text;
    if (blk.dataset.type === "code") {
      text = blk.querySelector("textarea").value;
      html = esc(text).replace(/\n/g, "<br>");
    } else if (blk.dataset.type === "hr") {
      html = "";
    } else {
      const txt = blk.querySelector(".txt");
      html = txt.innerHTML;
      text = txt.textContent;
    }
    const next = this.make({ type, html: type === "code" || type === "hr" ? undefined : html, text, checked: opts.checked });
    blk.replaceWith(next);
    this.renumber();
    this.focusBlock(next, caret);
    this.onChange();
    return next;
  }

  remove(blk) {
    if (this.root.children.length === 1) {
      blk.replaceWith(this.make({ type: "p" }));
      this.focusBlock(this.root.firstElementChild, "start");
    } else {
      const target = blk.previousElementSibling || blk.nextElementSibling;
      const atEnd = blk.previousElementSibling !== null;
      blk.remove();
      this.renumber();
      this.focusBlock(target, atEnd ? "end" : "start");
    }
    this.onChange();
  }

  // ----- focus and caret -----

  focusBlock(blk, where = "end") {
    if (!blk) return;
    if (blk.dataset.type === "code") {
      const ta = blk.querySelector("textarea");
      ta.focus();
      const pos = where === "start" ? 0 : ta.value.length;
      ta.setSelectionRange(pos, pos);
      return;
    }
    if (blk.dataset.type === "hr") {
      blk.focus();
      return;
    }
    this.placeCaret(blk.querySelector(".txt"), where === "start");
  }

  placeCaret(el, atStart) {
    el.focus();
    const range = document.createRange();
    range.selectNodeContents(el);
    range.collapse(atStart);
    const sel = getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
  }

  // Put the caret at a character offset inside an element, for merges.
  placeCaretAtOffset(el, offset) {
    el.focus();
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    let remaining = offset;
    let node = walker.nextNode();
    let range = document.createRange();
    range.selectNodeContents(el);
    range.collapse(false);
    while (node) {
      if (remaining <= node.nodeValue.length) {
        range = document.createRange();
        range.setStart(node, remaining);
        range.collapse(true);
        break;
      }
      remaining -= node.nodeValue.length;
      node = walker.nextNode();
    }
    const sel = getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
  }

  caretRange() {
    const sel = getSelection();
    return sel.rangeCount ? sel.getRangeAt(0) : null;
  }

  textBefore(txt) {
    const r = this.caretRange();
    if (!r || !txt.contains(r.startContainer)) return "";
    const pre = document.createRange();
    pre.selectNodeContents(txt);
    pre.setEnd(r.startContainer, r.startOffset);
    return pre.toString();
  }

  textAfter(txt) {
    const r = this.caretRange();
    if (!r || !txt.contains(r.endContainer)) return "";
    const post = document.createRange();
    post.selectNodeContents(txt);
    post.setStart(r.endContainer, r.endOffset);
    return post.toString();
  }

  // ----- events -----

  onKeydown(e) {
    const t = e.target;
    if (t.matches(".code-text")) return this.codeKey(e, t.closest(".blk"));
    if (t.matches(".blk[data-type='hr']")) return this.hrKey(e, t);
    if (!t.matches(".txt")) return;

    const blk = t.closest(".blk");
    if (e.ctrlKey || e.metaKey) {
      if (!e.altKey && ["b", "i", "e", "k"].includes(e.key.toLowerCase())) {
        e.preventDefault();
        this.inline(e.key.toLowerCase());
      }
      return;
    }

    if (e.key === "Enter" && !e.shiftKey) return this.enter(e, t, blk);
    if (e.key === "Backspace") return this.backspace(e, t, blk);
    if (e.key === " " && !e.shiftKey) return this.shortcut(e, t, blk);
    if (e.key === "ArrowUp" && this.textBefore(t) === "") {
      const prev = blk.previousElementSibling;
      if (prev) {
        e.preventDefault();
        this.focusBlock(prev, "end");
      }
    }
    if (e.key === "ArrowDown" && this.textAfter(t) === "") {
      const next = blk.nextElementSibling;
      if (next) {
        e.preventDefault();
        this.focusBlock(next, "start");
      }
    }
  }

  onInput(e) {
    const t = e.target;
    if (t.matches(".code-text")) {
      this.autosize(t);
    } else if (t.matches(".txt")) {
      if (t.innerHTML === "<br>") t.innerHTML = "";
      this.markEmpty(t);
    } else {
      return;
    }
    this.onChange();
  }

  onCheck(e) {
    const box = e.target;
    if (!box.matches("input[type='checkbox']")) return;
    const blk = box.closest(".blk");
    blk.dataset.checked = box.checked ? "1" : "0";
    blk.classList.toggle("done", box.checked);
    this.onChange();
  }

  onPaste(e) {
    const t = e.target;
    if (!t.matches(".txt")) return;
    const text = e.clipboardData.getData("text/plain");
    if (!text) return;
    e.preventDefault();
    if (!text.includes("\n")) {
      document.execCommand("insertText", false, text);
      return;
    }
    const blk = t.closest(".blk");
    const nodes = markdownToBlocks(text).map((b) => this.make(b));
    if (!nodes.length) return;
    const isEmptyPara = blk.dataset.type === "p" && t.textContent === "";
    if (isEmptyPara) blk.replaceWith(...nodes);
    else blk.after(...nodes);
    this.renumber();
    this.focusBlock(nodes[nodes.length - 1], "end");
    this.onChange();
  }

  onContextMenu(e) {
    const blk = e.target.closest?.(".blk");
    if (!blk || !this.root.contains(blk)) return;
    e.preventDefault();
    const items = MENU.map(([type, label]) => ({
      label,
      current: blk.dataset.type === type,
      run: () => this.convert(blk, type, "end"),
    }));
    items.push("sep", {
      label: "Duplicate",
      run: () => {
        const copy = this.make({ type: blk.dataset.type, html: blk.querySelector(".txt")?.innerHTML, text: blk.querySelector("textarea")?.value, checked: blk.dataset.checked === "1" });
        blk.after(copy);
        this.renumber();
        this.onChange();
      },
    });
    items.push({ label: "Delete block", danger: true, run: () => this.remove(blk) });
    showMenu(e.clientX, e.clientY, items);
  }

  // ----- key handling -----

  enter(e, txt, blk) {
    const type = blk.dataset.type;
    const plain = txt.textContent;
    e.preventDefault();

    if (plain === "```") {
      txt.replaceChildren();
      this.convert(blk, "code", "start");
      return;
    }
    if (plain.trim() === "---") {
      const hr = this.make({ type: "hr" });
      const p = this.make({ type: "p" });
      blk.replaceWith(hr);
      hr.after(p);
      this.focusBlock(p, "start");
      this.onChange();
      return;
    }
    if (plain === "" && type !== "p") return this.convert(blk, "p", "start");

    const range = this.caretRange();
    if (!range) return;
    if (!range.collapsed) range.deleteContents();
    const tail = document.createRange();
    tail.setStart(range.endContainer, range.endOffset);
    tail.setEnd(txt, txt.childNodes.length);
    const frag = tail.extractContents();

    const nextType = type === "h1" || type === "h2" || type === "h3" ? "p" : type;
    const next = this.make({ type: nextType });
    next.querySelector(".txt").replaceChildren(frag);
    this.markEmpty(next.querySelector(".txt"));
    blk.after(next);
    this.renumber();
    this.focusBlock(next, "start");
    this.onChange();
  }

  backspace(e, txt, blk) {
    if (this.textBefore(txt) !== "") return;
    e.preventDefault();
    const type = blk.dataset.type;
    if (type !== "p") return this.convert(blk, "p", "start");

    const prev = blk.previousElementSibling;
    if (!prev) return;
    if (prev.dataset.type === "hr") {
      prev.remove();
      this.renumber();
      this.onChange();
      return;
    }
    if (prev.dataset.type === "code") {
      this.focusBlock(prev, "end");
      return;
    }
    const prevTxt = prev.querySelector(".txt");
    const joinAt = prevTxt.textContent.length;
    prevTxt.append(...txt.childNodes);
    prevTxt.normalize();
    blk.remove();
    this.renumber();
    this.markEmpty(prevTxt);
    this.placeCaretAtOffset(prevTxt, joinAt);
    this.onChange();
  }

  shortcut(e, txt, blk) {
    if (blk.dataset.type !== "p") return;
    const before = this.textBefore(txt);
    const rule = SHORTCUTS.find(([re]) => re.test(before));
    if (!rule) return;
    e.preventDefault();
    const r = this.caretRange();
    const pre = document.createRange();
    pre.selectNodeContents(txt);
    pre.setEnd(r.startContainer, r.startOffset);
    pre.deleteContents();
    this.convert(blk, rule[1], "start", { checked: rule[2] });
  }

  inline(key) {
    if (key === "b") document.execCommand("bold");
    else if (key === "i") document.execCommand("italic");
    else if (key === "k") {
      let url = (prompt("Link address") || "").trim();
      if (!url) return;
      if (!isSafeLink(url)) url = "https://" + url;
      document.execCommand("createLink", false, url);
    } else if (key === "e") {
      const r = this.caretRange();
      if (!r || r.collapsed) return;
      const code = document.createElement("code");
      code.appendChild(r.extractContents());
      r.insertNode(code);
      const after = document.createRange();
      after.setStartAfter(code);
      after.collapse(true);
      const sel = getSelection();
      sel.removeAllRanges();
      sel.addRange(after);
    }
    this.onChange();
  }

  codeKey(e, blk) {
    const ta = e.target;
    if (e.key === "Backspace" && ta.value === "") {
      e.preventDefault();
      this.convert(blk, "p", "start");
    } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      const p = this.make({ type: "p" });
      blk.after(p);
      this.focusBlock(p, "start");
      this.onChange();
    } else if (e.key === "Tab") {
      e.preventDefault();
      ta.setRangeText("  ", ta.selectionStart, ta.selectionEnd, "end");
      this.autosize(ta);
      this.onChange();
    }
  }

  hrKey(e, blk) {
    if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      this.remove(blk);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const p = this.make({ type: "p" });
      blk.after(p);
      this.focusBlock(p, "start");
      this.onChange();
    }
  }

  // ----- helpers -----

  markEmpty(txt) {
    txt.toggleAttribute("data-empty", txt.textContent === "" && !txt.querySelector("*"));
  }

  autosize(ta) {
    ta.style.height = "auto";
    ta.style.height = `${ta.scrollHeight}px`;
  }
}
