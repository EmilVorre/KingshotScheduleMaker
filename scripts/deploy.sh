#!/usr/bin/env bash
# Run this on the server after pulling. Requires: Node.js, Rust, .env file.
# Usage: ./scripts/deploy.sh [port]
# Port defaults to 8080.

set -e
cd "$(dirname "$0")/.."
PORT="${1:-8080}"

echo "==> Loading environment from .env (if present)"
if [ -f .env ]; then
  set -a
  source .env
  set +a
  echo "    Loaded .env"
else
  echo "    No .env found - using existing environment"
fi

echo "==> Building frontend"
cd frontend
npm ci
npm run build
cd ..

echo "==> Building backend (release)"
cd prep-appointments
cargo build --release
cd ..

echo "==> Deploy complete. Start with:"
echo "    cd prep-appointments && ./scripts/run.sh $PORT"
echo ""
echo "Ensure .env exists in the project root with your secrets."
echo ""
echo "Or with systemd (if configured):"
echo "    sudo systemctl restart prep-appointments"
