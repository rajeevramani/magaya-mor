# Spec Requirements Document

> Spec: Platform API Enhancement & Unification
> Created: 2025-10-02

## Overview

Enhance and unify the Platform API to provide parity with the existing Native APIs through a consistent, simplified, and abstracted interface for API developers. The Native APIs already exist and work excellently - they will remain unchanged except for potential namespace organization. This spec focuses exclusively on bringing the Platform API to feature parity while consolidating the current OpenAPI import functionality into the Platform API, ensuring unified resource views across both the existing Native and enhanced Platform interfaces.

## User Stories

### Platform Engineer - Existing Native API User

As a platform engineer, I already have full access to Envoy configuration through the existing Native APIs, which work well and meet all my needs for advanced traffic management and troubleshooting.

The existing Native APIs at `/api/v1/clusters`, `/api/v1/routes`, `/api/v1/listeners` provide everything I need. These APIs may be organized under `/api/v1/native/*` namespace for clarity, but functionality remains unchanged. I can continue using these APIs exactly as before, with full access to circuit breakers, outlier detection, load balancing policies, and filter chains.

Make one change to ensure the native APIs are better aligned with envoy definitions. `/api/v1/routes` will become `/api/v1/route-configs`

### API Developer - Enhanced Platform API User

As an API developer, I want the Platform API to provide complete feature parity with Native APIs but through simplified abstractions, so that I can accomplish everything without understanding Envoy internals.

The enhanced Platform API (`/api/v1/platform/*`) will provide a consistent abstraction layer over ALL Native API capabilities. I can import OpenAPI specifications with custom filter tags (`x-flowplane-ratelimit`, `x-flowplane-jwt-auth`), define APIs using developer-friendly terms, and have confidence that anything achievable through Native APIs is also possible through Platform APIs, just simplified.

### DevOps Team - Unified Operations

As a member of the DevOps team, I want consistent resource state regardless of which API was used to create or modify configurations, so that I can maintain system integrity and troubleshoot effectively.

Resources created through either API must be visible and queryable from both interfaces. The Platform API enhancements ensure that ALL Native API resources are represented in the abstracted view, and all Platform API resources are fully accessible through Native APIs with complete Envoy details exposed.

## Spec Scope

1. **Platform API Enhancement** - Bring Platform API to full feature parity with Native APIs through consistent abstractions at `/api/v1/platform/*`
2. **OpenAPI Import Integration** - Move OpenAPI import from `/api/v1/gateway/import` to `/api/v1/platform/import/openapi` as part of unified Platform API
3. **Custom Filter Tag Support** - Implement `x-flowplane-*` prefixed extensions in OpenAPI specs for all existing filters
4. **Abstraction Completeness** - Ensure every Native API capability has a corresponding Platform API abstraction
5. **Native API Namespace** - Optionally organize existing Native APIs under `/api/v1/native/*` for clarity (no functional changes)

## Out of Scope

- Modifying existing Native API functionality (they work well as-is)
- Creating new Native API endpoints
- Changes to existing Native API request/response formats
- Removing or deprecating any Native API features
- Creating new xDS resource types beyond existing support

## Expected Deliverable

1. Platform API at `/api/v1/platform/*` with complete feature parity to Native APIs through simplified abstractions
2. OpenAPI import successfully integrated into Platform API at `/api/v1/platform/import/openapi`
3. All existing Native API resources viewable through Platform API abstractions, and all Platform API resources accessible via Native APIs