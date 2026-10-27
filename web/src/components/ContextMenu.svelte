<script>
  import { closeMenu, menu } from "../lib/menu.svelte.js";
  import { outside, place } from "../lib/place.js";

  let active = $state(-1);
  let el = $state();
  const actionable = $derived(menu.items.map((it, i) => (it.action ? i : -1)).filter((i) => i >= 0));

  $effect(() => {
    if (menu.open) {
      active = -1;
      requestAnimationFrame(() => el?.focus());
    }
  });

  function run(item) {
    closeMenu();
    item.action?.();
  }

  function key(e) {
    if (e.key === "Escape") {
      e.stopPropagation();
      closeMenu();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const pos = actionable.indexOf(active);
      const next = e.key === "ArrowDown" ? pos + 1 : pos - 1;
      active = actionable[(next + actionable.length) % actionable.length];
    } else if (e.key === "Enter" && active >= 0) {
      e.preventDefault();
      run(menu.items[active]);
    }
  }
</script>

{#if menu.open}
  <div class="menu" bind:this={el} tabindex="-1" role="menu" use:place={{ anchor: { left: menu.x, right: menu.x, top: menu.y, bottom: menu.y }, gap: 0 }} use:outside={closeMenu} onkeydown={key}>
    {#each menu.items as item, i}
      {#if item.sep}
        <div class="menu-sep"></div>
      {:else if item.heading}
        <div class="menu-label">{item.heading}</div>
      {:else if item.row}
        <div class="row">{@render item.row()}</div>
      {:else}
        <button type="button" role="menuitem" class="menu-item" class:active={active === i} class:danger={item.danger} onmouseenter={() => (active = i)} onclick={() => run(item)}>
          {#if item.icon}<item.icon size={16} strokeWidth={1.75} color={item.color} />{/if}
          <span>{item.label}</span>
          {#if item.checked}<span class="hint">✓</span>{:else if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .row { padding: 4px 6px; }
</style>
