

set -euo pipefail


ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"


USE_PODMAN_COMPOSE=0
USE_DOCKER_COMPOSE=0

if command -v podman >/dev/null 2>&1; then
  if podman compose version >/dev/null 2>&1; then
    USE_PODMAN_COMPOSE=1
  else
    USE_PODMAN_COMPOSE=0
  fi
fi

if [ "$USE_PODMAN_COMPOSE" -eq 1 ]; then
  echo "Using: podman compose"
  echo "Starting Postgres via podman compose..."
  podman compose up -d db

  echo "Waiting for Postgres to become available..."
  podman compose exec -T db bash -c 'until pg_isready -U "${POSTGRES_USER:=postgres}" -d "${POSTGRES_DB:=engine_dev}" >/dev/null 2>&1; do sleep 1; done'

  echo "Applying migrations..."
  for f in migrations/*.sql; do
    echo "Applying $f"
    podman compose exec -T db psql -U "${POSTGRES_USER:=postgres}" -d "${POSTGRES_DB:=engine_dev}" -f /migrations/$(basename "$f")
  done

  echo "Migrations applied. DB is ready."
  exit 0
fi

if command -v docker >/dev/null 2>&1; then
  echo "Using: docker compose"
  echo "Starting Postgres via docker compose..."
  docker compose up -d db

  echo "Waiting for Postgres to become available..."
  docker compose exec -T db bash -c 'until pg_isready -U "${POSTGRES_USER:=postgres}" -d "${POSTGRES_DB:=engine_dev}" >/dev/null 2>&1; do sleep 1; done'

  echo "Applying migrations..."
  for f in migrations/*.sql; do
    echo "Applying $f"
    docker compose exec -T db psql -U "${POSTGRES_USER:=postgres}" -d "${POSTGRES_DB:=engine_dev}" -f /migrations/$(basename "$f")
  done

  echo "Migrations applied. DB is ready."
  exit 0
fi


if command -v podman >/dev/null 2>&1; then
  echo "podman compose not available; starting postgres container with podman run"


  if ! podman volume inspect db_data >/dev/null 2>&1; then
    podman volume create db_data >/dev/null
  fi


  podman rm -f engine-db >/dev/null 2>&1 || true

  IMAGE=${IMAGE:-docker.io/library/postgres:15}

  podman run -d --name engine-db \
    -e POSTGRES_USER=${POSTGRES_USER:-postgres} \
    -e POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-password} \
    -e POSTGRES_DB=${POSTGRES_DB:-engine_dev} \
    -p 5432:5432 \
    -v db_data:/var/lib/postgresql/data:Z \
    -v "$ROOT_DIR/migrations":/migrations:ro,Z \
    $IMAGE

  echo "Waiting for Postgres to become available..."
  podman exec -i engine-db bash -c 'until pg_isready -U "${POSTGRES_USER:=postgres}" -d "${POSTGRES_DB:=engine_dev}" >/dev/null 2>&1; do sleep 1; done'

  echo "Applying migrations..."
  for f in migrations/*.sql; do
    echo "Applying $f"
    podman exec -i engine-db psql -U "${POSTGRES_USER:=postgres}" -d "${POSTGRES_DB:=engine_dev}" -f /migrations/$(basename "$f")
  done

  echo "Migrations applied. DB is ready."
  exit 0
fi

echo "Neither podman nor docker found with usable compose; please install podman or docker." >&2
exit 1

