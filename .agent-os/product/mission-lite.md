# Flowplane Mission (Condensed)

**Product**: Envoy xDS Control Plane with RESTful Management API

**Purpose**: Enable platform teams to offer API exposure infrastructure as a service to application developers across any cloud environment.

**Users**: Platform engineers (operators), API developers (consumers)

**Problem Solved**: Simplifies Envoy adoption by abstracting complex protobuf configurations while preserving full Envoy capabilities. Provides multi-tenancy, OpenAPI import, and automatic bootstrap generation.

**Key Differentiators**:
- Cloud-agnostic (deploy anywhere)
- Three API tiers: Platform API (simplified), OpenAPI import, Native Envoy API (full control)
- Multi-tenant with team isolation
- Security-first (JWT auth, TLS/mTLS, audit logs)

**Current State**: Production-ready core with xDS, REST API, authentication, TLS, filters (JWT, rate limiting, CORS). Platform API in beta.

**Next Priority**: Data model unification to resolve storage inconsistencies between API paths.