<script>
  // A floating panel anchored to an element or rectangle. Closes on outside click or Esc.
  import { outside, place } from "../lib/place.js";

  let { anchor, onclose, align = "start", width, children, class: cls = "" } = $props();

  function key(e) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onclose?.();
    }
  }
</script>

<div
  class="popover panel {cls}"
  style:width={width ? `${width}px` : undefined}
  use:place={{ anchor, align }}
  use:outside={() => onclose?.()}
  onkeydown={key}
  role="dialog"
  tabindex="-1"
>
  {@render children()}
</div>

<style>
  .popover {
    position: fixed;
    z-index: 60;
    border-radius: var(--radius);
    background: var(--raised);
    border-color: var(--border-strong);
    animation-duration: 100ms;
  }
</style>
