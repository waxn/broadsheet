<script>
  import { COLORS } from "../lib/colors.js";
  let { value = $bindable("charcoal"), allowNone = false } = $props();
</script>

<div class="swatches" role="radiogroup" aria-label="Colour">
  {#if allowNone}
    <button type="button" class="sw none" class:on={!value} title="Default" aria-label="Default" onclick={() => (value = "")}>∅</button>
  {/if}
  {#each Object.entries(COLORS) as [key, [name, hex]]}
    <button type="button" class="sw" class:on={value === key} style:background={hex} title={name} aria-label={name} onclick={() => (value = key)}></button>
  {/each}
</div>

<style>
  .swatches { display: flex; flex-wrap: wrap; gap: 6px; max-width: 290px; }
  .sw { width: 22px; height: 22px; border-radius: 50%; border: 2px solid transparent; padding: 0; }
  .sw.on { border-color: var(--text); box-shadow: inset 0 0 0 2px var(--surface); }
  .none { background: var(--bg); color: var(--muted); font-size: 12px; border-color: var(--border); }
</style>
