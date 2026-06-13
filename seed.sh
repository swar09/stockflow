#!/usr/bin/env bash
# ============================================================================
# seed.sh
#
# Loads DB credentials from a .env file and runs seed.sql against a
# PostgreSQL instance running inside a Docker container.
#
# Usage:
#   ./seed.sh [path-to-env-file] [path-to-seed-sql]
#
# Defaults:
#   env file  = ./.env
#   seed file = ./seed.sql
#
# Expected .env variables (common Postgres-in-Docker naming):
#   POSTGRES_USER=...
#   POSTGRES_PASSWORD=...
#   POSTGRES_DB=...
#   POSTGRES_CONTAINER=my-postgres-container   (optional, auto-detected if omitted)
#   POSTGRES_HOST=localhost                    (only used for local psql fallback)
#   POSTGRES_PORT=5432                         (only used for local psql fallback)
# ============================================================================

set -euo pipefail

ENV_FILE="${1:-.env}"
SEED_FILE="${2:-seed.sql}"

if [[ ! -f "$ENV_FILE" ]]; then
    echo "ERROR: env file '$ENV_FILE' not found." >&2
    exit 1
fi

if [[ ! -f "$SEED_FILE" ]]; then
    echo "ERROR: seed file '$SEED_FILE' not found." >&2
    exit 1
fi

# --- Load .env -------------------------------------------------------------
set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

# --- Required variables -----------------------------------------------------
: "${POSTGRES_USER:?POSTGRES_USER is not set in $ENV_FILE}"
: "${POSTGRES_DB:?POSTGRES_DB is not set in $ENV_FILE}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is not set in $ENV_FILE}"

# --- Resolve the docker container ------------------------------------------
CONTAINER_NAME="${POSTGRES_CONTAINER:-}"

if [[ -z "$CONTAINER_NAME" ]]; then
    echo "POSTGRES_CONTAINER not set, attempting to auto-detect a running postgres container..."

    # Try matching by image name first
    CONTAINER_NAME=$(docker ps --filter "ancestor=postgres" --format '{{.Names}}' | head -n1 || true)

    # Fallback: try matching container names containing "postgres" or "pg"
    if [[ -z "$CONTAINER_NAME" ]]; then
        CONTAINER_NAME=$(docker ps --format '{{.Names}}' | grep -iE 'postgres|pg' | head -n1 || true)
    fi
fi

if [[ -z "$CONTAINER_NAME" ]]; then
    echo "ERROR: Could not auto-detect a running postgres container." >&2
    echo "       Run 'docker ps' to find the right name and set" >&2
    echo "       POSTGRES_CONTAINER=<name> in $ENV_FILE, then re-run." >&2
    exit 1
fi

echo "----------------------------------------------------------------------"
echo "Container : $CONTAINER_NAME"
echo "Database  : $POSTGRES_DB"
echo "User      : $POSTGRES_USER"
echo "Seed file : $SEED_FILE"
echo "----------------------------------------------------------------------"

# --- Run the seed file ------------------------------------------------------
docker exec -i \
    -e PGPASSWORD="$POSTGRES_PASSWORD" \
    "$CONTAINER_NAME" \
    psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 \
    < "$SEED_FILE"

echo "----------------------------------------------------------------------"
echo "Seeding complete."