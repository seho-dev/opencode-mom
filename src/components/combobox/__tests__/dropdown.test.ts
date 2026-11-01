import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterAll, afterEach, beforeAll, expect, test, vi } from 'vitest';
import Harness from './dropdown-harness.svelte';

const scrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');
beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});
afterAll(() => {
  if (scrollIntoView) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scrollIntoView);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});
afterEach(() => vi.restoreAllMocks());

test('the free-text combobox portals options and preserves pointer, keyboard and outside-focus behavior', async () => {
  const onAgentChange = vi.fn();
  const { container, unmount } = render(Harness, { onAgentChange });
  const input = screen.getByRole('combobox', { name: 'Agent' }) as HTMLInputElement;
  // JSDOM has no layout; Bits UI treats an empty client rect list as a hidden anchor.
  vi.spyOn(input, 'getClientRects').mockReturnValue([new DOMRect(16, 320, 368, 36)] as unknown as DOMRectList);
  input.focus();
  await waitFor(() => expect(screen.getByRole('listbox')).toBeTruthy());
  expect(container.contains(screen.getByRole('listbox'))).toBe(false);
  expect(input.getAttribute('aria-controls')).toBe(screen.getByRole('listbox').id);
  await fireEvent.mouseDown(screen.getByRole('option', { name: 'Option 2', exact: true }));
  await fireEvent.click(screen.getByRole('option', { name: 'Option 2', exact: true }));
  expect(input.value).toBe('Option 2');
  expect(onAgentChange).toHaveBeenLastCalledWith('Option 2');
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(input);

  await fireEvent.input(input, { target: { value: 'custom agent' } });
  expect(input.value).toBe('custom agent');
  expect(onAgentChange).toHaveBeenLastCalledWith('custom agent');
  expect(screen.queryAllByRole('option')).toHaveLength(0);
  await fireEvent.keyDown(input, { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(input);
  await fireEvent.keyDown(input, { key: 'ArrowDown' });
  await fireEvent.keyDown(input, { key: 'ArrowDown' });
  await fireEvent.keyDown(input, { key: 'Enter' });
  expect(input.value).toBe('Option 1');
  expect(onAgentChange).toHaveBeenLastCalledWith('Option 1');
  await fireEvent.keyDown(input, { key: 'ArrowDown' });
  await waitFor(() => expect(screen.getByRole('listbox')).toBeTruthy());
  // Bits UI registers outside-focus listeners after mounting the floating layer.
  await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  screen.getByRole('button', { name: 'Save', exact: true }).focus();
  await waitFor(() => {
    expect(input.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  input.focus();
  await waitFor(() => expect(screen.getByRole('listbox')).toBeTruthy());
  expect(document.activeElement).toBe(input);
  unmount();
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
});

test('the searchable select keeps its portaled search focused and supports selection and dismissal', async () => {
  const onModelChange = vi.fn();
  const { container } = render(Harness, { onModelChange });
  const trigger = screen.getByRole('button', { name: 'Model', exact: true });
  await fireEvent.click(trigger);
  const search = await screen.findByRole('combobox', { name: 'Search options' });
  expect(container.contains(screen.getByRole('listbox'))).toBe(false);
  expect(document.activeElement).toBe(search);
  await fireEvent.input(search, { target: { value: 'Option 12' } });
  await fireEvent.keyDown(search, { key: 'Enter' });
  expect(trigger.textContent).toContain('Option 12');
  expect(onModelChange).toHaveBeenLastCalledWith('Option 12');
  expect(document.activeElement).toBe(trigger);
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());

  await fireEvent.click(trigger);
  await fireEvent.click(trigger);
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  await fireEvent.click(trigger);
  await fireEvent.click(screen.getByRole('option', { name: 'Option 3', exact: true }));
  expect(trigger.textContent).toContain('Option 3');
  expect(onModelChange).toHaveBeenLastCalledWith('Option 3');
  await fireEvent.click(trigger);
  await fireEvent.keyDown(screen.getByRole('combobox', { name: 'Search options' }), { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(trigger);
  await fireEvent.click(trigger);
  await fireEvent.keyDown(screen.getByRole('combobox', { name: 'Search options' }), { key: 'Tab' });
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(trigger);
  await fireEvent.click(trigger);
  screen.getByRole('button', { name: 'Cancel', exact: true }).focus();
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancel', exact: true }));
  await fireEvent.click(screen.getByRole('button', { name: 'Disabled model' }));
  expect(screen.queryByRole('listbox')).toBeNull();
});

test('a short select keeps keyboard navigation and skips disabled options', async () => {
  const onModelChange = vi.fn();
  render(Harness, {
    options: [{ value: 'Blocked', disabled: true }, { value: 'First' }, { value: 'Second' }],
    onModelChange,
  });
  const trigger = screen.getByRole('button', { name: 'Model', exact: true });
  trigger.focus();
  await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
  expect(screen.queryByRole('combobox', { name: 'Search options' })).toBeNull();
  await fireEvent.click(screen.getByRole('option', { name: 'Blocked' }));
  expect(trigger.getAttribute('aria-expanded')).toBe('true');
  expect(onModelChange).not.toHaveBeenCalled();
  await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
  await fireEvent.keyDown(trigger, { key: 'Enter' });
  expect(trigger.textContent).toContain('Second');
  expect(onModelChange).toHaveBeenCalledExactlyOnceWith('Second');
  expect(document.activeElement).toBe(trigger);
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
});
