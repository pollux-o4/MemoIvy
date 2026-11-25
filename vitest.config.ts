import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    projects: [
      { test: { name: 'unit', include: ['frontend/**/*.test.ts', 'scripts/**/*.test.ts'], environment: 'node' } },
      { test: { name: 'e2e', include: ['e2e/**/*.e2e.ts'], environment: 'node', testTimeout: 60000, hookTimeout: 120000, fileParallelism: false } },
    ],
  },
});
