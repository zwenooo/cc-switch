import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig({
  plugins: [react()],
  root: resolve(__dirname, 'src/renderer'),
  base: './',
  build: {
    outDir: resolve(__dirname, 'build'),
    emptyOutDir: true
  },
  server: {
    port: 3000,
    strictPort: true
  },
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_']
})