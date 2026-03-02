# Server Setup Guide

Step-by-step guide to set up the Docker deployment on your server and configure GitHub for auto-deploy.

---

## Part 1: Server Setup

### 1. Install Docker

```bash
# Ubuntu/Debian
sudo apt update && sudo apt install -y docker.io docker-compose-plugin

# Enable and start
sudo systemctl enable docker
sudo systemctl start docker

# Add your user to docker group (so you don't need sudo)
sudo usermod -aG docker $USER
# Log out and back in for this to take effect
```

### 2. Clone the repo

```bash
sudo mkdir -p /opt/myapp
sudo chown $USER:$USER /opt/myapp
git clone https://github.com/YOUR_USERNAME/KingshotScheduleMaker.git /opt/myapp
cd /opt/myapp
```

### 3. Create `.env`

```bash
cp .env.example .env
nano .env
```

Fill in:

| Variable | Example | Description |
|----------|---------|-------------|
| `GITHUB_REPOSITORY` | `vor/KingshotScheduleMaker` | Your GitHub `owner/repo` |
| `IMAGE_TAG` | `v1.0.0` or `latest` | Release tag to run |
| `OAUTH_DISCORD_CLIENT_ID` | (from Discord dev portal) | Discord OAuth |
| `OAUTH_DISCORD_CLIENT_SECRET` | (from Discord dev portal) | Discord OAuth |
| `OAUTH_GOOGLE_CLIENT_ID` | (from Google Cloud Console) | Google OAuth |
| `OAUTH_GOOGLE_CLIENT_SECRET` | (from Google Cloud Console) | Google OAuth |
| `BASE_URL` | `https://prep.vorre.dev` | Public URL of the app |
| `FRONTEND_URL` | `https://prep.vorre.dev` | Same as BASE_URL in most cases |
| `PORT` | `80` | Port nginx listens on (optional) |

### 4. Log in to GitHub Container Registry (one-time)

Create a Personal Access Token (PAT):

1. GitHub → **Settings** → **Developer settings** → **Personal access tokens**
2. **Generate new token (classic)**
3. Scopes: `read:packages`
4. Generate and copy the token

On the server:

```bash
echo YOUR_PAT | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin
```

### 5. Create a release and deploy

1. Create a release in GitHub (e.g. tag `v1.0.0`)
2. On the server:

```bash
cd /opt/myapp
# Set the tag you want to run
sed -i 's/IMAGE_TAG=.*/IMAGE_TAG=v1.0.0/' .env
git pull
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
```

---

## Part 2: GitHub Secrets (for auto-deploy)

Only needed if you want GitHub to deploy automatically when you publish a release.

**Settings** → **Secrets and variables** → **Actions** → **New repository secret**

| Secret | Value |
|--------|--------|
| `DEPLOY_HOST` | Server hostname or IP (e.g. `prep.vorre.dev` or `123.45.67.89`) |
| `DEPLOY_USER` | SSH user (e.g. `vor` or `deploy`) |
| `DEPLOY_KEY` | Full contents of the private SSH key (e.g. `~/.ssh/id_rsa`) |
| `DEPLOY_PATH` | Path to the app on the server (e.g. `/opt/myapp`) |
| `DEPLOY_PORT` | (optional) Port for nginx (default `80`) |

### Enable auto-deploy

**Settings** → **Secrets and variables** → **Actions** → **Variables** → **New repository variable**

| Variable | Value |
|----------|--------|
| `DEPLOY_ON_RELEASE` | `true` |

With this set, publishing a release will trigger the workflow to SSH into the server and run `git pull`, `docker compose pull`, and `docker compose up -d`.

---

## Quick reference

| Step | Where | What |
|------|--------|------|
| 1 | Server | Install Docker |
| 2 | Server | Clone repo to `/opt/myapp` |
| 3 | Server | Create `.env` with OAuth, URLs, `GITHUB_REPOSITORY`, `IMAGE_TAG` |
| 4 | Server | `docker login ghcr.io` with a PAT |
| 5 | GitHub | Create a release (e.g. `v1.0.0`) |
| 6 | Server | `docker compose -f docker-compose.prod.yml pull && up -d` |
| 7 | GitHub (optional) | Add deploy secrets and `DEPLOY_ON_RELEASE=true` for auto-deploy |
