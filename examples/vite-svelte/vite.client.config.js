import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  base: '/client/',
  plugins: [svelte({
    compilerOptions: {
      hydratable: true
    }
  })],
  build: {
    outDir: 'dist/client',
    emptyOutDir: true,
    rollupOptions: {
      input: './frontend/main.js', // 入口点为客户端应用
      output: {
        format: 'esm',
        entryFileNames: '[name].js',
        chunkFileNames: '[name]-[hash].js',
        assetFileNames: 'assets/[name][extname]',
      },
    }
  }
})
