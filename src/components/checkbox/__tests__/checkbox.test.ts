import { createEvent, fireEvent, render, screen } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import QuestionForm from '$src/routes/island/QuestionForm.svelte';
import ContextHarness from '$src/test-utils/harness.svelte';
import type { IslandQuestion } from '$src/types/island.js';
import { Checkbox, type CheckboxProps } from '../index.js';
import Harness from './checkbox-harness.svelte';

test('toggles checked state via click, label association, and Space while ignoring Enter', async () => {
  const onCheckedChange = vi.fn();
  const onSubmitFormData = vi.fn();
  const { component } = render(Harness, { onCheckedChange, onSubmitFormData });
  const harness = component as unknown as {
    getControlRef: () => HTMLButtonElement | null;
    getCheckedState: () => boolean;
  };

  const checkbox = screen.getByRole('checkbox', { name: 'Enable feature' }) as HTMLButtonElement;
  expect(checkbox.type).toBe('button');
  expect(checkbox.getAttribute('data-slot')).toBe('checkbox');
  expect(checkbox.getAttribute('aria-checked')).toBe('false');
  expect(harness.getControlRef()).toBe(checkbox);
  expect(harness.getCheckedState()).toBe(false);

  await fireEvent.click(checkbox);
  expect(checkbox.getAttribute('aria-checked')).toBe('true');
  expect(checkbox.getAttribute('data-state')).toBe('checked');
  expect(harness.getCheckedState()).toBe(true);
  expect(onCheckedChange).toHaveBeenCalledExactlyOnceWith(true);

  await fireEvent.click(screen.getByText('Enable feature'));
  expect(checkbox.getAttribute('aria-checked')).toBe('false');
  expect(harness.getCheckedState()).toBe(false);
  expect(onCheckedChange).toHaveBeenLastCalledWith(false);

  checkbox.focus();
  const space = createEvent.keyDown(checkbox, { key: ' ' });
  await fireEvent(checkbox, space);
  expect(space.defaultPrevented).toBe(true);
  expect(checkbox.getAttribute('aria-checked')).toBe('true');
  expect(harness.getCheckedState()).toBe(true);
  expect(onCheckedChange).toHaveBeenLastCalledWith(true);

  const enter = createEvent.keyDown(checkbox, { key: 'Enter' });
  await fireEvent(checkbox, enter);
  expect(enter.defaultPrevented).toBe(true);
  expect(checkbox.getAttribute('aria-checked')).toBe('true');
  expect(harness.getCheckedState()).toBe(true);
  expect(onCheckedChange).toHaveBeenCalledTimes(3);
  expect(onSubmitFormData).not.toHaveBeenCalled();
});

test('supports indeterminate state and transitions to checked on user activation', async () => {
  const onCheckedChange = vi.fn();
  const onIndeterminateChange = vi.fn();
  const { component } = render(Harness, {
    initialChecked: false,
    initialIndeterminate: true,
    onCheckedChange,
    onIndeterminateChange,
  });
  const harness = component as unknown as {
    getIndeterminateState: () => boolean;
  };

  const checkbox = screen.getByRole('checkbox', { name: 'Enable feature' }) as HTMLButtonElement;
  expect(checkbox.getAttribute('aria-checked')).toBe('mixed');
  expect(checkbox.getAttribute('data-state')).toBe('indeterminate');
  expect(checkbox.querySelector('[data-slot="checkbox-indicator"] svg')).not.toBeNull();
  expect(harness.getIndeterminateState()).toBe(true);

  await fireEvent.click(checkbox);
  expect(checkbox.getAttribute('aria-checked')).toBe('true');
  expect(checkbox.getAttribute('data-state')).toBe('checked');
  expect(harness.getIndeterminateState()).toBe(false);
  expect(onIndeterminateChange).toHaveBeenCalledExactlyOnceWith(false);
  expect(onCheckedChange).toHaveBeenCalledExactlyOnceWith(true);
});

test('prevents pointer and keyboard changes when disabled', async () => {
  const onCheckedChange = vi.fn();
  render(Harness, { initialChecked: true, disabled: true, onCheckedChange });

  const checkbox = screen.getByRole('checkbox', { name: 'Enable feature' }) as HTMLButtonElement;
  expect(checkbox.disabled).toBe(true);
  expect(checkbox.getAttribute('aria-checked')).toBe('true');

  await fireEvent.click(checkbox);
  await fireEvent.keyDown(checkbox, { key: ' ' });
  expect(checkbox.getAttribute('aria-checked')).toBe('true');
  expect(onCheckedChange).not.toHaveBeenCalled();
});

test('synchronizes hidden form input with name/value for FormData and omits it without name', async () => {
  const onSubmitFormData = vi.fn();
  const { container } = render(Harness, { onSubmitFormData });

  const hiddenInput = container.querySelector('input[type="checkbox"][name="featureFlag"]') as HTMLInputElement;
  expect(hiddenInput).not.toBeNull();
  expect(hiddenInput.getAttribute('aria-hidden')).toBe('true');
  expect(hiddenInput.tabIndex).toBe(-1);

  await fireEvent.click(screen.getByRole('button', { name: 'Submit form' }));
  expect(onSubmitFormData).toHaveBeenLastCalledWith([]);

  await fireEvent.click(screen.getByRole('checkbox', { name: 'Enable feature' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Submit form' }));
  expect(onSubmitFormData).toHaveBeenLastCalledWith([['featureFlag', 'enabled']]);

  const props: CheckboxProps = { 'aria-label': 'Standalone' };
  const standalone = render(Checkbox, props);
  expect(standalone.container.querySelector('input[type="checkbox"]')).toBeNull();
});

test('QuestionForm uses Space for selection and Enter never submits option or custom checkboxes', async () => {
  const onanswer = vi.fn();
  const onskip = vi.fn();
  const question: IslandQuestion = {
    id: 'question_keyboard',
    sessionId: 'session',
    title: 'Choose targets',
    answerable: true,
    fields: [
      {
        key: 'targets_key',
        type: 'multiselect',
        title: 'Targets',
        options: [{ value: 'web_value', label: 'Web label' }],
        custom: true,
        required: true,
      },
    ],
  };
  const config = { preferences: { locale: 'en' } } as unknown as ConfigStore;
  render(
    QuestionForm,
    { question, busy: false, onanswer, onskip },
    { wrapper: ContextHarness, wrapperProps: { config } },
  );

  const option = screen.getByRole('checkbox', { name: 'Web label' }) as HTMLButtonElement;
  option.focus();
  await fireEvent.keyDown(option, { key: ' ' });
  expect(option.getAttribute('aria-checked')).toBe('true');

  await fireEvent.input(screen.getByRole('textbox', { name: 'Custom answer for Targets' }), {
    target: { value: 'Custom value' },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Add' }));
  const custom = screen.getByRole('checkbox', { name: 'Custom value' }) as HTMLButtonElement;

  for (const checkbox of [option, custom]) {
    expect(checkbox.type).toBe('button');
    checkbox.focus();
    const enter = createEvent.keyDown(checkbox, { key: 'Enter' });
    await fireEvent(checkbox, enter);
    expect(enter.defaultPrevented).toBe(true);
    expect(checkbox.getAttribute('aria-checked')).toBe('true');
  }
  expect(onanswer).not.toHaveBeenCalled();
  expect(onskip).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole('button', { name: 'Submit answers' }));
  expect(onanswer).toHaveBeenCalledExactlyOnceWith({ targets_key: ['web_value', 'Custom value'] });
});
