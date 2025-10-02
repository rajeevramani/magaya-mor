//! Platform API service abstraction handlers
//!
//! These handlers provide a simplified service-oriented interface that
//! automatically transforms to Native API cluster configurations.

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::api::error::ApiError;
use crate::api::handlers::{
    create_cluster_handler, delete_cluster_handler, get_cluster_handler, list_clusters_handler,
    update_cluster_handler, CircuitBreakerThresholdsRequest, CircuitBreakersRequest,
    ClusterResponse, CreateClusterBody, EndpointRequest, HealthCheckRequest,
    OutlierDetectionRequest,
};
use crate::api::routes::ApiState;

/// Platform API service definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceDefinition {
    /// Service name
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    /// Service endpoints
    #[validate(length(min = 1))]
    pub endpoints: Vec<ServiceEndpoint>,

    /// Load balancing strategy
    #[serde(default = "default_load_balancing")]
    pub load_balancing: LoadBalancingStrategy,

    /// Health check configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<ServiceHealthCheck>,

    /// Circuit breaker configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<ServiceCircuitBreaker>,

    /// Outlier detection configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outlier_detection: Option<ServiceOutlierDetection>,

    /// Service metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Service endpoint definition
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceEndpoint {
    /// Endpoint host
    pub host: String,

    /// Endpoint port
    pub port: u16,

    /// Endpoint weight (1-100)
    #[validate(range(min = 1, max = 100))]
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// Endpoint metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastRequest,
    Random,
    RingHash,
    Maglev,
}

/// Service health check configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthCheck {
    /// Health check path
    pub path: String,

    /// Interval in seconds
    #[serde(default = "default_health_interval")]
    pub interval: u32,

    /// Timeout in seconds
    #[serde(default = "default_health_timeout")]
    pub timeout: u32,

    /// Healthy threshold
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,

    /// Unhealthy threshold
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
}

/// Service circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCircuitBreaker {
    /// Maximum number of requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests: Option<u32>,

    /// Maximum pending requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_pending_requests: Option<u32>,

    /// Maximum connections
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,

    /// Maximum retries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,

    /// Consecutive errors threshold
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consecutive_errors: Option<u32>,

    /// Interval in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,
}

/// Service outlier detection configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOutlierDetection {
    /// Consecutive 5xx errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consecutive_5xx: Option<u32>,

    /// Interval in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,

    /// Base ejection time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ejection_time_ms: Option<u64>,

    /// Maximum ejection percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_ejection_percent: Option<u32>,

    /// Minimum hosts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_healthy_percent: Option<u32>,
}

/// Service response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceResponse {
    /// Service name
    pub name: String,

    /// Underlying cluster ID
    pub cluster_id: String,

    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,

    /// Load balancing strategy
    pub load_balancing: LoadBalancingStrategy,

    /// Health check configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<ServiceHealthCheck>,

    /// Circuit breaker configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<ServiceCircuitBreaker>,

    /// Outlier detection configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outlier_detection: Option<ServiceOutlierDetection>,

    /// Service metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Query parameters for listing services
#[derive(Debug, Deserialize)]
pub struct ListServicesQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// Default values
fn default_load_balancing() -> LoadBalancingStrategy {
    LoadBalancingStrategy::RoundRobin
}

fn default_weight() -> u32 {
    100
}

fn default_health_interval() -> u32 {
    10
}

fn default_health_timeout() -> u32 {
    3
}

fn default_healthy_threshold() -> u32 {
    2
}

fn default_unhealthy_threshold() -> u32 {
    2
}

// === Handler Functions ===

/// Create a new service
#[utoipa::path(
    post,
    path = "/api/v1/platform/services",
    request_body = ServiceDefinition,
    responses(
        (status = 201, description = "Service created", body = ServiceResponse),
        (status = 400, description = "Validation error"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-services"
)]
pub async fn create_service_handler(
    state: State<ApiState>,
    Json(service): Json<ServiceDefinition>,
) -> Result<(StatusCode, Json<ServiceResponse>), ApiError> {
    service.validate().map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    // Transform service to cluster definition
    let cluster_body = service_to_cluster(&service);

    // Create cluster using Native API
    let (status, Json(cluster_response)) =
        create_cluster_handler(state, Json(cluster_body)).await?;

    // Transform cluster response back to service response
    let service_response = cluster_to_service_response(cluster_response, service);

    Ok((status, Json(service_response)))
}

/// List all services
#[utoipa::path(
    get,
    path = "/api/v1/platform/services",
    params(
        ("limit" = Option<i32>, Query, description = "Maximum number of services to return"),
        ("offset" = Option<i32>, Query, description = "Offset for paginated results"),
    ),
    responses(
        (status = 200, description = "List of services", body = [ServiceResponse]),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-services"
)]
pub async fn list_services_handler(
    state: State<ApiState>,
    Query(params): Query<ListServicesQuery>,
) -> Result<Json<Vec<ServiceResponse>>, ApiError> {
    // Get clusters from Native API
    let query =
        crate::api::handlers::ListClustersQuery { limit: params.limit, offset: params.offset };

    let Json(clusters) = list_clusters_handler(state, Query(query)).await?;

    // Transform clusters to services
    let services: Vec<ServiceResponse> =
        clusters.into_iter().map(cluster_response_to_service).collect();

    Ok(Json(services))
}

/// Get service by name
#[utoipa::path(
    get,
    path = "/api/v1/platform/services/{name}",
    params(("name" = String, Path, description = "Name of the service")),
    responses(
        (status = 200, description = "Service details", body = ServiceResponse),
        (status = 404, description = "Service not found"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-services"
)]
pub async fn get_service_handler(
    state: State<ApiState>,
    Path(name): Path<String>,
) -> Result<Json<ServiceResponse>, ApiError> {
    // Get cluster from Native API
    let Json(cluster) = get_cluster_handler(state, Path(name)).await?;

    // Transform to service
    let service = cluster_response_to_service(cluster);

    Ok(Json(service))
}

/// Update service
#[utoipa::path(
    put,
    path = "/api/v1/platform/services/{name}",
    params(("name" = String, Path, description = "Name of the service")),
    request_body = ServiceDefinition,
    responses(
        (status = 200, description = "Service updated", body = ServiceResponse),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Service not found"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-services"
)]
pub async fn update_service_handler(
    state: State<ApiState>,
    Path(name): Path<String>,
    Json(service): Json<ServiceDefinition>,
) -> Result<Json<ServiceResponse>, ApiError> {
    service.validate().map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    if service.name != name {
        return Err(ApiError::BadRequest(format!(
            "Service name '{}' does not match path '{}'",
            service.name, name
        )));
    }

    // Transform service to cluster definition
    let cluster_body = service_to_cluster(&service);

    // Update cluster using Native API
    let Json(cluster_response) =
        update_cluster_handler(state, Path(name), Json(cluster_body)).await?;

    // Transform cluster response back to service response
    let service_response = cluster_to_service_response(cluster_response, service);

    Ok(Json(service_response))
}

/// Delete service
#[utoipa::path(
    delete,
    path = "/api/v1/platform/services/{name}",
    params(("name" = String, Path, description = "Name of the service")),
    responses(
        (status = 204, description = "Service deleted"),
        (status = 404, description = "Service not found"),
        (status = 503, description = "Service unavailable"),
    ),
    tag = "platform-services"
)]
pub async fn delete_service_handler(
    state: State<ApiState>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Delete cluster using Native API
    delete_cluster_handler(state, Path(name)).await
}

// === Transformation Functions ===

/// Transform service definition to cluster body
fn service_to_cluster(service: &ServiceDefinition) -> CreateClusterBody {
    let endpoints: Vec<EndpointRequest> = service
        .endpoints
        .iter()
        .map(|ep| EndpointRequest { host: ep.host.clone(), port: ep.port })
        .collect();

    let health_checks: Vec<HealthCheckRequest> = service
        .health_check
        .as_ref()
        .map(|hc| {
            vec![HealthCheckRequest {
                r#type: "http".to_string(),
                path: Some(hc.path.clone()),
                host: None,
                method: None,
                interval_seconds: Some(hc.interval as u64),
                timeout_seconds: Some(hc.timeout as u64),
                healthy_threshold: Some(hc.healthy_threshold),
                unhealthy_threshold: Some(hc.unhealthy_threshold),
                expected_statuses: None,
            }]
        })
        .unwrap_or_default();

    let circuit_breakers = service.circuit_breaker.as_ref().map(|cb| {
        let thresholds = CircuitBreakerThresholdsRequest {
            max_connections: cb.max_connections,
            max_pending_requests: cb.max_pending_requests,
            max_requests: cb.max_requests,
            max_retries: cb.max_retries,
        };

        CircuitBreakersRequest { default: Some(thresholds), high: None }
    });

    let outlier_detection = service.outlier_detection.as_ref().map(|od| OutlierDetectionRequest {
        consecutive_5xx: od.consecutive_5xx,
        interval_seconds: od.interval_ms.map(|ms| ms / 1000),
        base_ejection_time_seconds: od.base_ejection_time_ms.map(|ms| ms / 1000),
        max_ejection_percent: od.max_ejection_percent,
    });

    CreateClusterBody {
        name: service.name.clone(),
        endpoints,
        service_name: Some(service.name.clone()),
        connect_timeout_seconds: None,
        use_tls: None,
        tls_server_name: None,
        dns_lookup_family: None,
        lb_policy: Some(load_balancing_to_string(&service.load_balancing)),
        health_checks,
        circuit_breakers,
        outlier_detection,
    }
}

/// Convert load balancing strategy to string
fn load_balancing_to_string(strategy: &LoadBalancingStrategy) -> String {
    match strategy {
        LoadBalancingStrategy::RoundRobin => "ROUND_ROBIN".to_string(),
        LoadBalancingStrategy::LeastRequest => "LEAST_REQUEST".to_string(),
        LoadBalancingStrategy::Random => "RANDOM".to_string(),
        LoadBalancingStrategy::RingHash => "RING_HASH".to_string(),
        LoadBalancingStrategy::Maglev => "MAGLEV".to_string(),
    }
}

/// Convert string to load balancing strategy
fn string_to_load_balancing(s: &str) -> LoadBalancingStrategy {
    match s {
        "LEAST_REQUEST" => LoadBalancingStrategy::LeastRequest,
        "RANDOM" => LoadBalancingStrategy::Random,
        "RING_HASH" => LoadBalancingStrategy::RingHash,
        "MAGLEV" => LoadBalancingStrategy::Maglev,
        _ => LoadBalancingStrategy::RoundRobin,
    }
}

/// Transform cluster response to service response
fn cluster_to_service_response(
    cluster: ClusterResponse,
    service: ServiceDefinition,
) -> ServiceResponse {
    ServiceResponse {
        name: cluster.name.clone(),
        cluster_id: cluster.name, // Using name as ID for simplicity
        endpoints: service.endpoints,
        load_balancing: service.load_balancing,
        health_check: service.health_check,
        circuit_breaker: service.circuit_breaker,
        outlier_detection: service.outlier_detection,
        metadata: service.metadata,
    }
}

/// Transform cluster response to service (for list/get operations)
fn cluster_response_to_service(cluster: ClusterResponse) -> ServiceResponse {
    // Extract endpoints from cluster config
    let endpoints: Vec<ServiceEndpoint> = cluster
        .config
        .endpoints
        .iter()
        .filter_map(|ep| {
            ep.to_host_port().map(|(host, port)| ServiceEndpoint {
                host,
                port: port as u16,
                weight: 100, // Default weight
                metadata: None,
            })
        })
        .collect();

    let load_balancing = cluster
        .config
        .lb_policy
        .as_ref()
        .map(|p| string_to_load_balancing(p))
        .unwrap_or(LoadBalancingStrategy::RoundRobin);

    let health_check = if !cluster.config.health_checks.is_empty() {
        cluster.config.health_checks.first().and_then(|hc| match hc {
            crate::xds::HealthCheckSpec::Http {
                path,
                interval_seconds,
                timeout_seconds,
                unhealthy_threshold,
                healthy_threshold,
                ..
            } => Some(ServiceHealthCheck {
                path: path.clone(),
                interval: interval_seconds.unwrap_or(10) as u32,
                timeout: timeout_seconds.unwrap_or(3) as u32,
                healthy_threshold: healthy_threshold.unwrap_or(2),
                unhealthy_threshold: unhealthy_threshold.unwrap_or(2),
            }),
            _ => None,
        })
    } else {
        None
    };

    let circuit_breaker = cluster.config.circuit_breakers.as_ref().and_then(|cb| {
        cb.default.as_ref().map(|t| ServiceCircuitBreaker {
            max_requests: t.max_requests,
            max_pending_requests: t.max_pending_requests,
            max_connections: t.max_connections,
            max_retries: t.max_retries,
            consecutive_errors: None,
            interval_ms: None,
        })
    });

    let outlier_detection =
        cluster.config.outlier_detection.as_ref().map(|od| ServiceOutlierDetection {
            consecutive_5xx: od.consecutive_5xx,
            interval_ms: od.interval_seconds.map(|s| s * 1000),
            base_ejection_time_ms: od.base_ejection_time_seconds.map(|s| s * 1000),
            max_ejection_percent: od.max_ejection_percent,
            min_healthy_percent: None,
        });

    ServiceResponse {
        name: cluster.name.clone(),
        cluster_id: cluster.name,
        endpoints,
        load_balancing,
        health_check,
        circuit_breaker,
        outlier_detection,
        metadata: None, // Metadata not preserved in cluster spec
    }
}
