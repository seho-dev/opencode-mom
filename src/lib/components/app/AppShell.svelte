<script lang="ts">
  import AppSidebar from './AppSidebar.svelte';
  import AppHeader from './AppHeader.svelte';
  import ConfigFeedback from './ConfigFeedback.svelte';
  import Toaster from './Toaster.svelte';
  let { children } = $props();
</script>

<div class="app-shell">
  <AppSidebar />
  <div class="workspace">
    <AppHeader />
    <main class="content">{@render children()}</main>
  </div>
  <ConfigFeedback />
  <Toaster />
</div>
