<script>
  import Popover from "./Popover.svelte";

  let { anchor, onpick, onclose, allowRemove = true } = $props();
  let custom = $state("");

  const SETS = {
    Common: "📄 📝 📘 📗 📕 📙 📓 📔 📒 📚 🗂️ 📁 📌 📎 ✏️ 🖊️ 🧠 💡 ✅ ☑️ ⭐ 🌟 🔥 ⚡ 🎯 🚀 🏁 🗓️ ⏰ ⌛ 🔔 🔖 🏷️ 💬 ❓ ❗ ⚠️ 🔒 🔑",
    School: "🎓 🏫 📐 📏 🧮 🧪 🔬 🧬 🔭 🌍 🗺️ 🏛️ ⚖️ 🎨 🎭 🎵 🎹 🎸 🏀 ⚽ 🏃 🧘 💻 🖥️ ⌨️ 🖱️ 🧑‍💻 📊 📈 📉 🧾 💰 🗣️ ✍️ 📖 🔤 🔢 ➗ ➕",
    Life: "🏠 🛒 🍎 🥗 🍳 ☕ 🍵 🍕 🎂 🎉 🎁 ✈️ 🚗 🚲 🚆 🏖️ ⛰️ 🌲 🌸 🌻 🌙 ☀️ 🌧️ ❄️ 🐶 🐱 🐻 🦊 🐼 🦄 🐢 🐙 ❤️ 🧡 💛 💚 💙 💜 🖤 🤍",
  };

  function submit(e) {
    e.preventDefault();
    const v = custom.trim();
    if (v) {
      onpick(v);
      onclose();
    }
  }
</script>

<Popover {anchor} {onclose} width={340}>
  <div class="ep">
    <form onsubmit={submit} class="top">
      <input class="input" placeholder="Paste any emoji…" bind:value={custom} maxlength="8" />
      {#if allowRemove}<button type="button" class="btn ghost sm" onclick={() => { onpick(""); onclose(); }}>Remove</button>{/if}
    </form>
    {#each Object.entries(SETS) as [name, list]}
      <div class="menu-label">{name}</div>
      <div class="grid">
        {#each list.split(" ") as e}
          <button type="button" onclick={() => { onpick(e); onclose(); }}>{e}</button>
        {/each}
      </div>
    {/each}
  </div>
</Popover>

<style>
  .ep { padding: 8px; max-height: 360px; overflow: auto; }
  .top { display: flex; gap: 6px; margin-bottom: 4px; }
  .grid { display: grid; grid-template-columns: repeat(10, 1fr); }
  .grid button { height: 30px; border: 0; background: none; border-radius: 5px; font-size: 19px; padding: 0; }
  .grid button:hover { background: var(--active); }
</style>
