/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
  resolve: {
    alias: {
      '$lib': resolve(__dirname, 'lib'),
    },
  },
  test: {
    environment: 'node',
    include: ['**/*.test.ts'],
    globals: true,
  },
});
