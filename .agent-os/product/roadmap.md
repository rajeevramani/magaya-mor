# Product Roadmap

## Phase 0: Already Completed ✅

The following features have been implemented and are production-ready:

### Core Infrastructure
- [x] **Envoy xDS Server** - Full implementation of CDS, RDS, LDS protocols with ADS support
- [x] **SQLite/PostgreSQL Persistence** - Dual database support with migration system
- [x] **Configuration Versioning** - Version tracking for xDS resource updates
- [x] **Async Runtime** - Tokio-based async architecture for high concurrency

### REST API Management (Native/Advanced API Foundation)
- [x] **Cluster Management API** - CRUD operations for Envoy upstream clusters
  - Endpoints, load balancing policies, health checks
  - Circuit breakers, outlier detection
  - TLS configuration with SNI support
- [x] **Route Management API** - CRUD for route configurations
  - Path matching (exact, prefix, regex, template)
  - Header and query parameter matching
  - Weighted clusters, redirects, rewrites
- [x] **Listener Management API** - CRUD for listeners
  - HTTP connection manager configuration
  - TCP proxy support
  - Filter chain management
- [x] **OpenAPI Documentation** - Swagger UI at `/swagger-ui` with complete API specs

### Security & Authentication
- [x] **JWT Authentication** - Bearer token authentication for all API endpoints
- [x] **Token Management API** - Create, list, revoke tokens with scoped permissions
- [x] **Admin Bootstrap Token** - One-time admin token generation on first startup
- [x] **Audit Logging** - All configuration changes logged with actor and timestamp
- [x] **TLS/mTLS for xDS** - Secure control plane ↔ data plane communication
  - Client certificate validation
  - Configurable CA bundles
- [x] **API TLS Termination** - HTTPS for admin/management API

### HTTP Filters
- [x] **JWT Authentication Filter** - Envoy JWT validation with JWKS support
- [x] **Local Rate Limiting** - Per-listener and per-route rate limits
- [x] **CORS Filter** - Cross-origin resource sharing configuration
- [x] **Distributed Tracing** - Integration with Jaeger/Zipkin

### Platform API Foundation (Beta)
- [x] **API Definitions** - Team-scoped API management
  - Team and domain uniqueness constraints
  - Multi-route support per definition
- [x] **Route Management** - Append routes to existing definitions
- [x] **Listener Isolation** - Optional dedicated listeners per API definition
- [x] **Bootstrap Generation** - Automatic Envoy ADS bootstrap YAML/JSON
  - Node metadata for team-based scoping
  - Configurable resource filtering (all/team/allowlist)
- [x] **OpenAPI Import (Legacy - TO BE MOVED)** - Currently at `/api/v1/gateway/import`
  - Automatic cluster derivation from `servers` array
  - Route generation from `paths`
  - **ACTION REQUIRED: Move this functionality to `/api/v1/platform/import/openapi`**
  - **DO NOT DUPLICATE - RELOCATE EXISTING CODE**

### Observability
- [x] **Structured Logging** - JSON and text formats with configurable levels
- [x] **Prometheus Metrics** - `/metrics` endpoint with xDS and API metrics
- [x] **Health Checks** - Liveness and readiness endpoints
- [x] **Distributed Tracing** - OpenTelemetry integration

### Developer Experience
- [x] **CLI Tool** - `flowplane-cli` for common operations
- [x] **Docker Support** - Production-ready Dockerfile
- [x] **Environment Configuration** - Full env-based configuration
- [x] **Comprehensive Documentation** - Cookbooks for clusters, routes, listeners, filters

---

## Phase 1: Two-Path API Architecture 🎯 **HIGHEST PRIORITY**

**Goal**: Establish clear separation between Native (Advanced) API for platform engineers and unified Platform API for API developers, with consistent resource views across both paths.

### API Architecture Restructuring (Critical)
- [ ] **Native/Advanced API** (`/api/v1/native/*`)
  - Direct Envoy configuration access for platform engineers
  - Full control over listeners, routes, clusters, filters
  - Uses existing Envoy terminology and concepts
  - Team isolation enforced for multi-tenancy

- [ ] **Unified Platform API** (`/api/v1/platform/*`)
  - Developer-friendly abstraction layer
  - **MOVE existing OpenAPI import from `/api/v1/gateway/import` to `/api/v1/platform/import/openapi`**
  - Merge Platform API capabilities with OpenAPI import (single codebase, not duplicate)
  - Resource terminology mapping:
    - APIs → Routes/Virtual Hosts
    - Endpoints → Clusters/Upstreams
    - Services → Listeners
  - OpenAPI import endpoint: `/api/v1/platform/import/openapi` (relocated from gateway)
  - API definition management: `/api/v1/platform/apis/*`

### OpenAPI Extension Support
- [ ] **Custom Filter Tags**
  - Support `x-flowplane-` prefixed extensions in OpenAPI specs
  - Pre-defined filters: `x-flowplane-ratelimit`, `x-flowplane-jwt-auth`, `x-flowplane-cors`
  - Filter configuration embedded in OpenAPI extensions
  - Validation and error reporting for unsupported filters

### Filter Injection Hierarchy
- [ ] **Multi-Level Filter Application**
  - Route-level filters (highest precedence)
  - Route-config-level filters
  - Listener-level filters
  - Filters applied as specified, let Envoy handle composition
  - Store filter configs embedded in resource JSON

### Data Transformation Layer
- [ ] **Direct Transformation Approach**
  - Platform API → Envoy entities using existing `/src/xds/*` code
  - Leverage existing `PathMatch`, `RouteConfig` types as common models
  - Path pattern automatic conversion:
    - `/api/v1/resource` → prefix match
    - `/api/v1/resource/{id}` → template match
    - `/api/v1/resource/*/details` → regex match

---

## Phase 2: Data Model Unification & Consistency 🔧 **CRITICAL**

**Goal**: Ensure unified resource views and prevent configuration loss across API paths. This remains high priority as specified in the unified cluster model spec.

### Unified Cluster Model Implementation
- [ ] **ClusterConfigV2** - As specified in `2025-10-02-unified-cluster-model` spec
  - Versioned serialization for all cluster configs
  - Field preservation across API updates
  - Automatic v1→v2 migration
  - Rollback safety with audit trail

### Resource Query Consistency
- [ ] **Unified View Implementation**
  - All resources (listeners, routes, clusters, filters) queryable from both APIs
  - Consistent state regardless of creation method
  - Platform API resources visible in Native API queries
  - Native API resources manageable via Platform API where appropriate

### Storage Normalization
- [ ] **Route Storage Unification**
  - Ensure all routes stored in `routes` table
  - Platform API routes materialized properly
  - Foreign key constraints maintained

### Cross-Path Integration Tests
- [ ] **Comprehensive Test Suite**
  - Create via Native → Update via Platform → Verify consistency
  - Import OpenAPI → Query Native API → Verify visibility
  - Update cluster → Verify route resolution across APIs
  - Filter preservation across API boundaries

---

## Phase 3: Multi-Tenancy & Team Isolation

**Goal**: Strengthen team isolation across both API paths with proper access control.

### Team Isolation Enhancement
- [ ] **Enforced Team Boundaries**
  - Team isolation applied to both Native and Platform APIs
  - Platform engineers manage resources within team scope
  - No bypass for team boundaries (security requirement)

### RBAC Implementation
- [ ] **Role-Based Access Control**
  - Roles: admin, platform-engineer, api-developer
  - Permissions scoped to teams and API paths
  - Token scopes aligned with roles

### Resource Quotas
- [ ] **Per-Team Limits**
  - Quotas for clusters, routes, listeners, API definitions
  - Rate limiting per team token
  - Storage quota enforcement

---

## Phase 4: Enhanced Platform Features

**Goal**: Expand Platform API capabilities and improve developer experience.

### Platform API Enhancements
- [ ] **Advanced Route Patterns**
  - Full OpenAPI path parameter support
  - Query parameter and header matching
  - Automatic path type detection

### Filter Catalog Expansion
- [ ] **Additional Pre-defined Filters**
  - External authorization
  - Request/response transformation
  - Custom headers manipulation
  - Circuit breaker configuration

### Developer Experience
- [ ] **API Builder UI**
  - Visual API definition creator
  - Filter configuration wizard
  - Real-time validation

---

## Phase 5: Protocol Extensions

**Goal**: MCP and A2A protocol support for advanced integrations.

### MCP Protocol Support
- [ ] **Model Context Protocol Server**
  - Expose configuration as MCP resources
  - AI agent integration for config management
  - Natural language API definition creation

### Service Mesh Integration
- [ ] **SPIFFE/SPIRE Support**
  - mTLS with workload identity
  - Automatic service discovery
  - Zero-trust networking

---

## Phase 6: Enterprise Features

**Goal**: Production-grade scalability and operational excellence.

### High Availability
- [ ] **Control Plane HA**
  - Leader election for active-passive setup
  - Stateless API servers
  - Session resumption for xDS

### Performance Optimization
- [ ] **Delta xDS Protocol**
  - Incremental updates only
  - Resource caching
  - 10,000+ data plane support

### Operations
- [ ] **GitOps Integration**
  - Declarative config from Git
  - PR-based workflows
  - Terraform provider

---

## Success Metrics

### Developer Experience
- Time to first API exposure: <5 minutes via Platform API
- Time to production: <1 hour for new instance
- API consistency: 100% resource visibility across both paths

### Reliability
- Control plane uptime: 99.9%
- Configuration consistency: Zero data loss across API updates
- xDS update latency: <500ms p99

### Adoption
- Platform teams using Native API: 5+ within 3 months
- Developers using Platform API: 20+ within 3 months
- OpenAPI specs imported: 30+ within 6 months

---

## Notes

- **Phase 1 & 2 are highest priority** - Two-path architecture and data consistency must be completed first
- **CRITICAL: OpenAPI import is MOVED, not duplicated** - Relocate existing `/api/v1/gateway/import` to `/api/v1/platform/import/openapi`
- No backward compatibility concerns - no existing customers, can remove old endpoint immediately
- Filter configuration embedded in resource JSON for simplicity
- Direct transformation approach using existing `/src/xds/*` code
- Platform API and OpenAPI import share same underlying implementation - unified codebase