import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  // Tauri development no longer keeps a WebSocket HMR channel alive. Changes
  // are picked up through the normal HTTP dev server after a manual reload.
  server: { port: 1420, strictPort: true, hmr: false },
  envPrefix: ['VITE_', 'TAURI_'],
  build: { target: 'es2022', minify: 'esbuild', sourcemap: false },
});
