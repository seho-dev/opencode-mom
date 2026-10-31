import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import ResourceScope from '../ResourceScope.svelte';
import ResourceScopeNotice from '../ResourceScopeNotice.svelte';
import { makeScopedConfig } from './scope-config.js';

test('scope remains readonly by default and enables project copy only by explicit opt-in', async () => {
  const view = render(ResourceScope, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig() } });
  expect(screen.getByText(en['scope.projectReadOnly'])).toBeTruthy();
  await view.rerender({ writable: true });
  expect(screen.getByText(en['scope.projectWritable'])).toBeTruthy();
  expect(screen.queryByText(en['scope.projectReadOnly'])).toBeNull();
});

test('destination notice uses captured scope, and readonly comparison always overrides writable', async () => {
  const view = render(
    ResourceScopeNotice,
    { writable: true },
    { wrapper: Harness, wrapperProps: { config: makeScopedConfig() } },
  );
  expect(screen.getByText(en['scope.globalDestination'])).toBeTruthy();
  expect(screen.queryByText('/work/alpha')).toBeNull();
  await view.rerender({ projectId: 'project-b', writable: true });
  expect(screen.getByText('/other/alpha')).toBeTruthy();
  expect(screen.getByText(en['scope.projectWritable'])).toBeTruthy();
  await view.rerender({ projectId: null, writable: true, readonly: true });
  expect(screen.getByText(en['scope.globalReadOnly'])).toBeTruthy();
  expect(screen.queryByText(en['scope.globalDestination'])).toBeNull();
});
