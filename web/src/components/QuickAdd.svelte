<script>
  import Modal from "./Modal.svelte";
  import TaskComposer from "./TaskComposer.svelte";
  import { app } from "../lib/store.svelte.js";
  import { toast } from "../lib/toast.svelte.js";
  import * as D from "../lib/dates.js";

  let { defaults = {} } = $props();
  const close = () => (app.quickAdd = null);
</script>

<Modal onclose={close} width={620} top="14vh" label="Quick add">
  <TaskComposer
    variant="modal"
    {defaults}
    keepOpen={false}
    oncancel={close}
    oncreated={(t) => toast(`Added “${t.title}”${t.due_date ? ` · ${D.label(t.due_date)}` : ""}`, { label: "Open", action: () => (app.detail = t.id) })}
  />
</Modal>
