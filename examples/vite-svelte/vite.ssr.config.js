import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  base: '/client/',
  plugins: [svelte()],
  build: {
    ssr: true,
    outDir: 'dist/server',
    emptyOutDir: true,
    rollupOptions: {
      input: './frontend/server.js', // 入口点为服务端渲染
      output: {
        format: 'esm',
        entryFileNames: '[name].js',
        chunkFileNames: '[name]-[hash].js',
      }
    }
  },
  ssr: {
    noExternal: true,
  }
})
