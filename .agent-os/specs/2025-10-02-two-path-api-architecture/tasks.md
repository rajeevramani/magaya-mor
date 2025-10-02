# Spec Tasks

These are the tasks to be completed for the spec detailed in @.agent-os/specs/2025-10-02-two-path-api-architecture/spec.md

> Created: 2025-10-02
> Status: Ready for Implementation

## Tasks

- [x] 1. Rename Native API routes endpoints to route-configs for Envoy alignment
  - [x] 1.1 Write tests for route-configs endpoint renaming
  - [x] 1.2 Update route handlers to use route-configs naming
  - [x] 1.3 Create route aliases for backward compatibility
  - [x] 1.4 Update OpenAPI documentation for route-configs
  - [x] 1.5 Update all route-related tests to use new naming
  - [x] 1.6 Verify all tests pass with new naming

- [x] 2. Implement Platform API service abstraction
  - [x] 2.1 Write tests for Platform API service endpoints
  - [x] 2.2 Create service data structures and transformers
  - [x] 2.3 Implement GET /api/v1/platform/services endpoint
  - [x] 2.4 Implement POST /api/v1/platform/services endpoint
  - [x] 2.5 Add service to cluster transformation logic
  - [x] 2.6 Implement PUT and DELETE service endpoints
  - [x] 2.7 Verify service CRUD operations work correctly
  - [x] 2.8 Verify all tests pass

- [x] 3. Implement Platform API for API definitions
  - [x] 3.1 Write tests for API definition endpoints
  - [x] 3.2 Create API definition data structures
  - [x] 3.3 Implement GET /api/v1/platform/apis endpoint
  - [x] 3.4 Implement POST /api/v1/platform/apis endpoint
  - [x] 3.5 Add API to route-config/listener transformation logic
  - [x] 3.6 Implement policy (filter) application
  - [x] 3.7 Verify all tests pass

- [x] 4. Relocate and enhance OpenAPI import functionality
  - [x] 4.1 Write tests for relocated OpenAPI import
  - [x] 4.2 Move import handler to /api/v1/platform/import/openapi
  - [x] 4.3 Implement x-flowplane-* tag processing
  - [x] 4.4 Add filter tag validation and error handling
  - [x] 4.5 Create redirect from old endpoint
  - [x] 4.6 Update OpenAPI import response format
  - [x] 4.7 Verify import works with filter tags
  - [x] 4.8 Verify all tests pass

- [x] 5. Ensure unified resource visibility across APIs
  - [x] 5.1 Write cross-API visibility tests
  - [x] 5.2 Implement Native to Platform view transformers
  - [x] 5.3 Implement Platform to Native query support
  - [x] 5.4 Add resource source tracking
  - [x] 5.5 Verify resources created via Platform visible in Native
  - [x] 5.6 Verify resources created via Native visible in Platform
  - [x] 5.7 Verify all integration tests pass