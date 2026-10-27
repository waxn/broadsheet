<script>
  import { post } from "../lib/api.js";
  import { boot } from "../lib/store.svelte.js";

  let mode = $state("login");
  let username = $state("");
  let password = $state("");
  let error = $state("");
  let busy = $state(false);

  async function submit(e) {
    e.preventDefault();
    busy = true;
    error = "";
    try {
      await post(mode === "login" ? "/api/login" : "/api/signup", { username, password });
      await boot();
    } catch (err) {
      error = err.message;
    }
    busy = false;
  }
</script>

<div class="wrap">
  <form class="card panel" onsubmit={submit}>
    <div class="logo"><img src="/favicon.svg" alt="" width="40" height="40" /></div>
    <h1>{mode === "login" ? "Welcome back" : "Make an account"}</h1>
    <p class="muted">Tasks, notes and your calendar, together.</p>
    <label>Username<input class="input" bind:value={username} autocomplete="username" required minlength="3" /></label>
    <label>Password<input class="input" type="password" bind:value={password} autocomplete={mode === "login" ? "current-password" : "new-password"} required /></label>
    {#if error}<p class="error">{error}</p>{/if}
    <button class="btn primary big" disabled={busy}>{mode === "login" ? "Log in" : "Sign up"}</button>
    <button type="button" class="btn ghost" onclick={() => { mode = mode === "login" ? "signup" : "login"; error = ""; }}>
      {mode === "login" ? "New here? Sign up" : "Have an account? Log in"}
    </button>
  </form>
</div>

<style>
  .wrap { height: 100%; display: grid; place-items: center; padding: 16px; background: radial-gradient(1200px 600px at 50% -10%, #2a2420, var(--bg)); }
  .card { width: min(380px, 100%); padding: 32px; display: flex; flex-direction: column; gap: 12px; }
  .logo { margin-bottom: 4px; }
  h1 { margin: 0; font-size: 28px; font-weight: 800; letter-spacing: -0.02em; }
  p { margin: 0 0 8px; }
  label { display: flex; flex-direction: column; gap: 6px; font-weight: 550; font-size: 13px; color: var(--text-2); }
  .big { height: 40px; font-size: 14px; margin-top: 6px; }
  .error { color: var(--p1); font-size: 13px; }
</style>
