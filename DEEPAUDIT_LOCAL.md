# DeepAudit (local, no external registration)

This repo includes a local Docker Compose deployment for DeepAudit (frontend + backend + postgres + redis) configured to use the host Ollama instance via the OpenAI-compatible API.

## URLs

- Frontend UI: http://localhost:3010/
- Backend health: http://localhost:8000/health
- Backend Swagger UI: http://localhost:8000/docs
- Backend OpenAPI schema: http://localhost:8000/api/v1/openapi.json

## “Sign in” vs “registration”

DeepAudit has its own **local** user accounts (for projects, reports, access control). This is **not** “registering with a third-party API provider”.

The backend initializes a demo account by default:

- Email (username): `demo@example.com`
- Password: `demo123`

You can also create your own local account via the UI or the API.

### API login (example)

The login endpoint uses `application/x-www-form-urlencoded` (OAuth2 password flow):

- `POST http://localhost:8000/api/v1/auth/login`
- Form fields: `username`, `password`

## Ollama (local LLM)

The Compose file is set up to reach Ollama from containers using:

- `LLM_BASE_URL=http://host.docker.internal:11434/v1`

On Linux, this works because the backend service includes:

- `extra_hosts: "host.docker.internal:host-gateway"`

…and the host Ollama service is configured to listen on the Docker bridge IP (`docker0`).

## Config file

- Compose file: `deepaudit.docker-compose.prod.yml`

If you have port conflicts (e.g. Next.js dev servers), adjust the frontend port mapping there.
