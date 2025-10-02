# API Specification

This is the API specification for the spec detailed in @.agent-os/specs/2025-10-02-two-path-api-architecture/spec.md

## Endpoints

### Native API (EXISTING - No Changes)

These endpoints already exist and work well. They may optionally be aliased under `/api/v1/native/*` for organizational clarity.

#### Existing Endpoints (Unchanged)
- `GET /api/v1/clusters` - List clusters with full Envoy configuration
- `POST /api/v1/clusters` - Create cluster with Envoy configuration
- `PUT /api/v1/clusters/{id}` - Update cluster
- `DELETE /api/v1/clusters/{id}` - Delete cluster

- `GET /api/v1/listeners` - List listeners with filter chains
- `POST /api/v1/listeners` - Create listener
- `PUT /api/v1/listeners/{id}` - Update listener
- `DELETE /api/v1/listeners/{id}` - Delete listener

#### Rename the following Endpoints to be better aligned with envoy
- `routes` -> `route-configs`
- `GET /api/v1/routes` -> `GET /api/v1/route-configs` - List routes with full configuration
- `POST /api/v1/routes` -> `POST /api/v1/route-configs` - Create route
- `PUT /api/v1/routes/{id}` -> `PUT /api/v1/route-configs/{id}` - Update route
- `DELETE /api/v1/routes/{id}` -> `DELETE /api/v1/route-configs/{id}` - Delete route



**Note**: These APIs continue to work exactly as before. Full documentation already exists in OpenAPI specs. Modify them based on the changes to routes. Make sure the test cases are also updated.

### Platform API (NEW - Primary Focus)

New endpoints providing simplified abstractions over Native APIs.

#### GET /api/v1/platform/services

**Purpose:** List services (abstraction over clusters)
**Response:**
```json
{
  "services": [
    {
      "name": "user-service",
      "endpoints": ["backend-1:8080", "backend-2:8080"],
      "health_status": "healthy",
      "load_balancing": "round_robin",
      "timeout_seconds": 30
    }
  ]
}
```
**Implementation:** Internally calls Native `GET /api/v1/clusters` and transforms response

#### POST /api/v1/platform/services

**Purpose:** Create service with simplified configuration
**Request Body:**
```json
{
  "name": "user-service",
  "endpoints": ["backend-1:8080", "backend-2:8080"],
  "health_check_path": "/health",
  "timeout_seconds": 30,
  "retry_policy": {
    "max_attempts": 3,
    "timeout_per_attempt": 10
  }
}
```
**Response:** Created service details
**Implementation:** Transforms to Native `POST /api/v1/clusters` request

#### GET /api/v1/platform/apis

**Purpose:** List API definitions (abstraction over routes + listeners)
**Response:**
```json
{
  "apis": [
    {
      "name": "user-api",
      "domain": "api.example.com",
      "base_path": "/v1",
      "routes": [
        {
          "path": "/users/{id}",
          "method": "GET",
          "service": "user-service",
          "policies": {
            "rate_limit": "100/min",
            "authentication": "jwt"
          }
        }
      ]
    }
  ]
}
```
**Implementation:** Aggregates Native routes and listeners, presents simplified view

#### POST /api/v1/platform/apis

**Purpose:** Create API definition with routes
**Request Body:**
```json
{
  "name": "user-api",
  "domain": "api.example.com",
  "base_path": "/v1",
  "routes": [
    {
      "path": "/users/{id}",
      "method": "GET",
      "service": "user-service",
      "timeout": 30
    }
  ],
  "policies": {
    "rate_limit": { "requests_per_minute": 100 },
    "cors": { "origins": ["https://example.com"] }
  }
}
```
**Response:** Created API definition
**Implementation:**
1. Creates/updates listener via Native API
2. Creates routes via Native API
3. Applies filters via Native API

#### POST /api/v1/platform/import/openapi (MOVED)

**Purpose:** Import OpenAPI specification (relocated from `/api/v1/gateway/import`)
**Request Body:**
```json
{
  "openapi": "3.0.0",
  "info": { /* Standard OpenAPI info */ },
  "servers": [
    { "url": "https://backend.example.com" }
  ],
  "paths": {
    "/users/{id}": {
      "get": {
        "x-flowplane-ratelimit": {
          "requests_per_minute": 100
        },
        "x-flowplane-jwt-auth": {
          "issuer": "https://auth.example.com"
        },
        /* Standard OpenAPI operation */
      }
    }
  }
}
```
**Response:**
```json
{
  "imported": {
    "services": 1,
    "apis": 1,
    "routes": 5
  },
  "filters_applied": ["ratelimit", "jwt_auth"]
}
```
**Implementation:** Extended version of existing import with filter tag processing

#### GET /api/v1/platform/policies

**Purpose:** List available policies (filters) and their configurations
**Response:**
```json
{
  "policies": [
    {
      "name": "rate_limit",
      "type": "traffic_management",
      "configurable_fields": {
        "requests_per_minute": "integer",
        "burst_size": "integer"
      }
    },
    {
      "name": "jwt_auth",
      "type": "authentication",
      "configurable_fields": {
        "issuer": "string",
        "audiences": "array<string>"
      }
    }
  ]
}
```

## Controllers

### PlatformApiController (NEW)

**Primary Responsibility:** Transform between Platform abstractions and Native APIs

**Actions:**
- `create_service()`: Transform service definition → Native cluster API
- `create_api()`: Transform API definition → Multiple Native API calls
- `import_openapi()`: Process OpenAPI with filter tags → Native resources
- `list_services()`: Query Native clusters → Transform to service view
- `list_apis()`: Aggregate Native resources → Transform to API view

**Implementation Pattern:**
```rust
// Example: Platform Service → Native Cluster
pub async fn create_service(
    State(state): State<AppState>,
    Json(service): Json<ServiceRequest>,
) -> Result<Json<ServiceResponse>> {
    // Transform to Native API request
    let cluster_req = transform_service_to_cluster(service);

    // Call existing Native API handler
    let cluster = native_api::create_cluster(state, cluster_req).await?;

    // Transform response back to Platform view
    Ok(Json(transform_cluster_to_service(cluster)))
}
```

### OpenApiImportController (ENHANCED)

**Changes from existing:**
- Moved to `/api/v1/platform/import/openapi`
- Added `x-flowplane-*` tag processing
- Returns Platform API resource names

**Filter Tag Processing:**
```rust
fn extract_flowplane_filters(operation: &Operation) -> Vec<Filter> {
    let mut filters = vec![];

    if let Some(ext) = operation.extensions.get("x-flowplane-ratelimit") {
        filters.push(create_rate_limit_filter(ext));
    }

    if let Some(ext) = operation.extensions.get("x-flowplane-jwt-auth") {
        filters.push(create_jwt_filter(ext));
    }

    filters
}
```

## Migration Support

### Temporary Redirect
```rust
// Old endpoint redirects to new location
router.route("/api/v1/gateway/import",
    post(|body| async {
        // Log deprecation
        warn!("Using deprecated endpoint, redirecting to /api/v1/platform/import/openapi");

        // Redirect to new endpoint
        Redirect::permanent("/api/v1/platform/import/openapi")
    })
)
```

### Optional Native API Aliasing
```rust
// Both paths work during transition
router
    .route("/api/v1/clusters", get(native::list_clusters))
    .route("/api/v1/native/clusters", get(native::list_clusters))
```

## Testing Requirements

### Integration Tests
- Create service via Platform → Verify cluster exists in Native
- Create API via Platform → Verify routes/listeners in Native
- Import OpenAPI → Verify all resources created correctly

### Parity Tests
- Every Native cluster feature accessible via Platform service
- Every Native route feature accessible via Platform API
- Filter configurations work identically

### No Breaking Changes
- All existing Native API tests continue to pass
- Existing OpenAPI import tests work with redirect