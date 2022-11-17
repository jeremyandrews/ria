#!/bin/sh
# wait-for-postgres.sh

POSTGRES_HOST=$1
POSTGRES_USER=$2
POSTGRES_PASSWORD=$3
POSTGRES_DB=$4

set -e

# Login for user (`-U`) and once logged in execute quit ( `-c \q` )
# If we can not login sleep for 1 sec
until PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -U ${POSTGRES_USER} -c '\q'; do
  >&2 echo "Postgres is unavailable - sleeping"
  sleep 1
done
  
>&2 echo "Postgres is up"

# Prepare the Ria schema. (This will delete any existing data!)
DATABASE_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@${POSTGRES_HOST}/${POSTGRES_DB}" sea-orm-cli migrate fresh

# Technically the log file doesn't have to exist, but by touching it we avoid a
# potentially confusing warning when it doesn't exist.
/usr/bin/touch /app/ria.log
>&2 echo "Ria is ready"
/usr/bin/tail -F /app/ria.log
