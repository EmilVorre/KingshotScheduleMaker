# OAuth2 Setup (Discord & Google)

To enable Discord and Google login, set these environment variables before starting the server.

## Discord

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create an application (or use existing)
3. Under OAuth2 → Redirects, add: `http://localhost:8080/api/auth/callback?provider=discord` (or your production URL)
4. Copy Client ID and Client Secret

```env
OAUTH_DISCORD_CLIENT_ID=your_discord_client_id
OAUTH_DISCORD_CLIENT_SECRET=your_discord_client_secret
```

## Google

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project (or use existing)
3. Enable "Google+ API" or "Google Identity" 
4. Create OAuth 2.0 credentials (Web application)
5. Add authorized redirect URI: `http://localhost:8080/api/auth/callback?provider=google`

```env
OAUTH_GOOGLE_CLIENT_ID=your_google_client_id.apps.googleusercontent.com
OAUTH_GOOGLE_CLIENT_SECRET=your_google_client_secret
```

## Base URL (optional)

For production, set the public base URL so OAuth redirects work correctly:

```env
BASE_URL=https://your-domain.com
```

If not set, the server derives it from the request (scheme + host).

## Frontend URL (development)

When running the frontend dev server (Vite on port 5173) separately from the backend (port 8080), set `FRONTEND_URL` so OAuth redirects back to the frontend instead of the backend:

```env
FRONTEND_URL=http://localhost:5173
```

Without this, after Discord/Google login you'll be redirected to `http://localhost:8080/dashboard/...` instead of staying on the frontend.
