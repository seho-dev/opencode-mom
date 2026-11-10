import { cleanup } from '@testing-library/svelte';
import { afterEach } from 'vitest';
import '@testing-library/svelte/vitest';

// Bits UI defers its body-scroll-lock cleanup by ~24ms after the last layer
// unmounts. When a test file ends inside that window the timer fires after
// jsdom is torn down and vitest reports `document is not defined` as an
// unhandled error, failing the run. Unmount explicitly, then give the timer
// room to run while the environment is still alive.
afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 40));
});
