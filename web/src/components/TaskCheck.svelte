<script>
  // Todoist's round checkbox, ringed in the priority colour.
  import { Check } from "@lucide/svelte";
  import { priorityColor } from "../lib/colors.js";

  let { done = false, priority = 0, onclick, size = 18 } = $props();
  let popping = $state(false);

  function click(e) {
    e.stopPropagation();
    if (!done) {
      popping = true;
      setTimeout(() => (popping = false), 300);
    }
    onclick?.(e);
  }
</script>

<button
  type="button"
  class="check"
  class:done
  class:pop={popping}
  class:pri={priority > 0}
  style:--c={priorityColor(priority)}
  style:width="{size}px"
  style:height="{size}px"
  aria-label={done ? "Mark not done" : "Complete"}
  aria-pressed={done}
  onclick={click}
>
  <Check size={size - 7} strokeWidth={3} />
</button>

<style>
  .check {
    flex: none; display: grid; place-items: center; padding: 0; border-radius: 50%;
    border: 1.5px solid var(--c); background: transparent; color: transparent; transition: background 120ms, color 120ms;
  }
  .check.pri { border-width: 2px; background: color-mix(in srgb, var(--c) 14%, transparent); }
  .check:hover { color: var(--c); }
  .check.done { background: var(--c); border-color: var(--c); color: var(--bg); }
  .check.done:not(.pri) { background: var(--muted); border-color: var(--muted); }
  .pop { animation: pop 280ms ease-out; background: var(--c) !important; color: var(--bg) !important; }
</style>
