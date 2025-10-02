# Technology Stack

## Core Technologies

### Language & Runtime
- **Rust 1.89+** (edition 2021)
  - Memory safety and performance for control plane operations
  - Strong type system for Envoy protobuf handling
  - Async runtime via Tokio

### HTTP Server
- **Axum 0.8.4** - Modern async web framework
- **axum-extra 0.10.1** - Additional middleware (typed headers, cookies)
- **Tower 0.5.1** - Service composition and middleware
- **tower-http 0.6** - HTTP-specific middleware (CORS, tracing)
- **tower_governor 0.8.0** - Rate limiting middleware

### gRPC & Protobuf (xDS Server)
- **tonic 0.14.0** - gRPC server with TLS support (`tls-ring` feature)
- **tonic-reflection 0.14.0** - gRPC reflection for debugging
- **prost 0.14.0** - Protocol buffer code generation
- **prost-types 0.14.0** - Well-known protobuf types
- **envoy-types 0.7.0** - Official Envoy protobuf definitions

### Database & Persistence
- **SQLx 0.8.6** - Async SQL toolkit
  - Features: `runtime-tokio-rustls`, `postgres`, `sqlite`, `json`, `migrate`, `chrono`, `uuid`
  - Supports both SQLite (default) and PostgreSQL
- **Migration Strategy**: SQL migrations in `migrations/` directory

### Security & Authentication
- **jsonwebtoken 9.3.1** - JWT token creation and validation
- **argon2 0.5** - Password hashing (Argon2id algorithm)
- **bcrypt 0.17** - Alternative password hashing
- **rustls 0.23** - Modern TLS implementation (ring crypto backend)
- **tokio-rustls 0.26** - Async TLS for Tokio
- **ring 0.17.14** - Cryptographic primitives

### Observability
- **tracing 0.1** - Structured logging and distributed tracing
- **tracing-subscriber 0.3.20** - Log collection and formatting (env-filter, JSON output)
- **tracing-appender 0.2** - Log rotation and file appending
- **metrics 0.23** - Metrics collection
- **metrics-exporter-prometheus 0.15** - Prometheus metrics endpoint

### Serialization & Configuration
- **serde 1.0.226** - Serialization framework
- **serde_json 1.0.145** - JSON serialization
- **serde_yaml 0.9** - YAML parsing (OpenAPI import)
- **config 0.15** - Configuration management
- **clap 4.5.48** - CLI argument parsing

### Validation & Error Handling
- **validator 0.20.0** - Struct validation with derive macros
- **anyhow 1.0** - Flexible error handling
- **thiserror 2.0** - Error type derivation

### API Documentation
- **utoipa 5.4.0** - OpenAPI schema generation
- **utoipa-swagger-ui 9.0.2** - Embedded Swagger UI
- **openapiv3 1.0** - OpenAPI 3.0 document parsing

### Utilities
- **uuid 1.0** - UUID generation (v4)
- **chrono 0.4.41** - Date/time handling
- **tokio-stream 0.1** - Async stream utilities
- **futures 0.3** - Async trait and utilities
- **async-trait 0.1** - Async trait support
- **regex 1.10** - Regular expressions
- **url 2.5** - URL parsing
- **bytes 1.6** - Byte buffer utilities
- **once_cell 1.19** - Lazy static initialization
- **lazy_static 1.4** - Static variable initialization

## Development Dependencies

### Testing
- **tokio-test 0.4** - Tokio async test utilities
- **axum-test 16.4.0** - Axum integration testing
- **tracing-test 0.2.5** - Tracing assertions for tests
- **proptest 1.5** - Property-based testing
- **tempfile 3.14** - Temporary file/directory management

### TLS Testing
- **rcgen 0.13** - Certificate generation for tests
- **rustls-pemfile 2.1** - PEM file parsing
- **hyper-rustls 0.27** - Hyper TLS integration

### Utilities
- **reserve-port 2.3** - Test port allocation
- **which 6.0** - PATH executable lookup

## Database Schema

### Core Tables
- `clusters` - Envoy upstream cluster configurations (JSON storage)
- `routes` - Route configurations with path matching
- `listeners` - Listener configurations (address, port, filters)
- `api_definitions` - Platform API team-scoped API definitions
- `api_routes` - Platform API route configurations
- `auth_tokens` - JWT authentication tokens with scopes
- `audit_log` - Configuration change audit trail
- `configuration_versions` - Version tracking for xDS updates

### Schema Versioning
- SQLx migrations with timestamp prefixes
- Foreign key constraints between routes→clusters, api_routes→api_definitions

## Infrastructure

### Deployment
- Containerized (Dockerfile provided)
- Environment variable configuration
- Supports SQLite (embedded) or PostgreSQL (production)

### Configuration
- Environment variables for all settings
- Sensible defaults for development
- TLS certificate paths configurable

### Observability Stack
- Structured JSON logging
- Prometheus metrics on `/metrics`
- Health check endpoint
- Admin API on configurable port
- xDS server on separate port (default 18003)

## Build & Development

### Build System
- **Cargo** - Rust package manager and build tool
- **cargo fmt** - Code formatting (`.rustfmt.toml`)
- **cargo clippy** - Linting (`.clippy.toml`)
- **cargo test** - Test runner

### CI/CD
- GitHub Actions workflows (`.github/`)
- Docker image builds
- Automated testing

## Architecture Patterns

### Code Organization
```
src/
├── api/              # REST API handlers and routes
├── auth/             # Authentication and authorization
├── cli/              # CLI commands
├── config/           # Configuration management
├── errors/           # Error types and handling
├── observability/    # Logging, metrics, tracing
├── openapi/          # OpenAPI import logic
├── platform_api/     # Platform API abstraction
├── storage/          # Database repositories
├── utils/            # Shared utilities
├── validation/       # Request validation layers
└── xds/              # Envoy xDS server and protobuf conversion
```

### Design Patterns
- **Repository Pattern** - Database access abstraction
- **Validation Pipeline** - Layered validation (request → business rules → Envoy protobuf)
- **State Management** - Shared `XdsState` with Arc<RwLock<>>
- **Error Propagation** - Result types with custom error enums
- **Async/Await** - Throughout with Tokio runtime

## External Dependencies

### Runtime Requirements
- Envoy proxy (data plane) - any version with xDS v3 support
- SQLite 3.x OR PostgreSQL 12+

### Optional
- Prometheus for metrics scraping
- Jaeger/Zipkin for distributed tracing (if enabled)