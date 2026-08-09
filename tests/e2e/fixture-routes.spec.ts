import { expect, test } from '@playwright/test';

test('renders the explicit Quick Switch group fixture at the compact viewport', async ({ page }) => {
  await page.setViewportSize({ width: 420, height: 360 });
  await page.goto('/?fixture=quick-switch');

  await expect(page).toHaveURL(/\/\?fixture=quick-switch$/);
  await expect(page.locator('[data-fixture-root="quick-switch"]')).toBeVisible();
  const quickSwitch = page.locator('main[data-surface="quick-switch"]');
  const openCodeOverrides = quickSwitch.getByRole('region', { name: 'OpenCode Overrides' });

  await expect(quickSwitch).toBeVisible();
  await expect(quickSwitch).toHaveAccessibleName('Default');
  await expect(quickSwitch.locator('[aria-label="Current group summary"]')).toBeVisible();
  await expect(openCodeOverrides).toBeVisible();
  await expect(openCodeOverrides).toContainText('reviewer');
  await expect(openCodeOverrides).toContainText('anthropic/claude-opus-4');
  await expect(page.getByRole('button', { name: 'Switch to Research' })).toBeEnabled();
});

test('renders group mappings and OpenCode agent overrides in the explicit Settings fixture at desktop width', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/settings?fixture=settings');

  await expect(page).toHaveURL(/\/settings\?fixture=settings$/);
  await expect(page.locator('[data-fixture-root="settings"]')).toBeVisible();
  await expect(page.getByRole('main', { name: 'Model group settings' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Group Settings' })).toBeVisible();
  await expect(page.getByRole('complementary', { name: 'Groups' })).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Category name 1' })).toHaveValue('build');
  await expect(page.getByRole('textbox', { name: 'Category model 1' })).toHaveValue('anthropic/claude-sonnet-4');
  await expect(page.getByRole('textbox', { name: 'Agent name 1' })).toHaveValue('planner');
  await expect(page.getByRole('textbox', { name: 'Agent model 1' })).toHaveValue('openai/gpt-5.1');
  const overrides = page.locator('[data-group-region="opencode-overrides"][data-opencode-mode="editable"]');

  await expect(overrides).toBeVisible();
  await expect(overrides.getByRole('textbox', { name: 'OpenCode model reviewer' })).toHaveValue('anthropic/claude-opus-4');
});

test('renders the explicit degraded Settings fixture at the intermediate viewport', async ({ page }) => {
  await page.setViewportSize({ width: 980, height: 620 });
  await page.goto('/settings?fixture=settings&degraded=opencode');

  await expect(page).toHaveURL(/\/settings\?fixture=settings&degraded=opencode$/);
  await expect(page.locator('[data-fixture-root="settings"]')).toBeVisible();
  await expect(page.getByRole('main', { name: 'Model group settings' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Group Settings' })).toBeVisible();
  const overrides = page.locator('[data-group-region="opencode-overrides"][data-opencode-mode="degraded"]');

  await overrides.scrollIntoViewIfNeeded();
  await expect(overrides).toBeVisible();
  await expect(overrides.getByRole('alert')).toContainText('OpenCode agent discovery warning: OpenCode config is malformed.');

  const readonlyOverride = overrides.locator('input[readonly]').first();
  await expect(readonlyOverride).toBeVisible();
  await expect(readonlyOverride).toHaveJSProperty('readOnly', true);
  await expect(readonlyOverride).not.toBeDisabled();
  await readonlyOverride.selectText();
  await expect(readonlyOverride).toBeFocused();
});

test('renders the explicit Settings fixture navigation at the narrow viewport', async ({ page }) => {
  await page.setViewportSize({ width: 680, height: 720 });
  await page.goto('/settings?fixture=settings');

  await expect(page).toHaveURL(/\/settings\?fixture=settings$/);
  await expect(page.locator('[data-fixture-root="settings"]')).toBeVisible();
  await expect(page.getByRole('main', { name: 'Model group settings' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Group Settings' })).toBeVisible();
  const groups = page.getByRole('complementary', { name: 'Groups' });
  const metadata = page.locator('[data-group-region="metadata"]');

  await expect(groups).toBeVisible();
  await expect(groups.getByRole('button', { name: 'New Group' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Save' })).toBeVisible();
  await metadata.scrollIntoViewIfNeeded();
  await expect(metadata.getByRole('textbox', { name: 'Name', exact: true })).toBeVisible();
});
