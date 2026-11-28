<script>
  import { onMount } from 'svelte';
  let { title, close, children } = $props();
  let dialog;
  onMount(() => dialog.showModal());
</script>

<dialog bind:this={dialog} aria-labelledby="dialog-title" oncancel={(event) => { event.preventDefault(); close(); }}>
  <div class="panel-heading"><h2 id="dialog-title">{title}</h2><button class="secondary" type="button" onclick={close}>关闭</button></div>
  {@render children()}
</dialog>
