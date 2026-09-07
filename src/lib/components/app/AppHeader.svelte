<script lang="ts">
  import { getConfig } from '$lib/features/config/context.js';
  import MobileNavigation from './MobileNavigation.svelte';
  const config = getConfig();
</script>

<header class="header">
  <div style="display:flex;align-items:center;gap:10px">
    <MobileNavigation /><span class="header-title">CONTROL PLANE</span>
  </div>
  <div class="header-meta">
    <span class="online">● READY</span><span>Local config</span>
  </div>
</header>
