import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import AlertRoot from './alert/alert.svelte';
import { buttonVariants } from './button/index.js';
import UiPrimitivesShowcase from './UiPrimitivesShowcase.svelte';

describe('owned UI primitive behavior', () => {
  afterEach(() => {
    cleanup();
  });

  it('Given Button variants When generated Then compact sizes remain available', () => {
    expect(buttonVariants({ variant: 'default', size: 'default' })).toContain('h-[var(--control-height-compact)]');
    expect(buttonVariants({ variant: 'secondary', size: 'sm' })).toContain('h-[var(--control-height-small)]');
    expect(buttonVariants({ variant: 'outline', size: 'icon' })).toContain('size-[var(--control-height-compact)]');
    expect(buttonVariants({ variant: 'ghost', size: 'icon-sm' })).toContain('size-[var(--control-height-small)]');
  });

  it('Given the showcase When rendered Then text controls forward invalid, readonly, disabled, and value attributes', async () => {
    render(UiPrimitivesShowcase);

    expect(screen.getByLabelText('Invalid input').getAttribute('aria-invalid')).toBe('true');
    expect(screen.getByLabelText('Read-only input').hasAttribute('readonly')).toBe(true);
    expect(screen.getByLabelText('Disabled input').hasAttribute('disabled')).toBe(true);
    expect((screen.getByLabelText('Read-only textarea') as HTMLTextAreaElement).value).toBe('Inspectable configuration');
  });

  it('Given the focused Textarea state When focused Then native focus and forwarded attributes are preserved', async () => {
    render(UiPrimitivesShowcase);

    const textarea = screen.getByLabelText('Focus textarea') as HTMLTextAreaElement;
    expect(textarea.tagName).toBe('TEXTAREA');
    expect(textarea.getAttribute('data-qa')).toBe('textarea-focus');
    textarea.focus();
    expect(document.activeElement).toBe(textarea);
    expect(textarea.getAttribute('aria-invalid')).toBeNull();
  });

  it('Given Checkbox states When Space is pressed Then checked changes and disabled stays unchanged', async () => {
    render(UiPrimitivesShowcase);

    const checkbox = screen.getByRole('checkbox', { name: 'Keyboard checkbox' });
    checkbox.focus();
    await fireEvent.keyDown(checkbox, { key: ' ' });
    await fireEvent.keyUp(checkbox, { key: ' ' });
    expect(checkbox.getAttribute('aria-checked')).toBe('true');

    const disabled = screen.getByRole('checkbox', { name: 'Disabled checkbox' });
    expect(disabled.hasAttribute('disabled')).toBe(true);
    expect(disabled.getAttribute('aria-checked')).toBe('true');
  });

  it('Given Alert tones When rendered Then roles, action placement, and forwarded props are preserved', async () => {
    const { container } = render(UiPrimitivesShowcase);

    expect(screen.getByRole('status').textContent).toContain('Warning alert');
    expect(screen.getByRole('alert').textContent).toContain('Destructive alert');
    const action = screen.getByRole('button', { name: 'Resolve warning' });
    expect(action.getAttribute('data-showcase-action')).toBe('warning');
    expect(action.closest('[data-slot="alert-action"]')).toBeInstanceOf(HTMLElement);
    expect(container.querySelectorAll('.alert-grid [data-slot="badge"]')).toHaveLength(0);
    expect(screen.getByRole('heading', { name: 'Badge and Alert' }).nextElementSibling?.querySelectorAll('[data-slot="badge"]')).toHaveLength(5);
  });

  it('Given Alert roles When omitted or explicitly undefined Then default and non-live semantics are preserved', () => {
    const { container: defaultContainer } = render(AlertRoot);
    const { container: nonLiveContainer } = render(AlertRoot, { props: { role: undefined } });
    const { container: statusContainer } = render(AlertRoot, { props: { role: 'status' } });
    const { container: alertContainer } = render(AlertRoot, { props: { role: 'alert' } });

    expect(defaultContainer.querySelector('[data-slot="alert"]')?.getAttribute('role')).toBe('alert');
    expect(nonLiveContainer.querySelector('[data-slot="alert"]')?.hasAttribute('role')).toBe(false);
    expect(statusContainer.querySelector('[data-slot="alert"]')?.getAttribute('role')).toBe('status');
    expect(alertContainer.querySelector('[data-slot="alert"]')?.getAttribute('role')).toBe('alert');
  });

  it('Given Dialog is opened When Escape is pressed Then it closes without a custom close control', async () => {
    render(UiPrimitivesShowcase);

    const trigger = screen.getByRole('button', { name: 'Open dialog' });
    trigger.focus();
    await fireEvent.click(trigger);
    const dialog = await screen.findByRole('dialog', { name: 'Primitive dialog' });
    expect(within(dialog).getByRole('button', { name: 'Cancel dialog' })).toBeInstanceOf(HTMLElement);
    await fireEvent.keyDown(document, { key: 'Escape' });
    await fireEvent.keyUp(document, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Primitive dialog' })).toBeNull());
    expect(trigger.getAttribute('data-state')).toBe('closed');
    expect(document.activeElement).toBe(trigger);
  });

  it('Given Spinner and Separator examples When rendered Then the icon is hidden and both orientations exist', async () => {
    const { container } = render(UiPrimitivesShowcase);

    expect(screen.queryByRole('status', { name: 'Loading' })).toBeNull();
    expect(container.querySelector('[data-slot="spinner"][aria-hidden="true"]')).toBeInstanceOf(SVGElement);
    expect(container.querySelector('[data-slot="separator"][data-orientation="horizontal"]')).toBeInstanceOf(HTMLElement);
    expect(container.querySelector('[data-slot="separator"][data-orientation="vertical"]')).toBeInstanceOf(HTMLElement);
  });

  it('Given a Tooltip trigger When keyboard focused Then it opens after the owned provider delay', async () => {
    const { container } = render(UiPrimitivesShowcase);

    const trigger = screen.getByRole('button', { name: 'Tooltip action' });
    expect(trigger.getAttribute('tabindex')).toBe('0');
    expect(trigger.getAttribute('data-state')).toBe('closed');
    expect(container.querySelector('[role="tooltip"]')).toBeNull();
    await fireEvent.focus(trigger);
    const tooltip = await screen.findByRole('tooltip');
    expect(tooltip.textContent).toContain('Keyboard tooltip');
  });
});
