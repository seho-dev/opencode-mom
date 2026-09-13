<script lang="ts">
  import { Moon, Sun } from '@lucide/svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';

  const config = getConfig();
  const i18n = getI18n();
  const theme = $derived(config.preferences.theme);
  const locale = $derived(config.preferences.locale);
  const themeLabel = $derived(
    theme === 'dark' ? i18n.t('appearance.switchToLight') : i18n.t('appearance.switchToDark'),
  );

  function toggleTheme() {
    config.setTheme(theme === 'dark' ? 'light' : 'dark');
  }
</script>

<div class="appearance" role="group" aria-label={i18n.t('appearance.group')}>
  <button type="button" class="appearance-button" aria-label={themeLabel} title={themeLabel} onclick={toggleTheme}>
    {#if theme === 'dark'}
      <Sun size={14} aria-hidden="true" />
    {:else}
      <Moon size={14} aria-hidden="true" />
    {/if}
  </button>
  <div class="language" role="group" aria-label={i18n.t('appearance.language')}>
    <button
      type="button"
      class="language-option"
      aria-pressed={locale === 'en'}
      aria-label={i18n.t('appearance.switchToEnglish')}
      onclick={() => config.setLocale('en')}>EN</button
    >
    <button
      type="button"
      class="language-option"
      aria-pressed={locale === 'zh'}
      aria-label={i18n.t('appearance.switchToChinese')}
      onclick={() => config.setLocale('zh')}>中文</button
    >
  </div>
</div>

<style>
  .appearance {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .appearance-button {
    width: var(--control-height-small);
    height: var(--control-height-small);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    transition:
      color 150ms ease,
      border-color 150ms ease,
      background-color 150ms ease;
  }
  .appearance-button:hover {
    border-color: var(--accent-primary);
    color: var(--accent-primary);
  }
  .appearance-button:active {
    background: var(--surface-active);
    color: var(--accent-primary);
  }
  .language {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
  }
  .language-option {
    height: 26px;
    padding: 0 8px;
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-primary);
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: 0.04em;
    transition:
      color 150ms ease,
      background-color 150ms ease;
  }
  .language-option + .language-option {
    border-left: 1px solid var(--border-subtle);
  }
  .language-option:hover {
    background: var(--surface-hover);
    color: var(--accent-primary);
  }
  .language-option[aria-pressed='true'] {
    background: var(--surface-active);
    color: var(--accent-primary);
  }
  /* Expand the tap area to ~44px on touch widths without changing visuals. */
  @media (max-width: 760px) {
    .appearance-button,
    .language-option {
      position: relative;
    }
    .appearance-button::after,
    .language-option::after {
      content: '';
      position: absolute;
      inset: -9px 0;
    }
  }
</style>
