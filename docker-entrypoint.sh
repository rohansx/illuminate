#!/bin/sh
set -e

echo "Running database migrations..."
dbmate --url "$DATABASE_URL" --migrations-dir ./migrations --no-dump-schema up

echo "Starting illuminate server..."
exec ./illuminate
