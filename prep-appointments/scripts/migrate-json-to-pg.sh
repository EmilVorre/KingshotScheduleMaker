#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   DATABASE_URL=postgres://... ./scripts/migrate-json-to-pg.sh /path/to/data [--dry-run]

DATA_DIR="${1:-data}"
MODE="${2:-}"

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required"
  exit 1
fi

DRY_RUN="false"
if [[ "$MODE" == "--dry-run" ]]; then
  DRY_RUN="true"
fi

echo "Applying SQL schema..."
psql "$DATABASE_URL" -f prep-appointments/migrations/0001_init.sql

echo "Running migration binary..."
DRY_RUN="$DRY_RUN" \
MIGRATE_DATA_DIR="$DATA_DIR" \
DATABASE_URL="$DATABASE_URL" \
cargo run --release --bin migrate_json_to_pg --manifest-path prep-appointments/Cargo.toml
