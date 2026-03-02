# Docker Deployment

Run the app as a container. Secrets are passed via environment variables (never baked into the image).

## Quick start

```bash
# 1. Create .env (copy from .env.example, fill in OAuth and URLs)
cp .env.example .env

# 2. Build and run
docker compose up -d

# App runs at http://localhost:8080
```

## Secrets (environment variables)

**Never put secrets in the image.** Pass them at runtime:

### Option A: env file (recommended)

```bash
cp .env.example .env
# Edit .env with your OAuth credentials, BASE_URL, etc.
docker compose up -d
```

`docker-compose.yml` uses `env_file: .env` to load all variables into the container.

### Option B: docker run with --env-file

```bash
docker build -t prep-appointments .
docker run -d -p 8080:8080 --env-file .env -v prep-data:/app/data prep-appointments
```

### Option C: Docker Compose secrets (file-based)

For stricter control, use Docker secrets (files mounted read-only):

```yaml
services:
  prep-appointments:
    secrets:
      - oauth_discord_secret
      - oauth_google_secret
    environment:
      OAUTH_DISCORD_CLIENT_ID: ${OAUTH_DISCORD_CLIENT_ID}
      OAUTH_DISCORD_CLIENT_SECRET_FILE: /run/secrets/oauth_discord_secret
      # ... app would need to read from file
```

The app currently reads from env vars only. To use file-based secrets, you'd need to add support for `*_FILE` vars (e.g. `OAUTH_DISCORD_CLIENT_SECRET_FILE`) that read the value from a file. For now, **env_file is the simplest and works well.**

### Option D: Host environment

```bash
export OAUTH_DISCORD_CLIENT_ID=...
export OAUTH_DISCORD_CLIENT_SECRET=...
docker compose up -d
```

Compose passes through variables that are set in the compose file's `environment` section.

## Different port

```bash
PORT=3000 docker compose up -d
# Or in .env: PORT=3000
```

## Data persistence

The `prep-data` volume stores accounts, forms, and schedules. It persists across container restarts.

To back up:
```bash
docker run --rm -v prep-data:/data -v $(pwd):/backup alpine tar czf /backup/prep-data-backup.tar.gz -C /data .
```

## Build only

```bash
docker build -t prep-appointments .
```

## Run without compose

```bash
docker build -t prep-appointments .
docker run -d \
  -p 8080:8080 \
  --env-file .env \
  -v prep-data:/app/data \
  --name prep-appointments \
  prep-appointments
```
