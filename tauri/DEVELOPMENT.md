# Hopp Development Guide

## Requirements

- [Task](https://taskfile.dev/docs/installation)
- [Rust](https://www.rust-lang.org/tools/install)
- [Go](https://go.dev/doc/install)
- [Node.js](https://nodejs.org/en/download/)
- [Yarn](https://yarnpkg.com/getting-started/install)
- [tauri](https://v2.tauri.app/start/prerequisites/)
- [Docker](https://docs.docker.com/get-docker/)
- [livekit-server](https://docs.livekit.io/home/self-hosting/local/)
- [psql](https://www.postgresql.org/docs/current/app-psql.html)
- [mkcert](https://github.com/FiloSottile/mkcert)

### Linux specific

On Linux, you need to install the following packages:

```bash
libasound2-dev libglib2.0-dev
```

## Development Workflow

### Backend

Before the first run, you need to setup the database and the redis docker images, also you need to create the certificates.

```bash
task backend:create-certs
task backend:compose-up
task backend:add-mock-data
```

Mock data adds the following users to the database:

TODO: make a command for this
You can get a token with the following command:

```bash
curl --request GET --url https://localhost:1926/api/jwt-debug\?email\=test@test.com
```

After the initial setup, you can start the backend with the local livekit server with the following command:

```bash
task dev-server
```

### Tauri App

Either from the the top folder or from the tauri folder, you can run the following command to start the app:

```bash
task tauri:dev
```

For debugging you can spawn a clone of the app with the following command:

```bash
task tauri:start-replica-app
```

## Exposing the backend and the livekit server

You can expose the backend and the livekit server to the internet by creating a tunnel. There are
multiple tools to do that, one of them is to use `cloudflared`

Example configuration
```
tunnel: <tunnel-id>
credentials-file: <path-to-credentials-file>

ingress:
  - hostname: <hostname>
    service: https://localhost:1926

  - hostname: <livekit_hostname>
    service: ws://localhost:7880

  - service: http_status:404
```

### Env files

#### Tauri
```.env
VITE_API_BASE_URL="localhost:1926"
```