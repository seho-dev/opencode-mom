<script lang="ts">
  import AppSidebar from './AppSidebar.svelte';
  import AppHeader from './AppHeader.svelte';
  import ConfigFeedback from './ConfigFeedback.svelte';
  let { children } = $props();
</script>

<div class="app-shell">
  <AppSidebar />
  <div class="workspace">
    <AppHeader />
    <main class="content"><ConfigFeedback />{@render children()}</main>
  </div>
</div>
