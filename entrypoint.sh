#!/bin/bash
POSTGRES_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}:${POSTGRES_PORT}/${POSTGRES_NAME}"
echo "Waiting for PostgreSQL to start..."
while ! pg_isready -h "${POSTGRES_HOST}" -p "${POSTGRES_PORT}" -U "${POSTGRES_USER}"; do
  sleep 1
done
echo "Running sqlx database create..."
sqlx database create --database-url $POSTGRES_URL
echo "Running sqlx migrate run..."
sqlx migrate run --database-url $POSTGRES_URL
