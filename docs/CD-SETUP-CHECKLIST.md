# CD Setup Checklist

What to configure outside the code for the Docker release CD to work.

---

## 1. Push your code

```bash
git push origin main
```

---

## 2. Server setup (one-time)

### 2a. Install Docker

```bash
# Ubuntu/Debian
sudo apt update && sudo apt install -y docker.io docker-compose-plugin
sudo systemctl enable docker && sudo systemctl start docker
sudo usermod -aG docker $USER
# Log out and back in
```

### 2b. Clone repo on server

```bash
mkdir -p /opt/myapp  # or ~/myapp
git clone https://github.com/YOUR_USERNAME/KingshotScheduleMaker.git /opt/myapp
cd /opt/myapp
```

### 2c. Create `.env` on server

```bash
cp .env.example .env
nano .env
```

Fill in: `GITHUB_REPOSITORY`, `IMAGE_TAG`, OAuth credentials, `BASE_URL`, `FRONTEND_URL`.

### 2d. Log in to GHCR (one-time)

1. GitHub → Settings → Developer settings → Personal access tokens
2. Generate token (classic) with scope `read:packages`
3. On server:

```bash
echo YOUR_PAT | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin
```

### 2e. SSH key for GitHub Actions (for auto-deploy)

Generate a key pair for GitHub Actions to SSH into the server:

```bash
# On your machine (or server)
ssh-keygen -t ed25519 -f deploy_key -N ""
```

- Add the **public** key (`deploy_key.pub`) to the server: append to `~/.ssh/authorized_keys` for `DEPLOY_USER`
- Add the **private** key (`deploy_key`) contents to GitHub secret `DEPLOY_KEY`

---

## 3. GitHub (for auto-deploy)

Only needed if you want GitHub to deploy automatically on release.

### 3a. Add secrets

**Repo** → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**

| Secret | Value |
|--------|--------|
| `DEPLOY_HOST` | Server hostname or IP (e.g. `prep.vorre.dev`) |
| `DEPLOY_USER` | SSH user on server |
| `DEPLOY_KEY` | Full contents of private SSH key |
| `DEPLOY_PATH` | Path to app (e.g. `/opt/myapp`) |
| `DEPLOY_PORT` | (optional) Port, default `80` |

### 3b. Enable auto-deploy

**Repo** → **Settings** → **Secrets and variables** → **Actions** → **Variables** → **New repository variable**

| Variable | Value |
|----------|--------|
| `DEPLOY_ON_RELEASE` | `true` |

---

## 4. Create the first release

1. GitHub → **Releases** → **Create a new release**
2. Tag: `v1.0.0` (or similar)
3. Publish release

This triggers **build-and-push** (builds images, pushes to GHCR). If `DEPLOY_ON_RELEASE=true` and secrets are set, deploy runs automatically.

---

## 5. Manual deploy (if not using auto-deploy)

After a release is published:

```bash
# On server
cd /opt/myapp
sed -i 's/IMAGE_TAG=.*/IMAGE_TAG=v1.0.0/' .env  # use your release tag
git pull
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
```

---

## Checklist summary

| # | Where | Done |
|---|--------|------|
| 1 | Push code | `git push origin main` |
| 2a | Server | Install Docker |
| 2b | Server | Clone repo |
| 2c | Server | Create `.env` |
| 2d | Server | `docker login ghcr.io` |
| 2e | Server | SSH key in `authorized_keys` |
| 3a | GitHub | Add deploy secrets |
| 3b | GitHub | `DEPLOY_ON_RELEASE=true` |
| 4 | GitHub | Create a release (e.g. v1.0.0) |
