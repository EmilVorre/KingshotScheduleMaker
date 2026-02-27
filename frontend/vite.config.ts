import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  base: '/',
  build: {
    outDir: '../prep-appointments/static/dist',
    emptyDirOnBuild: true,
  },
  server: {
    port: 5173,
    proxy: {
      '/api': { target: 'http://localhost:8080', changeOrigin: true },
      // Form routes: proxy only API calls (/form/xyz/api/*), serve SPA for page routes
      '/form': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        bypass: (req) => (req.url?.includes('/api/') ? null : '/index.html'),
      },
      // Account routes: /accountname/server/api/* - proxy to backend
      '^/[^/]+/[^/]+/api': { target: 'http://localhost:8080', changeOrigin: true },
    },
  },
})
