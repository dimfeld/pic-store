#!/bin/bash
set -euo pipefail

if [ -f ../.env ]; then
  set -o allexport
  source ../.env
  set +o allexport
fi

export ENV=${ENV:=development}

export DATABASE_NAME=${DATABASE_NAME:=pic_store}
export DATABASE_HOST=${DATABASE_HOST:=%2Fvar%2Frun%2Fpostgresql}
export DATABASE_PORT=${DATABASE_PORT:=5432}
