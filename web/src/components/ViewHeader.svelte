<script>
  // Hey-style headline for each view.
  let { title, subtitle = "", icon = null, children, color = null } = $props();
</script>

<header class="vh">
  <div class="titles">
    <h1>
      {#if icon}<span class="icon" style:color>{icon}</span>{/if}
      {title}
    </h1>
    {#if subtitle}<p class="sub">{subtitle}</p>{/if}
  </div>
  <div class="actions">{@render children?.()}</div>
</header>

<style>
  .vh { display: flex; align-items: flex-end; gap: 12px; padding: 34px 0 14px; position: sticky; top: 0; background: var(--bg); z-index: 5; }
  .titles { flex: 1; min-width: 0; }
  h1 { margin: 0; font-size: 30px; line-height: 1.15; font-weight: 800; letter-spacing: -0.025em; display: flex; align-items: center; gap: 10px; }
  .icon { font-size: 26px; }
  .sub { margin: 4px 0 0; color: var(--muted); font-size: 13.5px; font-weight: 500; }
  .actions { display: flex; align-items: center; gap: 4px; padding-bottom: 2px; }
</style>
