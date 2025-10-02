# HTTP API Reference

Flowplane exposes a REST API on the configured bind address (defaults to `127.0.0.1:8080`). Every
request must carry a valid bearer token with scopes that match the requested resource. See
[`docs/authentication.md`](docs/authentication.md) for a detailed overview of personal access tokens
and scope assignments.

The OpenAPI document is always available at `/api-docs/openapi.json`; an interactive Swagger UI is
served from `/swagger-ui`.

## Authentication Header

```
Authorization: Bearer fp_pat_<token-id>.<secret>
```

Tokens are checked for scope membership before handlers execute. Failure to present a credential or
attempting an operation without the matching scope yields a `401`/`403` error with a sanitized body:

```json
{
  "error": "unauthorized",
  "message": "missing or invalid bearer"
}
```

## Token Management

| Endpoint | Method | Scope | Description |
|----------|--------|-------|-------------|
| `/api/v1/tokens` | `POST` | `tokens:write` | Issue a new personal access token. Returns the token once. |
| `/api/v1/tokens` | `GET` | `tokens:read` | List tokens with pagination. |
| `/api/v1/tokens/{id}` | `GET` | `tokens:read` | Retrieve token metadata. Secret is never returned. |
| `/api/v1/tokens/{id}` | `PATCH` | `tokens:write` | Update name, description, status, scopes, or expiration. |
| `/api/v1/tokens/{id}` | `DELETE` | `tokens:write` | Revoke a token (status becomes `revoked`). |
| `/api/v1/tokens/{id}/rotate` | `POST` | `tokens:write` | Rotate the secret. Response contains the new token once. |

### Create a Token

```bash
curl -sS \
  -X POST http://127.0.0.1:8080/api/v1/tokens \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
        "name": "ci-pipeline",
        "description": "Token used by CI deployments",
        "scopes": ["clusters:write", "routes:write", "listeners:read"],
        "expiresAt": null
      }'
```

Successful responses return `201 Created` and the new token in the body:

```json
{
  "id": "8a6f9d37-9a4c-4dbe-a494-9bd924dbd1b1",
  "token": "fp_pat_8a6f9d37-9a4c-4dbe-a494-9bd924dbd1b1.CJ7p..."
}
```

### List Tokens

```bash
curl -sS \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://127.0.0.1:8080/api/v1/tokens?limit=20&offset=0"
```

Returns a JSON array of token records (without secrets):

```json
[
  {
    "id": "8a6f9d37-9a4c-4dbe-a494-9bd924dbd1b1",
    "name": "ci-pipeline",
    "status": "active",
    "scopes": ["clusters:write", "routes:write", "listeners:read"],
    "expiresAt": null,
    "lastUsedAt": "2025-01-05T17:10:22Z"
  }
]
```

### Rotate a Token

```bash
curl -sS \
  -X POST http://127.0.0.1:8080/api/v1/tokens/8a6f9d37-9a4c-4dbe-a494-9bd924dbd1b1/rotate \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

Response contains the new token value. Update dependent systems immediately—previous secrets stop
working as soon as the rotation succeeds.

## Native API Endpoints

The Native API provides direct control over Envoy configuration primitives:

| Endpoint | Method | Scope | Description |
|----------|--------|-------|-------------|
| `/api/v1/clusters` | `GET` | `clusters:read` | List all clusters |
| `/api/v1/clusters` | `POST` | `clusters:write` | Create a new cluster |
| `/api/v1/clusters/{name}` | `GET` | `clusters:read` | Get a specific cluster |
| `/api/v1/clusters/{name}` | `PUT`/`DELETE` | `clusters:write` | Update or delete a cluster |
| `/api/v1/route-configs` | `GET` | `route-configs:read` | List all route configurations |
| `/api/v1/route-configs` | `POST` | `route-configs:write` | Create a new route config |
| `/api/v1/route-configs/{name}` | `GET` | `route-configs:read` | Get a specific route config |
| `/api/v1/route-configs/{name}` | `PUT`/`DELETE` | `route-configs:write` | Update or delete route config |
| `/api/v1/listeners` | `GET` | `listeners:read` | List all listeners |
| `/api/v1/listeners` | `POST` | `listeners:write` | Create a new listener |
| `/api/v1/listeners/{name}` | `GET` | `listeners:read` | Get a specific listener |
| `/api/v1/listeners/{name}` | `PUT`/`DELETE` | `listeners:write` | Update or delete a listener |

## Platform API Endpoints

The Platform API provides simplified, business-oriented abstractions:

| Endpoint | Method | Scope | Description |
|----------|--------|-------|-------------|
| `/api/v1/platform/apis` | `GET` | `apis:read` | List all API definitions |
| `/api/v1/platform/apis` | `POST` | `apis:write, route-configs:write, listeners:write, clusters:write` | Create a new API definition |
| `/api/v1/platform/apis/{id}` | `GET` | `apis:read` | Get an API definition by ID |
| `/api/v1/platform/apis/{id}` | `PUT` | `apis:write, route-configs:write, listeners:write, clusters:write` | Update an API definition |
| `/api/v1/platform/apis/{id}` | `DELETE` | `apis:write, route-configs:write, listeners:write, clusters:write` | Delete an API definition |
| `/api/v1/platform/services` | `GET` | `services:read` | List all services |
| `/api/v1/platform/services` | `POST` | `services:write` | Create a new service |
| `/api/v1/platform/services/{name}` | `GET` | `services:read` | Get a service by name |
| `/api/v1/platform/services/{name}` | `PUT` | `services:write` | Update a service |
| `/api/v1/platform/services/{name}` | `DELETE` | `services:write` | Delete a service |
| `/api/v1/platform/import/openapi` | `POST` | `apis:write, import:write` | Import an OpenAPI specification |

### Legacy Redirects

| Endpoint | Method | Redirects To | Description |
|----------|--------|--------------|-------------|
| `/api/v1/gateways/openapi` | `POST` | `/api/v1/platform/import/openapi` | Legacy OpenAPI import endpoint (deprecated) |

Each request returns a structured error payload on validation or authorization failure, and logs an
audit entry for traceability.

## Observability Endpoints

- `/healthz` – control plane readiness (no auth required).
- `/metrics` – Prometheus exporter. Requires metrics to be enabled via configuration. The exporter is
  bound to the address returned by `ObservabilityConfig::metrics_bind_address`.

## CLI vs API

The CLI uses the same service layer as the HTTP API. If you prefer terminal workflows, run
`flowplane auth --help` for usage examples or see [`docs/token-management.md`](docs/token-management.md).
