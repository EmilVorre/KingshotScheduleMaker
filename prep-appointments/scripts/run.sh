#!/usr/bin/env bash
# Run the web server with env vars from .env
# Usage: ./scripts/run.sh [port]

set -e
cd "$(dirname "$0")/.."
PORT="${1:-8080}"

# Load .env from repo root (one level up)
if [ -f ../.env ]; then
  set -a
  source ../.env
  set +a
fi

# Prefer release binary (from deploy), fallback to debug
if [ -f ./target/release/prep-appointments ]; then
  exec ./target/release/prep-appointments web "$PORT"
else
  exec cargo run --release -- web "$PORT"
fi
