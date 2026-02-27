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

Builds the app and outputs to `../prep-appointments/static/dist/`. The backend serves this when running in production.

## Production

1. Run `npm run build` in the frontend folder
2. Start the backend - it will serve the React app from `static/dist/`

If the build doesn't exist, the backend falls back to the legacy Vue templates.
