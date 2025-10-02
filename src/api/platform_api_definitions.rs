//! Platform API definitions handlers
//!
//! These handlers provide a high-level API definition interface that
//! automatically creates and manages route configurations, listeners, and clusters.

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::api::error::ApiError;
use crate::api::handlers::{CreateClusterBody, EndpointRequest};
use crate::api::route_handlers::{
    PathMatchDefinition, RouteActionDefinition, RouteDefinition, RouteMatchDefinition,
    RouteRuleDefinition, VirtualHostDefinition,
};
use crate::api::routes::ApiState;
use std::collections::HashMap;

/// Platform API definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiDefinition {
    /// API name
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    /// API version
    #[validate(length(min = 1, max = 50))]
    pub version: String,

    /// Base path for all routes
    #[validate(length(min = 1, max = 255))]
    pub base_path: String,

    /// Upstream service configuration
    pub upstream: UpstreamConfig,

    /// API routes
    #[validate(length(min = 1))]
    pub routes: Vec<ApiRoute>,

    /// Global policies applied to all routes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<ApiPolicies>,

    /// API metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Upstream service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamConfig {
    /// Service name
    #[validate(length(min = 1, max = 100))]
    pub service: String,

    /// Service endpoints
    #[validate(length(min = 1))]
    pub endpoints: Vec<UpstreamEndpoint>,

    /// Enable TLS for upstream connections
    #[serde(default)]
    pub tls: bool,

    /// Load balancing strategy
    #[serde(default = "default_load_balancing")]
    pub load_balancing: String,
}

/// Upstream endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamEndpoint {
    /// Endpoint host
    pub host: String,

    /// Endpoint port
    pub port: u16,

    /// Optional weight for load balancing
    #[serde(default = "default_weight")]
    pub weight: u32,
}

/// API route definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiRoute {
    /// Route path (relative to base path)
    #[validate(length(min = 1, max = 255))]
    pub path: String,

    /// HTTP methods allowed for this route
    #[validate(length(min = 1))]
    pub methods: Vec<String>,

    /// Route description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Route-specific policies (override global policies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<ApiPolicies>,
}

/// API policies configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiPolicies {
    /// Rate limiting configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitPolicy>,

    /// Authentication configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<AuthenticationPolicy>,

    /// Authorization configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<AuthorizationPolicy>,

    /// CORS configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors: Option<CorsPolicy>,

    /// Circuit breaker configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerPolicy>,

    /// Retry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryPolicy>,

    /// Timeout configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<TimeoutPolicy>,
}

/// Rate limiting policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitPolicy {
    /// Number of requests allowed
    pub requests: u32,

    /// Time interval (e.g., "1m", "1h")
    pub interval: String,

    /// Key to use for rate limiting (e.g., "client_ip", "header:x-api-key")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_by: Option<String>,
}

/// Authentication policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationPolicy {
    /// Authentication type (e.g., "jwt", "oauth2", "api_key")
    #[serde(rename = "type")]
    pub auth_type: String,

    /// Whether authentication is required
    #[serde(default = "default_required")]
    pub required: bool,

    /// Additional configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// Authorization policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationPolicy {
    /// Required roles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    /// Required permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
}

/// CORS policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CorsPolicy {
    /// Allowed origins
    pub origins: Vec<String>,

    /// Allowed methods
    pub methods: Vec<String>,

    /// Allowed headers
    pub headers: Vec<String>,

    /// Whether to allow credentials
    #[serde(default)]
    pub allow_credentials: bool,

    /// Max age for preflight requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u32>,
}

/// Circuit breaker policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerPolicy {
    /// Maximum number of requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests: Option<u32>,

    /// Interval in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,

    /// Consecutive errors threshold
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consecutive_errors: Option<u32>,
}

/// Retry policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    /// Number of retry attempts
    pub attempts: u32,

    /// Backoff strategy ("fixed", "exponential")
    #[serde(default = "default_backoff")]
    pub backoff: String,

    /// Initial delay in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_delay_ms: Option<u64>,
}

/// Timeout policy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutPolicy {
    /// Request timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<u32>,

    /// Idle timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle: Option<u32>,
}

/// API definition response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiDefinitionResponse {
    /// Unique API ID
    pub id: String,

    /// API name
    pub name: String,

    /// API version
    pub version: String,

    /// Base path
    pub base_path: String,

    /// Upstream configuration
    pub upstream: UpstreamConfig,

    /// API routes
    pub routes: Vec<ApiRoute>,

    /// API policies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<ApiPolicies>,

    /// Associated route configuration ID
    pub route_config_id: String,

    /// Associated listener ID
    pub listener_id: String,

    /// Associated cluster ID
    pub cluster_id: String,

    /// API metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Creation timestamp
    pub created_at: String,

    /// Update timestamp
    pub updated_at: String,
}

/// Query parameters for listing API definitions
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListApisQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub name: Option<String>,
    pub version: Option<String>,
}

// Default values
fn default_load_balancing() -> String {
    "ROUND_ROBIN".to_string()
}

fn default_weight() -> u32 {
    100
}

fn default_required() -> bool {
    true
}

fn default_backoff() -> String {
    "exponential".to_string()
}

// === Handler Functions ===

// Helper function to transform API definition to cluster configuration
fn api_to_cluster(api: &ApiDefinition, cluster_name: &str) -> CreateClusterBody {
    let endpoints: Vec<EndpointRequest> = api
        .upstream
        .endpoints
        .iter()
        .map(|ep| EndpointRequest { host: ep.host.clone(), port: ep.port })
        .collect();

    let cluster = CreateClusterBody {
        name: cluster_name.to_string(),
        service_name: Some(api.upstream.service.clone()),
        endpoints,
        connect_timeout_seconds: api
            .policies
            .as_ref()
            .and_then(|p| p.timeout.as_ref())
            .and_then(|t| t.request.map(|r| r as u64)),
        use_tls: Some(api.upstream.tls),
        tls_server_name: None,
        dns_lookup_family: None,
        lb_policy: Some(api.upstream.load_balancing.clone()),
        health_checks: vec![],
        circuit_breakers: None,
        outlier_detection: None,
    };

    // Add circuit breaker if policy is defined
    if let Some(policies) = &api.policies {
        if policies.circuit_breaker.is_some() {
            // Circuit breaker configuration would be added here
            // This is simplified - actual implementation would map properly
        }
    }

    cluster
}

// Helper function to transform API routes to route configuration
fn api_to_route_config(api: &ApiDefinition, route_config_name: &str) -> RouteDefinition {
    let routes: Vec<RouteRuleDefinition> = api
        .routes
        .iter()
        .map(|route| {
            let full_path = format!("{}{}", api.base_path, route.path);

            RouteRuleDefinition {
                name: route.description.clone(),
                r#match: RouteMatchDefinition {
                    path: PathMatchDefinition::Prefix { value: full_path },
                    headers: vec![],
                    query_parameters: vec![],
                },
                action: RouteActionDefinition::Forward {
                    cluster: api.upstream.service.clone(),
                    timeout_seconds: api
                        .policies
                        .as_ref()
                        .and_then(|p| p.timeout.as_ref())
                        .and_then(|t| t.request.map(|r| r as u64)),
                    prefix_rewrite: None,
                    template_rewrite: None,
                },
                typed_per_filter_config: HashMap::new(),
            }
        })
        .collect();

    RouteDefinition {
        name: route_config_name.to_string(),
        virtual_hosts: vec![VirtualHostDefinition {
            name: api.name.clone(),
            domains: vec!["*".to_string()], // This should be configurable
            routes,
            typed_per_filter_config: HashMap::new(),
        }],
    }
}

/// Create a new API definition
#[utoipa::path(
    post,
    path = "/api/v1/platform/apis",
    request_body = ApiDefinition,
    responses(
        (status = 201, description = "API definition created", body = ApiDefinitionResponse),
        (status = 400, description = "Validation error"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-apis"
)]
pub async fn create_api_definition_handler(
    State(_state): State<ApiState>,
    Json(api): Json<ApiDefinition>,
) -> Result<(StatusCode, Json<ApiDefinitionResponse>), ApiError> {
    api.validate().map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    // Generate unique IDs
    let api_id = Uuid::new_v4().to_string();
    let cluster_id = format!("{}-cluster", api_id);
    let route_config_id = format!("{}-routes", api_id);
    let listener_id = format!("{}-listener", api_id);

    // Store API definition in database (if repository is available)
    // For now, we'll just create the resources in-memory

    // Transform and create cluster via Native API
    let _cluster_config = api_to_cluster(&api, &cluster_id);
    // In real implementation, we'd call the cluster handler here

    // Transform and create route configuration via Native API
    let _route_config = api_to_route_config(&api, &route_config_id);
    // In real implementation, we'd call the route handler here

    // Create listener if needed (simplified)
    // In real implementation, we'd check if listener exists and create if needed

    let response = ApiDefinitionResponse {
        id: api_id.clone(),
        name: api.name,
        version: api.version,
        base_path: api.base_path,
        upstream: api.upstream,
        routes: api.routes,
        policies: api.policies,
        route_config_id,
        listener_id,
        cluster_id,
        metadata: api.metadata,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// List all API definitions
#[utoipa::path(
    get,
    path = "/api/v1/platform/apis",
    params(
        ("limit" = Option<i32>, Query, description = "Maximum number of APIs to return"),
        ("offset" = Option<i32>, Query, description = "Offset for paginated results"),
        ("name" = Option<String>, Query, description = "Filter by API name"),
        ("version" = Option<String>, Query, description = "Filter by API version"),
    ),
    responses(
        (status = 200, description = "List of API definitions", body = [ApiDefinitionResponse]),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-apis"
)]
pub async fn list_api_definitions_handler(
    State(_state): State<ApiState>,
    Query(params): Query<ListApisQuery>,
) -> Result<Json<Vec<ApiDefinitionResponse>>, ApiError> {
    // Store API definitions in memory for now (would use database in production)
    // Return empty list for now - in production would query from repository

    // Apply filters if provided
    let mut results = vec![];

    // Filter by name if provided
    if let Some(name_filter) = params.name {
        results.retain(|api: &ApiDefinitionResponse| api.name.contains(&name_filter));
    }

    // Filter by version if provided
    if let Some(version_filter) = params.version {
        results.retain(|api: &ApiDefinitionResponse| api.version == version_filter);
    }

    // Apply pagination
    let offset = params.offset.unwrap_or(0) as usize;
    let limit = params.limit.unwrap_or(100) as usize;

    let paginated: Vec<ApiDefinitionResponse> =
        results.into_iter().skip(offset).take(limit).collect();

    Ok(Json(paginated))
}

/// Get API definition by ID
#[utoipa::path(
    get,
    path = "/api/v1/platform/apis/{id}",
    params(("id" = String, Path, description = "API definition ID")),
    responses(
        (status = 200, description = "API definition details", body = ApiDefinitionResponse),
        (status = 404, description = "API definition not found"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-apis"
)]
pub async fn get_api_definition_by_id_handler(
    State(_state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApiDefinitionResponse>, ApiError> {
    // In production, would fetch from database/repository
    // For now, return not found
    Err(ApiError::NotFound(format!("API definition with ID '{}' not found", id)))
}

/// Update API definition
#[utoipa::path(
    put,
    path = "/api/v1/platform/apis/{id}",
    params(("id" = String, Path, description = "API definition ID")),
    request_body = ApiDefinition,
    responses(
        (status = 200, description = "API definition updated", body = ApiDefinitionResponse),
        (status = 400, description = "Validation error"),
        (status = 404, description = "API definition not found"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-apis"
)]
pub async fn update_api_definition_handler(
    State(_state): State<ApiState>,
    Path(id): Path<String>,
    Json(api): Json<ApiDefinition>,
) -> Result<Json<ApiDefinitionResponse>, ApiError> {
    api.validate().map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    // In production:
    // 1. Fetch existing API definition
    // 2. Update clusters via Native API if upstream changed
    // 3. Update route configs via Native API if routes changed
    // 4. Update listeners if needed
    // 5. Store updated definition

    // For now, return not found
    Err(ApiError::NotFound(format!("API definition with ID '{}' not found", id)))
}

/// Delete API definition
#[utoipa::path(
    delete,
    path = "/api/v1/platform/apis/{id}",
    params(("id" = String, Path, description = "API definition ID")),
    responses(
        (status = 204, description = "API definition deleted"),
        (status = 404, description = "API definition not found"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-apis"
)]
pub async fn delete_api_definition_handler(
    State(_state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // In production:
    // 1. Fetch API definition
    // 2. Delete associated clusters via Native API
    // 3. Delete associated route configs via Native API
    // 4. Delete associated listeners if exclusively owned
    // 5. Delete definition from database

    // For now, return not found
    Err(ApiError::NotFound(format!("API definition with ID '{}' not found", id)))
}
