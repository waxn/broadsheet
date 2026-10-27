<script>
  // A centred dialog over a dimmed page. Esc or a click on the backdrop closes it.
  let { onclose, width = 560, top = "12vh", label = "Dialog", children, class: cls = "" } = $props();

  function key(e) {
    if (e.key === "Escape" && !e.defaultPrevented) {
      e.stopPropagation();
      onclose?.();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" style:padding-top={top} onmousedown={(e) => e.target === e.currentTarget && onclose?.()} onkeydown={key}>
  <div class="panel modal {cls}" style:width="min({width}px, calc(100vw - 32px))" role="dialog" aria-modal="true" aria-label={label} tabindex="-1">
    {@render children()}
  </div>
</div>

<style>
  .modal { max-height: calc(100vh - 2 * 8vh); overflow: auto; display: flex; flex-direction: column; }
</style>
