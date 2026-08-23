<script lang="ts">
  import { ShieldCheck } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { getConfig } from '$lib/features/config/context.js';
  import MobileNavigation from './MobileNavigation.svelte';
  const config = getConfig();
</script>

<header class="header">
  <div style="display:flex;align-items:center;gap:10px">
    <MobileNavigation /><span class="header-title">CONTROL PLANE</span>
  </div>
  <div class="header-meta">
    <Button
      size="sm"
      variant="ghost"
      title="Validate configuration"
      onclick={() => config.validate()}
      disabled={config.saving}><ShieldCheck size={14} /> Validate</Button
    ><span class="online">● READY</span><span>Local config</span>
  </div>
</header>
