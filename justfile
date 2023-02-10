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

  const createReq = await $`http -b -A bearer -a ${process.env.DEFAULT_API_KEY} POST {{server}}/api/images filename=${filename}`;
  const id = JSON.parse(createReq.stdout).id;

  const url = `{{server}}/api/images/${id}`;
  console.log('Uploading image ID ${id}');
  await $`http -b -A bearer -a ${process.env.DEFAULT_API_KEY} POST ${url}/upload @${file}`;
  await $`http -b -A bearer -a ${process.env.DEFAULT_API_KEY} GET ${url}`;

image-status id:
  just send-request GET /images/{{id}}

