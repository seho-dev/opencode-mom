import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import FormPageHarness from './form-page-harness.svelte';

test('form page keeps heading and actions outside its body without detaching submit controls', async () => {
  const onsubmit = vi.fn();
  const { container, rerender } = render(FormPageHarness, { onsubmit });
  const form = container.querySelector('form');
  const body = form?.querySelector(':scope > .form-body');
  const actions = form?.querySelector(':scope > .form-actions');
  expect(form?.parentElement?.classList.contains('form-page')).toBe(true);
  expect(body?.contains(screen.getByRole('heading'))).toBe(false);
  expect(body?.contains(actions ?? null)).toBe(false);
  expect((screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).form).toBe(form);
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(onsubmit).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(onsubmit).toHaveBeenCalledOnce();
  await rerender({ active: false, onsubmit });
  expect(container.querySelector('.form-page')).toBeNull();
  expect(screen.getByRole('textbox', { name: 'Name' })).toBeTruthy();
});

test('every long editor has an explicit body and footer; route wrappers opt in without changing read-only pages', () => {
  const source = (path: string) => readFileSync(resolve('src', path), 'utf8');
  for (const [route, form] of [
    ['providers', 'ProviderForm'],
    ['models', 'ModelForm'],
    ['agents', 'AgentForm'],
    ['groups', 'GroupForm'],
    ['mcp', 'McpForm'],
    ['skills', 'SkillForm'],
  ]) {
    const editor = source(`routes/${route}/${form}.svelte`);
    expect(editor).toContain('class="form-body form-grid"');
    expect(editor.indexOf('<FormActions')).toBeGreaterThan(editor.indexOf('class="form-body form-grid"'));
    expect(source(`routes/${route}/new/+page.svelte`)).toContain('<FormPage>');
    if (route === 'mcp' || route === 'skills') {
      expect(source(`routes/${route}/[id]/+page.svelte`)).toContain('<FormPage active={editing &&');
    } else {
      expect(source(`routes/${route}/[id]/edit/+page.svelte`)).toContain('<FormPage>');
    }
  }
  const proxy = source('routes/mcp/ProxyServerForm.svelte');
  expect(proxy).toContain('flex flex-col overflow-hidden');
  expect(proxy).toContain('<div class="form-body">');
  expect(proxy).toContain('<FormActions>');
  expect(proxy).not.toContain('overflow-y-auto');
});
