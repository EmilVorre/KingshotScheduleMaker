# Deployment Guide

Deploy with minimal manual steps: store secrets once in `.env`, then use a simple deploy flow.

## One-time setup on the server

### 1. Create `.env` (secrets live here, never in git)

```bash
cd /path/to/KingshotScheduleMaker
cp .env.example .env
nano .env   # or vim, etc.
```

Fill in all values. See [OAUTH_SETUP.md](OAUTH_SETUP.md) for OAuth credentials.

### 2. Ensure data directory exists

```bash
mkdir -p prep-appointments/data
```

---

## Deploy workflow

### Option A: Manual deploy (improved)

```bash
# On your machine
git push origin main

# On the server
cd /path/to/KingshotScheduleMaker
git pull
./scripts/deploy.sh 8080
cd prep-appointments && ./scripts/run.sh 8080
```

No secrets to re-enter. The `.env` file stays on the server and is loaded automatically.

### Option B: GitHub Actions (fully automated)

Push to `main` → GitHub Actions builds and deploys via SSH.

**Setup:**

1. Add these **GitHub Secrets** (Settings → Secrets and variables → Actions):
   - `DEPLOY_HOST` – server hostname or IP
   - `DEPLOY_USER` – SSH user (e.g. `root` or `deploy`)
   - `DEPLOY_KEY` – private SSH key (contents of `id_rsa` or similar)
   - `DEPLOY_PATH` – project path on server (e.g. `/home/deploy/KingshotScheduleMaker`)

2. Ensure the server has:
   - `.env` with all secrets (one-time setup)
   - SSH key added to `~/.ssh/authorized_keys` for `DEPLOY_USER`
   - Git (for `git pull`). Node/Rust not required – the workflow builds and copies artifacts.

3. Optional: Add `DEPLOY_RESTART_CMD` secret (e.g. `sudo systemctl restart prep-appointments`) for a clean restart instead of pkill + nohup.

4. Push to `main` – the workflow builds and deploys automatically.

---

## Process manager (recommended for production)

### systemd

Create `/etc/systemd/system/prep-appointments.service`:

```ini
[Unit]
Description=Prep Appointments Web Server
After=network.target

[Service]
Type=simple
User=deploy
WorkingDirectory=/path/to/KingshotScheduleMaker/prep-appointments
EnvironmentFile=/path/to/KingshotScheduleMaker/.env
ExecStart=/path/to/KingshotScheduleMaker/prep-appointments/target/release/prep-appointments web 8080
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable prep-appointments
sudo systemctl start prep-appointments
```

Deploy becomes: `git pull && ./scripts/deploy.sh && sudo systemctl restart prep-appointments`

---

## Environment variables reference

| Variable | Required | Description |
|----------|----------|-------------|
| `OAUTH_DISCORD_CLIENT_ID` | For Discord login | Discord app client ID |
| `OAUTH_DISCORD_CLIENT_SECRET` | For Discord login | Discord app secret |
| `OAUTH_GOOGLE_CLIENT_ID` | For Google login | Google OAuth client ID |
| `OAUTH_GOOGLE_CLIENT_SECRET` | For Google login | Google OAuth secret |
| `BASE_URL` | Production | e.g. `https://prep.vorre.dev` |
| `FRONTEND_URL` | If frontend separate | Usually same as BASE_URL in prod |
