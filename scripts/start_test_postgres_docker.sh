#!/bin/bash
set -euo pipefail
set -x

cd $(realpath $(dirname $0))
source ./env.sh

TEST_DATABASE_CONTAINER_NAME=${TEST_DATABASE_CONTAINER_NAME:-postgres-pic-store-test}
TEST_DATABASE_USER=${TEST_DATABASE_USER:-postgres}
TEST_DATABASE_PORT=${TEST_DATABASE_PORT:-6500}
TEST_DATABASE_PASSWORD=${TEST_DATABASE_PASSWORD:-pic-store-test-db}

docker run -d  \
  -e "POSTGRES_USER=${TEST_DATABASE_USER}" \
  -e "POSTGRES_PASSWORD=${TEST_DATABASE_PASSWORD}" \
  -p ${TEST_DATABASE_PORT}:5432 \
  --name ${TEST_DATABASE_CONTAINER_NAME} \
  postgres:14 postgres -N 1000
