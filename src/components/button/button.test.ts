import { fireEvent, render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import Button from './button.svelte';

test('mounts an interactive Svelte component in the DOM', async () => {
  let clicks = 0;
  render(Button, { 'aria-label': 'Smoke', onclick: () => clicks++ });
  await fireEvent.click(screen.getByRole('button', { name: 'Smoke' }));
  expect(clicks).toBe(1);
});
