<script>
  import { dismiss, toasts } from "../lib/toast.svelte.js";
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast" class:error={t.error}>
      <span>{t.message}</span>
      {#if t.action}
        <button type="button" class="undo" onclick={() => { dismiss(t.id); t.action(); }}>{t.label}</button>
      {/if}
      <button type="button" class="close" aria-label="Dismiss" onclick={() => dismiss(t.id)}>×</button>
    </div>
  {/each}
</div>

<style>
  .toasts { position: fixed; left: 24px; bottom: 24px; z-index: 90; display: flex; flex-direction: column; gap: 8px; }
  .toast {
    display: flex; align-items: center; gap: 14px; min-width: 260px; padding: 10px 12px 10px 16px;
    background: #f2efe9; color: #1b1a18; border-radius: 10px; box-shadow: var(--shadow);
    font-weight: 500; animation: rise 160ms ease-out;
  }
  .toast.error { background: #ffd9d4; }
  .undo { margin-left: auto; border: 0; background: none; color: var(--accent); font-weight: 650; padding: 2px 6px; }
  .close { border: 0; background: none; color: #6b6861; font-size: 18px; line-height: 1; padding: 0 4px; }
</style>
