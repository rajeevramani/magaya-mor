# Platform API

Flowplane's Platform API provides a simplified, business-oriented interface for managing API gateways and services. It abstracts away the complexity of Envoy configuration while maintaining cross-API visibility with the Native API.

## Overview

The Platform API consists of three main components:
1. **API Definitions** - High-level API gateway configurations with routes, policies, and upstreams
2. **Services** - Backend service definitions with endpoints, load balancing, and health checks
3. **OpenAPI Import** - Import OpenAPI specifications with custom x-flowplane-* extensions

## Audience & Scopes

- **Audience**: Platform teams, developers, and API product owners who need simplified API management
- **Required scopes**: Vary by operation (see endpoint documentation below)
- **Audit**: Every mutation is logged via `AuditLogRepository`
- **Cross-API Visibility**: Resources created via Platform API are visible in Native API and vice versa

## API Definitions

### Endpoints

| Method | Path | Description | Required Scopes |
|--------|------|-------------|-----------------|
| `GET` | `/api/v1/platform/apis` | List all API definitions | `apis:read` |
| `POST` | `/api/v1/platform/apis` | Create a new API definition | `apis:write, route-configs:write, listeners:write, clusters:write` |
| `GET` | `/api/v1/platform/apis/{id}` | Get an API definition by ID | `apis:read` |
| `PUT` | `/api/v1/platform/apis/{id}` | Update an API definition | `apis:write, route-configs:write, listeners:write, clusters:write` |
| `DELETE` | `/api/v1/platform/apis/{id}` | Delete an API definition | `apis:write, route-configs:write, listeners:write, clusters:write` |

### Create API Definition

```json
POST /api/v1/platform/apis
{
  "name": "payments-api",
  "version": "1.0.0",
  "basePath": "/api/v1/payments",
  "upstream": {
    "service": "payments-backend",
    "endpoints": [
      {
        "host": "payments.svc.cluster.local",
        "port": 8443
      }
    ],
    "tls": true,
    "loadBalancing": "ROUND_ROBIN"
  },
  "routes": [
    {
      "path": "/transactions",
      "methods": ["GET", "POST"],
      "description": "Transaction management"
    },
    {
      "path": "/accounts/{id}",
      "methods": ["GET"],
      "description": "Account details"
    }
  ],
  "policies": {
    "rateLimit": {
      "requests": 100,
      "interval": "1m"
    },
    "authentication": {
      "authType": "jwt",
      "required": true,
      "config": {
        "issuer": "https://auth.example.com",
        "audience": "payments-api"
      }
    },
    "cors": {
      "origins": ["https://app.example.com"],
      "methods": ["GET", "POST", "OPTIONS"],
      "headers": ["Content-Type", "Authorization"],
      "allowCredentials": true,
      "maxAge": 3600
    }
  }
}
```

### Response

```json
{
  "id": "5b9b6a6d-8b81-4d62-92f4-7e9355d8f5c3",
  "name": "payments-api",
  "version": "1.0.0",
  "basePath": "/api/v1/payments",
  "upstream": {...},
  "routes": [...],
  "policies": {...},
  "clusterId": "payments-api-cluster",
  "routeConfigId": "payments-api-routes",
  "listenerId": "payments-api-listener",
  "createdAt": "2025-01-15T10:00:00Z",
  "updatedAt": "2025-01-15T10:00:00Z"
}
```

## Services

### Endpoints

| Method | Path | Description | Required Scopes |
|--------|------|-------------|-----------------|
| `GET` | `/api/v1/platform/services` | List all services | `services:read` |
| `POST` | `/api/v1/platform/services` | Create a new service | `services:write` |
| `GET` | `/api/v1/platform/services/{name}` | Get a service by name | `services:read` |
| `PUT` | `/api/v1/platform/services/{name}` | Update a service | `services:write` |
| `DELETE` | `/api/v1/platform/services/{name}` | Delete a service | `services:write` |

### Create Service

```json
POST /api/v1/platform/services
{
  "name": "user-service",
  "endpoints": [
    {
      "host": "user-1.svc.cluster.local",
      "port": 8080,
      "weight": 50
    },
    {
      "host": "user-2.svc.cluster.local",
      "port": 8080,
      "weight": 50
    }
  ],
  "loadBalancing": "ROUND_ROBIN",
  "healthCheck": {
    "path": "/health",
    "interval": 10,
    "timeout": 5,
    "healthyThreshold": 2,
    "unhealthyThreshold": 3
  },
  "circuitBreaker": {
    "maxRequests": 100,
    "maxPendingRequests": 50,
    "maxConnections": 100,
    "maxRetries": 3,
    "consecutiveErrors": 5,
    "intervalMs": 10000
  },
  "outlierDetection": {
    "consecutive5xx": 5,
    "intervalMs": 30000,
    "baseEjectionTimeMs": 30000,
    "maxEjectionPercent": 50,
    "minHealthyPercent": 30
  }
}
```

### Load Balancing Strategies

- `ROUND_ROBIN` - Distribute requests evenly across endpoints
- `LEAST_REQUEST` - Route to endpoint with fewest active requests
- `RANDOM` - Random endpoint selection
- `RING_HASH` - Consistent hashing
- `MAGLEV` - Maglev consistent hashing

## OpenAPI Import

### Endpoint

| Method | Path | Description | Required Scopes |
|--------|------|-------------|-----------------|
| `POST` | `/api/v1/platform/import/openapi` | Import an OpenAPI specification | `apis:write, import:write` |

### Import Request

```bash
curl -X POST http://localhost:8080/api/v1/platform/import/openapi?name=my-api \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/yaml" \
  --data-binary @openapi.yaml
```

### Flowplane Extensions

The OpenAPI import supports custom x-flowplane-* extensions for configuring filters and policies:

```yaml
openapi: 3.0.0
info:
  title: My API
  version: 1.0.0
servers:
  - url: https://api.example.com
paths:
  /users:
    get:
      summary: List users
      x-flowplane-ratelimit:
        requests: 100
        interval: "1m"
      x-flowplane-jwt-auth:
        required: true
        issuer: "https://auth.example.com"
      x-flowplane-cors:
        origins: ["https://app.example.com"]
        methods: ["GET", "POST"]
        headers: ["Content-Type", "Authorization"]
        allowCredentials: true
      responses:
        '200':
          description: Success
```

### Import Response

```json
{
  "id": "generated-api-id",
  "name": "my-api",
  "version": "1.0.0",
  "basePath": "/",
  "upstream": {
    "service": "my-api-backend",
    "endpoints": [{
      "host": "api.example.com",
      "port": 443
    }],
    "tls": true
  },
  "routes": [...],
  "policies": {
    "rateLimit": {...},
    "authentication": {...},
    "cors": {...}
  },
  "createdAt": "2025-01-15T10:00:00Z",
  "metadata": {
    "source": "openapi_import",
    "openapi_version": "3.0.0"
  }
}
```

## Cross-API Visibility

Resources created through the Platform API are automatically visible in the Native API:

- **API Definitions** create corresponding:
  - Clusters (visible at `/api/v1/clusters`)
  - Route configurations (visible at `/api/v1/route-configs`)
  - Listeners (visible at `/api/v1/listeners`)

- **Services** create corresponding:
  - Clusters with the naming convention `{service-name}-cluster`

Similarly, Native API resources are visible in Platform API views when they match Platform conventions.

## Policy Configuration

### Rate Limiting

```json
"rateLimit": {
  "requests": 100,
  "interval": "1m"  // Supports: 1s, 1m, 1h, 1d
}
```

### JWT Authentication

```json
"authentication": {
  "authType": "jwt",
  "required": true,
  "config": {
    "issuer": "https://auth.example.com",
    "audience": "my-api",
    "jwksUri": "https://auth.example.com/.well-known/jwks.json"
  }
}
```

### CORS

```json
"cors": {
  "origins": ["https://app.example.com", "https://admin.example.com"],
  "methods": ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
  "headers": ["Content-Type", "Authorization", "X-Request-ID"],
  "allowCredentials": true,
  "maxAge": 3600
}
```

### Circuit Breaker

```json
"circuitBreaker": {
  "maxRequests": 100,
  "maxPendingRequests": 50,
  "maxConnections": 100,
  "maxRetries": 3,
  "consecutiveErrors": 5,
  "intervalMs": 10000
}
```

## Error Responses

All errors follow a consistent format:

```json
{
  "error": "bad_request",
  "message": "Validation failed: name is required"
}
```

Common error codes:
- `bad_request` (400) - Invalid request data
- `unauthorized` (401) - Missing or invalid authentication
- `forbidden` (403) - Insufficient permissions
- `not_found` (404) - Resource not found
- `conflict` (409) - Resource already exists
- `internal_server_error` (500) - Unexpected server error

## Migration Guide

### From Legacy Gateway API

The OpenAPI import endpoint has moved:
- **Old**: `/api/v1/gateways/openapi`
- **New**: `/api/v1/platform/import/openapi`

The old endpoint automatically redirects to the new location with a 301 status code.

### From Native API

To migrate from Native API to Platform API:

1. **Clusters** → **Services**:
   - Extract endpoints from cluster configuration
   - Map load balancing policies
   - Convert health checks and circuit breakers

2. **Route Configs + Listeners** → **API Definitions**:
   - Combine route and listener configuration
   - Extract base paths and domains
   - Convert filter chains to policies

## Best Practices

1. **Use API Definitions** for public-facing APIs that need standardized policies
2. **Use Services** for internal service discovery and load balancing
3. **Import OpenAPI** specs to maintain API documentation as source of truth
4. **Leverage Cross-API Visibility** to gradually migrate from Native to Platform API
5. **Apply Policies Consistently** across all routes in an API definition

## Related Documentation

- [`/docs/api.md`](../api.md) - Main API reference
- [`/specs/007-two-path-api-architecture/spec.md`](../../specs/007-two-path-api-architecture/spec.md) - Architecture specification
- [`/tests/platform_api/`](../../tests/platform_api/) - Integration test examples