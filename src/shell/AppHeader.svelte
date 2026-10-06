<script lang="ts">
import { getConfig } from '$src/config/context.js';
import AppearanceControls from './AppearanceControls.svelte';
import MobileNavigation from './MobileNavigation.svelte';

const config = getConfig();
</script>

<header class="header">
  <div class="header-main">
    <MobileNavigation />
  </div>
  <div class="header-meta"><AppearanceControls /></div>
</header>
