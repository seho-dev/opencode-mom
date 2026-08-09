<script lang="ts">
  import type { SettingsState, SettingsStore } from '../../../settingsStore';
  import GroupDraftEditor from './GroupDraftEditor.svelte';
  import GroupList from './GroupList.svelte';

  type Props = Readonly<{ store: SettingsStore; state: SettingsState }>;
  let { store, state }: Props = $props();
</script>

<section class="group-pane" aria-labelledby="group-settings-title">
  <GroupList {store} {state} />
  <GroupDraftEditor {store} {state} />
</section>

<style>
  .group-pane {
    display: grid;
    min-width: 0;
    min-height: 0;
    flex: 1;
    grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
    background: var(--surface-panel);
    overflow: hidden;
    container-type: inline-size;
    container-name: groups-pane;
  }

  @container groups-pane (width < 1040px) {
    .group-pane {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(140px, 180px) minmax(0, 1fr);
    }
  }
</style>
