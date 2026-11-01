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

  rerender({ active: false, onsubmit });
  expect(container.querySelector('.form-page')).toBeNull();
  expect(screen.getByRole('textbox', { name: 'Name' })).toBeTruthy();
});
