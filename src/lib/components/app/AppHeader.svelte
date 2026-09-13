<script lang="ts">
  import { getConfig } from '$lib/features/config/context.js';
  import AppearanceControls from './AppearanceControls.svelte';
  import MobileNavigation from './MobileNavigation.svelte';
  const config = getConfig();
</script>

<header class="header">
  <div class="header-main">
    <MobileNavigation /><span class="header-title">CONTROL PLANE</span>
  </div>
  <div class="header-meta"><AppearanceControls /></div>
</header>
