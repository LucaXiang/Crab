import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      '@components': path.resolve(__dirname, 'src/components'),
      '@stores': path.resolve(__dirname, 'src/stores'),
      '@utils': path.resolve(__dirname, 'src/utils'),
      '@screens': path.resolve(__dirname, 'src/screens'),
      '@services': path.resolve(__dirname, 'src/services'),
    }
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './vitest.setup.ts',
    include: ['src/**/*.test.tsx', 'src/**/*.test.ts'],
  },
});
