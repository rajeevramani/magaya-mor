# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-10-02-two-path-api-architecture/spec.md

## Technical Requirements

### Native API (Existing - No Changes Required)
- **Current Endpoints**: `/api/v1/clusters`, `/api/v1/routes`, `/api/v1/listeners` - WORKING WELL
- **Optional Namespace**: May organize under `/api/v1/native/*` for clarity (simple route aliasing)
- **Functionality**: NO CHANGES - existing implementation is complete and functional
- **Code Location**: Current handlers in `/src/api/*` remain unchanged

### Platform API Enhancement (Primary Focus)
- **New Namespace**: `/api/v1/platform/*` for all abstracted APIs
- **Feature Parity Goal**: Every Native API capability must have Platform API equivalent
- **OpenAPI Integration**: Move `/api/v1/gateway/import` to `/api/v1/platform/import/openapi`
- **Abstraction Mapping**:
  - APIs → Routes + Virtual Hosts
  - Endpoints → Clusters + Health Checks
  - Services → Listeners + Filter Chains
  - Policies → Filters + Configurations

### Implementation Strategy
- **Reuse Existing Code**: Platform API calls existing Native API handlers internally
- **Transformation Layer**: Platform abstractions → Native API calls → xDS resources
- **No Duplication**: Platform API is a facade over Native API, not a parallel implementation
- **Shared Storage**: Both APIs read/write same database tables

### Platform API Abstractions Required

#### Service Abstraction (Maps to Clusters)
```rust
// Platform API simplified view
pub struct Service {
    pub name: String,
    pub endpoints: Vec<String>, // ["host:port"]
    pub health_check: Option<String>, // "/health"
    pub timeout_seconds: Option<u32>,
}

// Transforms to existing Native API CreateClusterRequest
impl From<Service> for CreateClusterRequest {
    // Use existing cluster creation logic
}
```

#### API Definition Abstraction (Maps to Routes + Listeners)
```rust
pub struct ApiDefinition {
    pub name: String,
    pub domain: String,
    pub base_path: String,
    pub routes: Vec<SimpleRoute>,
    pub policies: HashMap<String, Value>, // Filters
}

// Transforms to multiple Native API calls
impl ApiDefinition {
    pub async fn apply(&self, native_api: &NativeApi) {
        // 1. Create/update listener via existing API
        // 2. Create routes via existing API
        // 3. Apply filters via existing API
    }
}
```

### OpenAPI Import Enhancement
- **Current Location**: `/api/v1/gateway/import`
- **New Location**: `/api/v1/platform/import/openapi`
- **Enhancement**: Add `x-flowplane-*` tag processing
- **Implementation**: Extend existing import logic, don't rewrite

### Filter Tag Processing
```rust
// Process OpenAPI extensions
pub fn process_flowplane_tags(spec: &OpenApiSpec) -> FilterConfig {
    // Extract x-flowplane-ratelimit → LocalRateLimit filter
    // Extract x-flowplane-jwt-auth → JwtAuthentication filter
    // Extract x-flowplane-cors → Cors filter
    // Use existing filter implementations from /src/xds/filters/
}
```

### Resource Visibility
- **Native → Platform**: Show full resources with simplified view
- **Platform → Native**: Platform resources are just Native resources (no conversion needed)
- **Query Translation**: Platform API transforms Native API responses for display

### Code Organization
```rust
// New Platform API module structure
src/platform_api/
├── mod.rs              // Platform API router
├── services.rs         // Service abstraction (uses Native cluster API)
├── apis.rs            // API definition abstraction (uses Native route/listener APIs)
├── openapi_import.rs  // Relocated and enhanced OpenAPI import
└── transformers.rs    // Abstraction ↔ Native transformations
```

### Migration Steps

#### Phase 1: Namespace Organization (Optional)
- Add route aliases: `/api/v1/native/clusters` → `/api/v1/clusters`
- No code changes, just routing

#### Phase 2: Platform API Implementation
- Create `/api/v1/platform/*` endpoints
- Each endpoint internally calls existing Native APIs
- Focus on abstraction and simplification

#### Phase 3: OpenAPI Relocation
- Move import handler to Platform API module
- Add filter tag processing
- Deprecate old endpoint with redirect

### Testing Strategy
- **Integration Tests**: Platform API → Native API → Database
- **Parity Tests**: Verify every Native feature has Platform equivalent
- **Cross-Query Tests**: Create via Platform, query via Native
- **No Breaking Changes**: Existing Native API tests must still pass

### Performance Considerations
- **Minimal Overhead**: Platform API is thin abstraction layer
- **Reuse Optimizations**: Leverage existing Native API caching/batching
- **Lazy Transformation**: Only transform data when requested

### Success Criteria
1. All existing Native API functionality remains unchanged
2. Platform API provides simplified access to all Native capabilities
3. OpenAPI import integrated into Platform API with filter tags
4. Resources fully visible across both APIs
5. Zero breaking changes to existing APIs