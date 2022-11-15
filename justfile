_default:
  @just --list

# Start a PostgreSQL docker container for creating test databases
start-test-postgres-docker:
  scripts/start_test_postgres_docker.sh
