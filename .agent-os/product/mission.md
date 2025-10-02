# Product Mission

## Pitch

Flowplane is an Envoy xDS control plane that helps platform engineering teams expose APIs securely across any cloud or environment by providing a RESTful management interface, multi-tenant API definitions, and automatic Envoy bootstrap configuration generation.

## Users

### Primary Customers

**Platform Engineering Teams** within organizations who need to provide API exposure infrastructure as a service to their application development teams.

### End Users

**Application Development Teams** who build internal or external services and need to expose APIs securely without managing complex Envoy configurations directly.

**DevOps Engineers** who operate API gateways and need centralized control over routing, security policies, and observability.

## The Problem

Organizations adopting Envoy proxy face steep learning curves with complex protobuf configurations, lack of multi-tenancy, and no standardized way to manage API lifecycles across teams. Existing solutions either:
- Lock users into specific cloud providers
- Require deep Envoy expertise to operate
- Don't support team-based isolation and self-service API management
- Can't unify OpenAPI-based workflows with low-level Envoy control

Flowplane solves this by abstracting Envoy's complexity while preserving full capability, enabling platform teams to offer secure API exposure as an internal service.

## Differentiators

1. **Cloud-Agnostic Deployment**: Deploy anywhere—on-prem, AWS, GCP, Azure, or hybrid environments
2. **Three-Tier API Model**:
   - Platform API for simplified team workflows
   - OpenAPI import for existing specs
   - Native Envoy API for advanced use cases
3. **Multi-Tenancy Built-In**: Team-scoped API definitions with domain isolation and optional dedicated listeners
4. **Full Envoy Feature Surface**: Circuit breakers, health checks, TLS/mTLS, rate limiting, JWT auth, CORS—all via REST
5. **Bootstrap Generation**: Automatic Envoy data plane configuration with ADS and metadata-based scoping
6. **Security-First**: Authentication (JWT tokens with scopes), TLS for both admin API and xDS channel, audit logging

## Key Features

### Already Implemented (Phase 0)

- **Envoy xDS Server**: Implements CDS, RDS, LDS with SQLite/PostgreSQL persistence
- **REST API Management**: CRUD operations for clusters, routes, and listeners with OpenAPI documentation
- **Authentication & Authorization**: JWT bearer tokens with scoped permissions, admin token bootstrapping
- **TLS/mTLS Support**: Secure xDS channel and admin API with certificate validation
- **OpenAPI Import**: Generate complete gateway stacks (clusters, routes, listeners) from OpenAPI 3.0 specs
- **Platform API (Beta)**: Team-scoped API definitions with route management and bootstrap generation
- **HTTP Filters**: JWT authentication, local rate limiting, CORS, distributed tracing
- **Observability**: Structured logging, Prometheus metrics, health checks
- **Audit Trail**: All configuration changes logged with actor and timestamp

### In Development (Phase 1)

- **E2E Envoy Integration Tests**: Automated validation of control plane → data plane workflows

### Planned (Phases 2-5)

- **Data Model Unification**: Resolve storage inconsistencies between Platform API and Native API paths
- **Enhanced Multi-Tenancy**: RBAC, tenant quotas, resource isolation
- **MCP Protocol Support**: Model Context Protocol for AI agent integration
- **A2A Protocols**: Application-to-Application authentication patterns
- **Advanced Filter Catalog**: External authorization, WASM filters, custom extensions

## Success Criteria

1. **Developer Experience**: Application teams can expose a new API in <5 minutes via Platform API
2. **Operations**: Platform teams can deploy Flowplane in any environment without vendor lock-in
3. **Security**: All API traffic secured by default with TLS, JWT auth, and audit logging
4. **Reliability**: 99.9% uptime for control plane with zero-downtime xDS updates
5. **Adoption**: Used by 10+ teams within first 6 months of internal deployment