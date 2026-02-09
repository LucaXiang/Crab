import path from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => {
    const isProduction = mode === 'production';

    return {
      base: './',
      server: {
        port: 3000,
        host: '0.0.0.0',
        hmr: {
          overlay: true,
        },
      },
      plugins: [react()],
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

      // 构建优化配置
      build: {
        // 代码分割 - 提升并行加载性能
        rollupOptions: {
          output: {
            manualChunks: {
              // React核心库
              'vendor-react': ['react', 'react-dom', 'react-router-dom'],

              // UI库
              'vendor-ui': ['lucide-react', '@dnd-kit/core', '@dnd-kit/sortable', '@dnd-kit/utilities'],

              // 状态管理
              'vendor-state': ['zustand'],

              // 图表库
              'vendor-charts': ['recharts'],

              // 工具库
              'vendor-utils': ['uuid', 'decimal.js'],

              // Tauri API
              'vendor-tauri': ['@tauri-apps/api', '@tauri-apps/plugin-dialog'],
            },
          },
        },

        // 压缩优化（仅生产环境）
        minify: isProduction ? 'terser' : false,
        terserOptions: isProduction ? {
          compress: {
            drop_debugger: true,       // 移除debugger
            pure_funcs: ['console.log', 'console.info', 'console.debug'], // 移除调试日志，保留 warn/error
          },
          format: {
            comments: false,           // 移除注释
          },
        } : undefined,

        // 资源优化
        assetsInlineLimit: 4096,       // 4KB以下的资源内联为base64

        // Chunk大小警告阈值
        chunkSizeWarningLimit: 1000,   // 1MB

        // sourcemap（开发环境启用）
        sourcemap: !isProduction,

        // 优化依赖预构建
        target: 'es2020',
        cssCodeSplit: true,            // CSS代码分割
      },

      // 优化依赖
      optimizeDeps: {
        include: [
          'react',
          'react-dom',
          'react-router-dom',
          'zustand',
        ],
      },
    };
});
