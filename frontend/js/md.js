// Markdown <-> block conversion. Documents are stored as plain markdown;
// the editor works on blocks and uses these helpers to move between the two.

const escapeHtml = (s) =>
  String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

// Escape markers that would otherwise be read as formatting.
const escapeInline = (s) => s.replace(/[\\`*[]/g, "\\$&");

// Inline grammar: escapes, `code`, **bold**, *italic*, [text](url), and hard line breaks.
const INLINE = /\\([\\`*_[\]#>+\-.!()|~])|`([^`\n]+)`|\*\*([^\n]+?)\*\*|\*([^*\n]+?)\*|\[([^\]\n]+)\]\(([^)\s]+)\)|\n/g;
const SAFE_URL = /^(https?:\/\/|mailto:)/i;

export function inlineToHtml(src) {
  let out = "";
  let last = 0;
  const re = new RegExp(INLINE.source, "g");
  let m;
  while ((m = re.exec(src))) {
    out += escapeHtml(src.slice(last, m.index));
    last = re.lastIndex;
    if (m[1] !== undefined) out += escapeHtml(m[1]);
    else if (m[2] !== undefined) out += `<code>${escapeHtml(m[2])}</code>`;
    else if (m[3] !== undefined) out += `<strong>${inlineToHtml(m[3])}</strong>`;
    else if (m[4] !== undefined) out += `<em>${inlineToHtml(m[4])}</em>`;
    else if (m[5] !== undefined) {
      out += SAFE_URL.test(m[6])
        ? `<a href="${escapeHtml(m[6])}" target="_blank" rel="noopener noreferrer">${inlineToHtml(m[5])}</a>`
        : inlineToHtml(m[5]);
    } else out += "<br>";
  }
  return out + escapeHtml(src.slice(last));
}

// DOM inline content -> markdown inline text.
export function inlineFromNode(node) {
  let out = "";
  for (const child of node.childNodes) {
    if (child.nodeType === Node.TEXT_NODE) {
      out += escapeInline(child.nodeValue);
    } else if (child.nodeType === Node.ELEMENT_NODE) {
      const tag = child.tagName;
      const inner = inlineFromNode(child);
      if (tag === "STRONG" || tag === "B") out += inner ? `**${inner}**` : "";
      else if (tag === "EM" || tag === "I") out += inner ? `*${inner}*` : "";
      else if (tag === "CODE") {
        const text = child.textContent.replace(/`/g, "'");
        out += text ? `\`${text}\`` : "";
      } else if (tag === "A") {
        const href = (child.getAttribute("href") || "").replace(/\)/g, "%29");
        out += href ? `[${inner}](${href})` : inner;
      } else if (tag === "BR") out += "\\\n";
      else out += inner;
    }
  }
  return out;
}

// A paragraph that starts like a block marker must be escaped to stay a paragraph.
function guardBlockStart(md) {
  if (/^[#>+\-]/.test(md)) return "\\" + md;
  if (/^\d+\./.test(md)) return md.replace(/^(\d+)\./, "$1\\.");
  return md;
}

const LIST_TYPES = new Set(["ul", "ol", "todo", "quote"]);

// blocks: [{ type, md?, checked?, text? }] -> markdown string.
export function blocksToMarkdown(blocks) {
  const lines = [];
  let olIndex = 0;
  let prevType = null;
  blocks.forEach((b, i) => {
    olIndex = b.type === "ol" && prevType === "ol" ? olIndex + 1 : b.type === "ol" ? 1 : 0;
    let line;
    switch (b.type) {
      case "h1": line = `# ${b.md}`; break;
      case "h2": line = `## ${b.md}`; break;
      case "h3": line = `### ${b.md}`; break;
      case "ul": line = `- ${b.md}`; break;
      case "ol": line = `${olIndex}. ${b.md}`; break;
      case "todo": line = `- [${b.checked ? "x" : " "}] ${b.md}`; break;
      case "quote": line = `> ${b.md}`; break;
      case "hr": line = "---"; break;
      case "code": line = "```\n" + b.text + "\n```"; break;
      default: line = guardBlockStart(b.md);
    }
    // Consecutive items of the same list type stay tight; everything else gets a blank line.
    const tight = LIST_TYPES.has(b.type) && b.type === prevType;
    if (i > 0 && !tight) lines.push("");
    lines.push(line);
    prevType = b.type;
  });
  return lines.join("\n") + "\n";
}

// Markdown -> [{ type, md, checked?, text? }]
export function markdownToBlocks(source) {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (/^\s*$/.test(line)) {
      i++;
      continue;
    }
    if (/^```/.test(line)) {
      const buf = [];
      i++;
      while (i < lines.length && !/^```\s*$/.test(lines[i])) buf.push(lines[i++]);
      i++; // closing fence
      blocks.push({ type: "code", text: buf.join("\n") });
      continue;
    }
    if (/^(---+|\*\*\*+)\s*$/.test(line)) {
      blocks.push({ type: "hr" });
      i++;
      continue;
    }

    let type = "p";
    let content = line;
    let checked = false;
    let m;
    if ((m = /^(#{1,3})\s+(.*)$/.exec(line))) {
      type = `h${m[1].length}`;
      content = m[2];
    } else if ((m = /^[-*+]\s+\[( |x|X)\]\s?(.*)$/.exec(line))) {
      type = "todo";
      checked = m[1] !== " ";
      content = m[2];
    } else if ((m = /^[-*+]\s+(.*)$/.exec(line))) {
      type = "ul";
      content = m[1];
    } else if ((m = /^\d+\.\s+(.*)$/.exec(line))) {
      type = "ol";
      content = m[1];
    } else if ((m = /^>\s?(.*)$/.exec(line))) {
      type = "quote";
      content = m[1];
    }
    i++;
    // A trailing backslash is a hard line break: join the next line into this block.
    while (content.endsWith("\\") && i < lines.length && !/^\s*$/.test(lines[i])) {
      content = content.slice(0, -1) + "\n" + lines[i++];
    }
    blocks.push({ type, md: content, checked });
  }
  return blocks;
}
