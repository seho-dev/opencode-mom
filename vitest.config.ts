import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  resolve: { conditions: ['browser'] },
  test: {
    environment: 'jsdom',
    include: ['src/**/__tests__/*.test.ts'],
    setupFiles: ['src/test-utils/setup.ts'],
  },
});
