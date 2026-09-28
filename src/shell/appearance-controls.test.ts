import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import AppearanceControls from './AppearanceControls.svelte';

test('provides config and translated labels to a child', () => {
  const config = {
    preferences: { theme: 'dark', locale: 'zh' },
    loading: false,
    splashLoading: false,
  } as ConfigStore;

  render(AppearanceControls, {}, { wrapper: Harness, wrapperProps: { config } });
  expect(screen.getByRole('group', { name: '页眉控件' })).toBeTruthy();
  expect(screen.getByRole('button', { name: '主题：深色。切换到浅色。' })).toBeTruthy();
});
