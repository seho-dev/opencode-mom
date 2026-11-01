import { fireEvent, render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import Button from '../button.svelte';

test('mounts an interactive Svelte component in the DOM', async () => {
  let clicks = 0;
  render(Button, { 'aria-label': 'Smoke', onclick: () => clicks++ });
  await fireEvent.click(screen.getByRole('button', { name: 'Smoke' }));
  expect(clicks).toBe(1);
});

test('disabled links share native disabled styling and restore navigation when enabled', async () => {
  const { rerender } = render(Button, { href: '/groups', 'aria-label': 'Groups' });
  const link = screen.getByRole('link', { name: 'Groups' });
  expect(link.getAttribute('href')).toBe('/groups');
  expect(link.tabIndex).toBe(0);

  await rerender({ href: '/groups', 'aria-label': 'Groups', disabled: true });
  expect(link.getAttribute('aria-disabled')).toBe('true');
  expect(link.hasAttribute('href')).toBe(false);
  expect(link.tabIndex).toBe(-1);

  render(Button, { disabled: true, 'aria-label': 'Save' });
  const button = screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement;
  expect(button.disabled).toBe(true);
  for (const utility of [
    'pointer-events-none',
    'cursor-not-allowed',
    'opacity-[var(--disabled-opacity)]',
    'shadow-none',
  ]) {
    expect(link.classList.contains(`aria-disabled:${utility}`)).toBe(true);
    expect(button.classList.contains(`disabled:${utility}`)).toBe(true);
  }

  await rerender({ href: '/groups', 'aria-label': 'Groups', disabled: false });
  expect(link.getAttribute('aria-disabled')).toBe('false');
  expect(link.getAttribute('href')).toBe('/groups');
  expect(link.tabIndex).toBe(0);
});
