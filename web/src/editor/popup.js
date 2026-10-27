// The floating list used by the slash menu and by @ / [[ suggestions. Plain DOM, so it can be
// driven straight from TipTap's suggestion lifecycle.

const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

export function popupRenderer(emptyText = "No results") {
  return () => {
    let el;
    let items = [];
    let active = 0;
    let command;

    function position(rect) {
      if (!rect || !el) return;
      const h = el.offsetHeight;
      const w = el.offsetWidth;
      let top = rect.bottom + 6;
      if (top + h > innerHeight - 8) top = Math.max(8, rect.top - h - 6);
      el.style.top = `${top}px`;
      el.style.left = `${Math.max(8, Math.min(rect.left, innerWidth - w - 8))}px`;
    }

    function draw() {
      if (!items.length) {
        el.innerHTML = `<div class="sg-empty">${esc(emptyText)}</div>`;
        return;
      }
      let html = "";
      let group = null;
      items.forEach((it, i) => {
        if (it.group && it.group !== group) {
          group = it.group;
          html += `<div class="menu-label">${esc(group)}</div>`;
        }
        html += `<button type="button" class="sg-item${i === active ? " active" : ""}" data-i="${i}">
          <span class="sg-icon">${esc(it.icon ?? "")}</span>
          <span class="sg-text"><span class="sg-title">${esc(it.title)}</span>${it.subtitle ? `<span class="sg-sub">${esc(it.subtitle)}</span>` : ""}</span>
          ${it.hint ? `<span class="sg-hint">${esc(it.hint)}</span>` : ""}
        </button>`;
      });
      el.innerHTML = html;
      el.querySelector(".sg-item.active")?.scrollIntoView({ block: "nearest" });
    }

    function pick(i) {
      const it = items[i];
      if (it) command(it);
    }

    return {
      onStart(props) {
        items = props.items;
        command = props.command;
        active = 0;
        el = document.createElement("div");
        el.className = "suggest menu";
        el.addEventListener("mousedown", (e) => {
          e.preventDefault();
          const b = e.target.closest(".sg-item");
          if (b) pick(Number(b.dataset.i));
        });
        el.addEventListener("mousemove", (e) => {
          const b = e.target.closest(".sg-item");
          if (b && Number(b.dataset.i) !== active) {
            active = Number(b.dataset.i);
            el.querySelectorAll(".sg-item").forEach((x, i) => x.classList.toggle("active", i === active));
          }
        });
        document.body.append(el);
        draw();
        position(props.clientRect?.());
      },
      onUpdate(props) {
        items = props.items;
        command = props.command;
        active = Math.min(active, Math.max(0, items.length - 1));
        draw();
        position(props.clientRect?.());
      },
      onKeyDown({ event }) {
        if (event.key === "ArrowDown") {
          active = (active + 1) % Math.max(1, items.length);
          draw();
          return true;
        }
        if (event.key === "ArrowUp") {
          active = (active - 1 + items.length) % Math.max(1, items.length);
          draw();
          return true;
        }
        if (event.key === "Enter" || event.key === "Tab") {
          if (!items.length) return false;
          pick(active);
          return true;
        }
        if (event.key === "Escape") {
          el?.remove();
          el = null;
          return true;
        }
        return false;
      },
      onExit() {
        el?.remove();
        el = null;
      },
    };
  };
}
