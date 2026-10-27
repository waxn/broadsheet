<script>
  import Modal from "./Modal.svelte";
  import { app } from "../lib/store.svelte.js";

  const groups = [
    ["Anywhere", [["Q", "Quick add a task"], ["N", "Add a task here"], ["Ctrl K  or  /", "Search and commands"], ["G then I / T / U / C / N / H", "Go to Inbox, Today, Upcoming, Calendar, Notes, Habits"], ["[", "Show or hide the sidebar"], ["Ctrl Shift F", "Focus mode (hide everything but the page)"], ["?", "This list"]]],
    ["Task lists", [["J / K", "Next / previous task"], ["Enter or E", "Open the task"], ["X", "Complete"], ["1 2 3 4", "Priority 1–4"], ["D", "Set date"], ["Delete", "Delete task"]]],
    ["Calendar", [["D / W / M", "Day, week, month"], ["T", "Today"], ["← / →", "Previous / next"], ["C", "New event"]]],
    ["Notes", [["/", "Insert a block (in the editor)"], ["@", "Mention a page, task, class or date"], ["[[", "Link a page"], ["Ctrl B / I / U / E", "Bold, italic, underline, code"], ["Ctrl K", "Add a link to the selection"], ["# ## ###  - 1.  [] > ``` ---", "Markdown shortcuts"]]],
    ["Quick add words", [["fri, tmr, next week, oct 30, in 3 days", "Due date"], ["3pm, at 9, 15:30, for 1h", "Time and length (books a time block)"], ["every day, every mon, wed, every 2 weeks", "Repeat"], ["p1 p2 p3", "Priority"], ["#Class  /Section", "Where it goes"]]],
  ];
</script>

<Modal onclose={() => (app.help = false)} width={720} top="6vh" label="Keyboard shortcuts">
  <div class="sc">
    <h2>Keyboard shortcuts</h2>
    <div class="grid">
      {#each groups as [name, rows]}
        <section>
          <h3>{name}</h3>
          {#each rows as [k, v]}
            <div class="row"><span class="keys">{#each k.split("  ") as part, i}{#if i}<span class="or"> </span>{/if}<kbd>{part}</kbd>{/each}</span><span>{v}</span></div>
          {/each}
        </section>
      {/each}
    </div>
  </div>
</Modal>

<style>
  .sc { padding: 20px 24px 26px; }
  h2 { margin: 0 0 12px; font-size: 20px; font-weight: 800; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px 28px; }
  h3 { font-size: 12.5px; color: var(--muted); margin: 14px 0 6px; text-transform: uppercase; letter-spacing: .05em; }
  .row { display: flex; justify-content: space-between; gap: 14px; padding: 4px 0; font-size: 13px; border-bottom: 1px solid var(--border); }
  .keys { flex: none; max-width: 55%; }
  kbd { white-space: nowrap; }
  .row span:last-child { text-align: right; color: var(--text-2); }
</style>
