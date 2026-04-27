# Kingshot Schedule Maker - React Frontend

React + TypeScript frontend for the Schedule Maker, built with Vite and Tailwind CSS.

## Setup

```bash
cd frontend
npm install
```

## Development

```bash
npm run dev
```

Starts the Vite dev server on port 5173. Proxy is configured to forward API requests to the backend (default: `http://localhost:8080`). Make sure the backend is running.

## Build

```bash
npm run build
```

Builds static assets into `frontend/dist/` for the frontend container/static host.

## Production

1. Run `npm run build` in the frontend folder
2. Serve `frontend/dist/` with the frontend container or your web server
3. Run the Rust backend separately for API routes
