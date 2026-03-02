# Docker Deployment (Production)

Two-container setup: nginx (frontend) + Rust API (backend). Images are built and pushed to GitHub Container Registry (GHCR) on release.

**See [SERVER-SETUP.md](SERVER-SETUP.md) for a full step-by-step server setup and GitHub secrets guide.**

## Server setup (`/opt/myapp/` or your path)

### 1. One-time setup

```bash
# Clone repo
git clone https://github.com/YOUR_USERNAME/KingshotScheduleMaker.git /opt/myapp
cd /opt/myapp

# Create .env (secrets + config)
cp .env.example .env
nano .env
```

**Required in `.env`:**
```
# From your GitHub repo (e.g. vor/KingshotScheduleMaker)
GITHUB_REPOSITORY=owner/repo

# Pin to a release tag, or use 'latest'
IMAGE_TAG=v1.0.0

# OAuth (see docs/OAUTH_SETUP.md)
OAUTH_DISCORD_CLIENT_ID=...
OAUTH_DISCORD_CLIENT_SECRET=...
OAUTH_GOOGLE_CLIENT_ID=...
OAUTH_GOOGLE_CLIENT_SECRET=...

# URLs
BASE_URL=https://prep.vorre.dev
FRONTEND_URL=https://prep.vorre.dev

# Optional
PORT=80
```

### 2. Log in to GHCR (one-time)

```bash
# Create a PAT with read:packages, then:
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin
```

### 3. Deploy

```bash
cd /opt/myapp
git pull
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
```

## Version pinning & rollback

**Pin to a specific release:**
```bash
# In .env
IMAGE_TAG=v1.2.3
```

**Rollback to previous version:**
```bash
# Edit .env: IMAGE_TAG=v1.2.2 (or whatever previous tag)
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
```

**Use latest (not recommended for prod):**
```bash
IMAGE_TAG=latest
```

## GitHub Actions

1. **Build & push**: On Release publish, images are built and pushed to `ghcr.io/owner/repo/frontend:TAG` and `ghcr.io/owner/repo/backend:TAG`.

2. **Auto-deploy** (optional): Set repository variable `DEPLOY_ON_RELEASE=true` and add secrets (`DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_KEY`, `DEPLOY_PATH`). The workflow will SSH and run `docker compose pull && up` after each release.

## Local development

```bash
# Build and run
docker compose up --build -d

# App at http://localhost:80
```

Or run without Docker: `npm run dev` (frontend) + `cargo run web` (backend).
