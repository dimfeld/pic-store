set dotenv-load

server := env_var_or_default('PIC_STORE_SERVER_URL', 'http://localhost:7205')

_default:
  @just --list

# Start a PostgreSQL docker container for creating test databases
start-test-postgres-docker:
  scripts/start_test_postgres_docker.sh

send-request method url *body='':
  http -b -A {{env_var('DEFAULT_API_KEY')}} {{method}} {{server}}{{url}} {{body}}

upload-image imagepath:
  #!/usr/bin/env zx
  const file = `{{imagepath}}`;
  const filename = path.basename(file);

  const createReq = await $`http -b -A ${process.env.DEFAULT_API_KEY} POST {{server}}/images filename=${filename}`;
  const id = JSON.parse(createReq.stdout).id;

  const url = `{{server}}/images/${id}/upload`;
  console.log('Uploading image ID ${id}');
  await spinner(() => $`http -b -A ${process.env.DEFAULT_API_KEY}` POST ${url} @${file}`);

image-status id:
  just send-request GET /images/{{id}}

