use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{error, info};
use utoipa::ToSchema;

use validator::Validate;

use envoy_types::pb::envoy::extensions::path::r#match::uri_template::v3::UriTemplateMatchConfig;
use prost::Message;

use crate::{
    errors::Error,
    openapi::{defaults::is_default_gateway_route, strip_gateway_tags},
    storage::{
        CreateRouteRepositoryRequest, RouteData, RouteRepository, UpdateRouteRepositoryRequest,
    },
    xds::filters::http::HttpScopedConfig,
    xds::route::{
        HeaderMatchConfig as XdsHeaderMatchConfig, PathMatch as XdsPathMatch,
        QueryParameterMatchConfig as XdsQueryParameterMatchConfig,
        RouteActionConfig as XdsRouteActionConfig, RouteConfig as XdsRouteConfig,
        RouteMatchConfig as XdsRouteMatchConfig, RouteRule as XdsRouteRule,
        VirtualHostConfig as XdsVirtualHostConfig,
        WeightedClusterConfig as XdsWeightedClusterConfig,
    },
};

use super::{error::ApiError, routes::ApiState};

// === Request & Response Models ===

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({
    "name": "primary-routes",
    "virtualHosts": [
        {
            "name": "default",
            "domains": ["*"],
            "routes": [
                {
                    "name": "api",
                    "match": {"path": {"type": "prefix", "value": "/api"}},
                    "action": {"type": "forward", "cluster": "api-cluster", "timeoutSeconds": 5}
                }
            ]
        }
    ]
}))]
pub struct RouteDefinition {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 1))]
    #[schema(min_items = 1, value_type = Vec<VirtualHostDefinition>)]
    pub virtual_hosts: Vec<VirtualHostDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualHostDefinition {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 1))]
    #[schema(min_items = 1)]
    pub domains: Vec<String>,

    #[validate(length(min = 1))]
    #[schema(min_items = 1, value_type = Vec<RouteRuleDefinition>)]
    pub routes: Vec<RouteRuleDefinition>,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub typed_per_filter_config: HashMap<String, HttpScopedConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RouteRuleDefinition {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(nested)]
    pub r#match: RouteMatchDefinition,

    pub action: RouteActionDefinition,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub typed_per_filter_config: HashMap<String, HttpScopedConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RouteMatchDefinition {
    pub path: PathMatchDefinition,

    #[serde(default)]
    #[schema(value_type = Vec<HeaderMatchDefinition>)]
    pub headers: Vec<HeaderMatchDefinition>,

    #[serde(default)]
    #[schema(value_type = Vec<QueryParameterMatchDefinition>)]
    pub query_parameters: Vec<QueryParameterMatchDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PathMatchDefinition {
    #[schema(example = json!({"type": "exact", "value": "/health"}))]
    Exact { value: String },
    #[schema(example = json!({"type": "prefix", "value": "/api"}))]
    Prefix { value: String },
    #[schema(example = json!({"type": "regex", "value": "^/v[0-9]+/.*"}))]
    Regex { value: String },
    #[schema(example = json!({"type": "template", "template": "/api/v1/users/{user_id}"}))]
    Template { template: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeaderMatchDefinition {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub present: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueryParameterMatchDefinition {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub present: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RouteActionDefinition {
    #[serde(rename_all = "camelCase")]
    Forward {
        #[schema(example = "demo_cluster_api_002")]
        cluster: String,
        #[serde(default)]
        #[schema(example = 5)]
        timeout_seconds: Option<u64>,
        #[serde(default)]
        #[schema(example = "/internal/api")]
        prefix_rewrite: Option<String>,
        #[serde(default)]
        #[schema(example = "/users/{user_id}")]
        template_rewrite: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Weighted {
        clusters: Vec<WeightedClusterDefinition>,
        #[serde(default)]
        total_weight: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    Redirect {
        #[serde(default)]
        host_redirect: Option<String>,
        #[serde(default)]
        path_redirect: Option<String>,
        #[serde(default)]
        response_code: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeightedClusterDefinition {
    #[schema(example = "blue-canary")]
    pub name: String,
    #[schema(example = 80)]
    pub weight: u32,

    #[serde(default)]
    #[schema(value_type = Object)]
    pub typed_per_filter_config: HashMap<String, HttpScopedConfig>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RouteResponse {
    pub name: String,
    pub path_prefix: String,
    pub cluster_targets: String,
    pub config: RouteDefinition,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListRoutesQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// === Handler Implementations ===

#[utoipa::path(
    post,
    path = "/api/v1/route-configs",
    request_body = RouteDefinition,
    responses(
        (status = 201, description = "Route configuration created", body = RouteResponse),
        (status = 400, description = "Validation error"),
        (status = 503, description = "Route repository unavailable"),
    ),
    tag = "route-configs"
)]
pub async fn create_route_handler(
    State(state): State<ApiState>,
    Json(payload): Json<RouteDefinition>,
) -> Result<(StatusCode, Json<RouteResponse>), ApiError> {
    validate_route_payload(&payload)?;

    let route_repository = require_route_repository(&state)?;

    let xds_config = payload.to_xds_config().and_then(validate_route_config)?;

    let (path_prefix, cluster_summary) = summarize_route(&payload);
    let configuration = serde_json::to_value(&xds_config).map_err(|err| {
        ApiError::from(Error::internal(format!("Failed to serialize route definition: {}", err)))
    })?;

    let request = CreateRouteRepositoryRequest {
        name: payload.name.clone(),
        path_prefix,
        cluster_name: cluster_summary,
        configuration,
    };

    let created = route_repository.create(request).await.map_err(ApiError::from)?;

    info!(route_id = %created.id, route_name = %created.name, "Route created via API");

    state.xds_state.refresh_routes_from_repository().await.map_err(|err| {
        error!(error = %err, "Failed to refresh xDS caches after route creation");
        ApiError::from(err)
    })?;

    let response = RouteResponse {
        name: created.name,
        path_prefix: created.path_prefix,
        cluster_targets: created.cluster_name,
        config: payload,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    get,
    path = "/api/v1/route-configs",
    params(
        ("limit" = Option<i32>, Query, description = "Maximum number of route configurations to return"),
        ("offset" = Option<i32>, Query, description = "Offset for paginated results"),
    ),
    responses(
        (status = 200, description = "List of route configurations", body = [RouteResponse]),
        (status = 503, description = "Route repository unavailable"),
    ),
    tag = "route-configs"
)]
pub async fn list_routes_handler(
    State(state): State<ApiState>,
    Query(params): Query<ListRoutesQuery>,
) -> Result<Json<Vec<RouteResponse>>, ApiError> {
    let repository = require_route_repository(&state)?;
    let rows = repository.list(params.limit, params.offset).await.map_err(ApiError::from)?;

    let mut routes = Vec::with_capacity(rows.len());
    for row in rows {
        routes.push(route_response_from_data(row)?);
    }

    Ok(Json(routes))
}

#[utoipa::path(
    get,
    path = "/api/v1/route-configs/{name}",
    params(("name" = String, Path, description = "Name of the route configuration")),
    responses(
        (status = 200, description = "Route configuration details", body = RouteResponse),
        (status = 404, description = "Route configuration not found"),
        (status = 503, description = "Route repository unavailable"),
    ),
    tag = "route-configs"
)]
pub async fn get_route_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> Result<Json<RouteResponse>, ApiError> {
    let repository = require_route_repository(&state)?;
    let route = repository.get_by_name(&name).await.map_err(ApiError::from)?;
    Ok(Json(route_response_from_data(route)?))
}

#[utoipa::path(
    put,
    path = "/api/v1/route-configs/{name}",
    params(("name" = String, Path, description = "Name of the route configuration")),
    request_body = RouteDefinition,
    responses(
        (status = 200, description = "Route configuration updated", body = RouteResponse),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Route configuration not found"),
        (status = 503, description = "Route repository unavailable"),
    ),
    tag = "route-configs"
)]
pub async fn update_route_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(payload): Json<RouteDefinition>,
) -> Result<Json<RouteResponse>, ApiError> {
    validate_route_payload(&payload)?;

    if payload.name != name {
        return Err(ApiError::BadRequest(format!(
            "Payload route name '{}' does not match path '{}'",
            payload.name, name
        )));
    }

    let repository = require_route_repository(&state)?;
    let existing = repository.get_by_name(&payload.name).await.map_err(ApiError::from)?;

    let xds_config = payload.to_xds_config().and_then(validate_route_config)?;
    let (path_prefix, cluster_summary) = summarize_route(&payload);
    let configuration = serde_json::to_value(&xds_config).map_err(|err| {
        ApiError::from(Error::internal(format!("Failed to serialize route definition: {}", err)))
    })?;

    let update_request = UpdateRouteRepositoryRequest {
        path_prefix: Some(path_prefix.clone()),
        cluster_name: Some(cluster_summary.clone()),
        configuration: Some(configuration),
    };

    let updated = repository.update(&existing.id, update_request).await.map_err(ApiError::from)?;

    info!(route_id = %updated.id, route_name = %updated.name, "Route updated via API");

    state.xds_state.refresh_routes_from_repository().await.map_err(|err| {
        error!(error = %err, "Failed to refresh xDS caches after route update");
        ApiError::from(err)
    })?;

    let response = RouteResponse {
        name: updated.name,
        path_prefix,
        cluster_targets: cluster_summary,
        config: payload,
    };

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/api/v1/route-configs/{name}",
    params(("name" = String, Path, description = "Name of the route configuration")),
    responses(
        (status = 204, description = "Route configuration deleted"),
        (status = 404, description = "Route configuration not found"),
        (status = 503, description = "Route repository unavailable"),
    ),
    tag = "route-configs"
)]
pub async fn delete_route_handler(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    if is_default_gateway_route(&name) {
        return Err(ApiError::Conflict(
            "The default gateway route configuration cannot be deleted".to_string(),
        ));
    }

    let repository = require_route_repository(&state)?;
    let existing = repository.get_by_name(&name).await.map_err(ApiError::from)?;

    repository.delete(&existing.id).await.map_err(ApiError::from)?;

    info!(route_id = %existing.id, route_name = %existing.name, "Route deleted via API");

    state.xds_state.refresh_routes_from_repository().await.map_err(|err| {
        error!(error = %err, "Failed to refresh xDS caches after route deletion");
        ApiError::from(err)
    })?;

    Ok(StatusCode::NO_CONTENT)
}

// === Conversion Helpers ===

impl RouteDefinition {
    fn to_xds_config(&self) -> Result<XdsRouteConfig, ApiError> {
        let virtual_hosts = self
            .virtual_hosts
            .iter()
            .map(VirtualHostDefinition::to_xds_config)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(XdsRouteConfig { name: self.name.clone(), virtual_hosts })
    }

    fn from_xds_config(config: &XdsRouteConfig) -> Self {
        RouteDefinition {
            name: config.name.clone(),
            virtual_hosts: config
                .virtual_hosts
                .iter()
                .map(VirtualHostDefinition::from_xds_config)
                .collect(),
        }
    }
}

impl VirtualHostDefinition {
    fn to_xds_config(&self) -> Result<XdsVirtualHostConfig, ApiError> {
        let routes = self
            .routes
            .iter()
            .map(RouteRuleDefinition::to_xds_config)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(XdsVirtualHostConfig {
            name: self.name.clone(),
            domains: self.domains.clone(),
            routes,
            typed_per_filter_config: self.typed_per_filter_config.clone(),
        })
    }

    fn from_xds_config(config: &XdsVirtualHostConfig) -> Self {
        VirtualHostDefinition {
            name: config.name.clone(),
            domains: config.domains.clone(),
            routes: config.routes.iter().map(RouteRuleDefinition::from_xds_config).collect(),
            typed_per_filter_config: config.typed_per_filter_config.clone(),
        }
    }
}

impl RouteRuleDefinition {
    fn to_xds_config(&self) -> Result<XdsRouteRule, ApiError> {
        Ok(XdsRouteRule {
            name: self.name.clone(),
            r#match: self.r#match.to_xds_config()?,
            action: self.action.to_xds_config()?,
            typed_per_filter_config: self.typed_per_filter_config.clone(),
        })
    }

    fn from_xds_config(config: &XdsRouteRule) -> Self {
        RouteRuleDefinition {
            name: config.name.clone(),
            r#match: RouteMatchDefinition::from_xds_config(&config.r#match),
            action: RouteActionDefinition::from_xds_config(&config.action),
            typed_per_filter_config: config.typed_per_filter_config.clone(),
        }
    }
}

impl RouteMatchDefinition {
    fn to_xds_config(&self) -> Result<XdsRouteMatchConfig, ApiError> {
        let headers = if self.headers.is_empty() {
            None
        } else {
            Some(self.headers.iter().map(HeaderMatchDefinition::to_xds_config).collect())
        };

        let query_parameters = if self.query_parameters.is_empty() {
            None
        } else {
            Some(
                self.query_parameters
                    .iter()
                    .map(QueryParameterMatchDefinition::to_xds_config)
                    .collect(),
            )
        };

        Ok(XdsRouteMatchConfig { path: self.path.to_xds_config(), headers, query_parameters })
    }

    fn from_xds_config(config: &XdsRouteMatchConfig) -> Self {
        RouteMatchDefinition {
            path: PathMatchDefinition::from_xds_config(&config.path),
            headers: config
                .headers
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(HeaderMatchDefinition::from_xds_config)
                .collect(),
            query_parameters: config
                .query_parameters
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(QueryParameterMatchDefinition::from_xds_config)
                .collect(),
        }
    }
}

impl PathMatchDefinition {
    fn to_xds_config(&self) -> XdsPathMatch {
        match self {
            PathMatchDefinition::Exact { value } => XdsPathMatch::Exact(value.clone()),
            PathMatchDefinition::Prefix { value } => XdsPathMatch::Prefix(value.clone()),
            PathMatchDefinition::Regex { value } => XdsPathMatch::Regex(value.clone()),
            PathMatchDefinition::Template { template } => XdsPathMatch::Template(template.clone()),
        }
    }

    fn from_xds_config(path: &XdsPathMatch) -> Self {
        match path {
            XdsPathMatch::Exact(value) => PathMatchDefinition::Exact { value: value.clone() },
            XdsPathMatch::Prefix(value) => PathMatchDefinition::Prefix { value: value.clone() },
            XdsPathMatch::Regex(value) => PathMatchDefinition::Regex { value: value.clone() },
            XdsPathMatch::Template(value) => {
                PathMatchDefinition::Template { template: value.clone() }
            }
        }
    }
}

impl HeaderMatchDefinition {
    fn to_xds_config(&self) -> XdsHeaderMatchConfig {
        XdsHeaderMatchConfig {
            name: self.name.clone(),
            value: self.value.clone(),
            regex: self.regex.clone(),
            present: self.present,
        }
    }

    fn from_xds_config(config: XdsHeaderMatchConfig) -> Self {
        HeaderMatchDefinition {
            name: config.name,
            value: config.value,
            regex: config.regex,
            present: config.present,
        }
    }
}

impl QueryParameterMatchDefinition {
    fn to_xds_config(&self) -> XdsQueryParameterMatchConfig {
        XdsQueryParameterMatchConfig {
            name: self.name.clone(),
            value: self.value.clone(),
            regex: self.regex.clone(),
            present: self.present,
        }
    }

    fn from_xds_config(config: XdsQueryParameterMatchConfig) -> Self {
        QueryParameterMatchDefinition {
            name: config.name,
            value: config.value,
            regex: config.regex,
            present: config.present,
        }
    }
}

impl RouteActionDefinition {
    fn to_xds_config(&self) -> Result<XdsRouteActionConfig, ApiError> {
        match self {
            RouteActionDefinition::Forward {
                cluster,
                timeout_seconds,
                prefix_rewrite,
                template_rewrite,
            } => Ok(XdsRouteActionConfig::Cluster {
                name: cluster.clone(),
                timeout: *timeout_seconds,
                prefix_rewrite: prefix_rewrite.clone(),
                path_template_rewrite: template_rewrite.clone(),
            }),
            RouteActionDefinition::Weighted { clusters, total_weight } => {
                if clusters.is_empty() {
                    return Err(ApiError::from(Error::validation(
                        "Weighted route must include at least one cluster",
                    )));
                }

                let weights = clusters
                    .iter()
                    .map(|cluster| XdsWeightedClusterConfig {
                        name: cluster.name.clone(),
                        weight: cluster.weight,
                        typed_per_filter_config: cluster.typed_per_filter_config.clone(),
                    })
                    .collect();

                Ok(XdsRouteActionConfig::WeightedClusters {
                    clusters: weights,
                    total_weight: *total_weight,
                })
            }
            RouteActionDefinition::Redirect { host_redirect, path_redirect, response_code } => {
                Ok(XdsRouteActionConfig::Redirect {
                    host_redirect: host_redirect.clone(),
                    path_redirect: path_redirect.clone(),
                    response_code: *response_code,
                })
            }
        }
    }

    fn from_xds_config(config: &XdsRouteActionConfig) -> Self {
        match config {
            XdsRouteActionConfig::Cluster {
                name,
                timeout,
                prefix_rewrite,
                path_template_rewrite,
            } => RouteActionDefinition::Forward {
                cluster: name.clone(),
                timeout_seconds: *timeout,
                prefix_rewrite: prefix_rewrite.clone(),
                template_rewrite: path_template_rewrite.clone(),
            },
            XdsRouteActionConfig::WeightedClusters { clusters, total_weight } => {
                RouteActionDefinition::Weighted {
                    clusters: clusters
                        .iter()
                        .map(|cluster| WeightedClusterDefinition {
                            name: cluster.name.clone(),
                            weight: cluster.weight,
                            typed_per_filter_config: cluster.typed_per_filter_config.clone(),
                        })
                        .collect(),
                    total_weight: *total_weight,
                }
            }
            XdsRouteActionConfig::Redirect { host_redirect, path_redirect, response_code } => {
                RouteActionDefinition::Redirect {
                    host_redirect: host_redirect.clone(),
                    path_redirect: path_redirect.clone(),
                    response_code: *response_code,
                }
            }
        }
    }
}

// === Utility Functions ===

fn require_route_repository(state: &ApiState) -> Result<RouteRepository, ApiError> {
    state
        .xds_state
        .route_repository
        .as_ref()
        .cloned()
        .ok_or_else(|| ApiError::service_unavailable("Route repository not configured"))
}

fn route_response_from_data(data: RouteData) -> Result<RouteResponse, ApiError> {
    let mut value: Value = serde_json::from_str(&data.configuration).map_err(|err| {
        ApiError::from(Error::internal(format!(
            "Failed to parse stored route configuration: {}",
            err
        )))
    })?;

    strip_gateway_tags(&mut value);

    let xds_config: XdsRouteConfig = serde_json::from_value(value).map_err(|err| {
        ApiError::from(Error::internal(format!(
            "Failed to deserialize stored route configuration: {}",
            err
        )))
    })?;

    let config = RouteDefinition::from_xds_config(&xds_config);

    Ok(RouteResponse {
        name: data.name,
        path_prefix: data.path_prefix,
        cluster_targets: data.cluster_name,
        config,
    })
}

fn summarize_route(definition: &RouteDefinition) -> (String, String) {
    let path_prefix = definition
        .virtual_hosts
        .iter()
        .flat_map(|vh| vh.routes.iter())
        .map(|route| match &route.r#match.path {
            PathMatchDefinition::Exact { value } | PathMatchDefinition::Prefix { value } => {
                value.clone()
            }
            PathMatchDefinition::Regex { value } => format!("regex:{}", value),
            PathMatchDefinition::Template { template } => format!("template:{}", template),
        })
        .next()
        .unwrap_or_else(|| "*".to_string());

    let cluster_summary = definition
        .virtual_hosts
        .iter()
        .flat_map(|vh| vh.routes.iter())
        .map(|route| match &route.action {
            RouteActionDefinition::Forward { cluster, .. } => cluster.clone(),
            RouteActionDefinition::Weighted { clusters, .. } => {
                clusters.first().map(|cluster| cluster.name.clone()).unwrap_or_default()
            }
            RouteActionDefinition::Redirect { .. } => "__redirect__".to_string(),
        })
        .next()
        .unwrap_or_else(|| "unknown".to_string());

    (path_prefix, cluster_summary)
}

fn validate_route_config(config: XdsRouteConfig) -> Result<XdsRouteConfig, ApiError> {
    config.to_envoy_route_configuration().map_err(ApiError::from)?;
    Ok(config)
}

fn validate_route_payload(definition: &RouteDefinition) -> Result<(), ApiError> {
    definition.validate().map_err(|err| ApiError::from(Error::from(err)))?;

    for virtual_host in &definition.virtual_hosts {
        virtual_host.validate().map_err(|err| ApiError::from(Error::from(err)))?;

        if virtual_host.domains.iter().any(|domain| domain.trim().is_empty()) {
            return Err(validation_error("Virtual host domains must not be empty"));
        }

        for route in &virtual_host.routes {
            route.validate().map_err(|err| ApiError::from(Error::from(err)))?;
            validate_route_match(&route.r#match)?;
            validate_route_action(&route.action)?;

            match (&route.r#match.path, &route.action) {
                (
                    PathMatchDefinition::Template { .. },
                    RouteActionDefinition::Forward { prefix_rewrite: Some(_), .. },
                ) => {
                    return Err(validation_error(
                        "Template path matches do not support prefixRewrite",
                    ));
                }
                (PathMatchDefinition::Template { .. }, RouteActionDefinition::Forward { .. }) => {}
                (PathMatchDefinition::Template { .. }, _) => {
                    return Err(validation_error("Template path matches require a forward action"));
                }
                (_, RouteActionDefinition::Forward { template_rewrite: Some(_), .. }) => {
                    return Err(validation_error("templateRewrite requires a template path match"));
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn validate_route_match(r#match: &RouteMatchDefinition) -> Result<(), ApiError> {
    match &r#match.path {
        PathMatchDefinition::Exact { value } | PathMatchDefinition::Prefix { value } => {
            if value.trim().is_empty() {
                return Err(validation_error("Route match path value must not be empty"));
            }
        }
        PathMatchDefinition::Regex { value } => {
            if value.trim().is_empty() {
                return Err(validation_error("Route match path value must not be empty"));
            }
        }
        PathMatchDefinition::Template { template } => {
            if template.trim().is_empty() {
                return Err(validation_error("Route match template must not be empty"));
            }

            ensure_valid_uri_template(template)?;
        }
    }

    if r#match.headers.iter().any(|header| header.name.trim().is_empty()) {
        return Err(validation_error("Header match name must not be empty"));
    }

    if r#match.query_parameters.iter().any(|param| param.name.trim().is_empty()) {
        return Err(validation_error("Query parameter match name must not be empty"));
    }

    Ok(())
}

fn validate_route_action(action: &RouteActionDefinition) -> Result<(), ApiError> {
    match action {
        RouteActionDefinition::Forward { cluster, prefix_rewrite, template_rewrite, .. } => {
            if cluster.trim().is_empty() {
                return Err(validation_error("Forward action requires a cluster name"));
            }

            if let Some(prefix) = prefix_rewrite {
                if prefix.trim().is_empty() {
                    return Err(validation_error("prefixRewrite must not be an empty string"));
                }

                if !prefix.starts_with('/') {
                    return Err(validation_error("prefixRewrite must start with a slash"));
                }
            }

            if let Some(template) = template_rewrite {
                if template.trim().is_empty() {
                    return Err(validation_error("templateRewrite must not be an empty string"));
                }

                ensure_valid_uri_template(template)?;
            }
        }
        RouteActionDefinition::Weighted { clusters, .. } => {
            if clusters.is_empty() {
                return Err(validation_error("Weighted action must include at least one cluster"));
            }

            if clusters.iter().any(|cluster| cluster.name.trim().is_empty()) {
                return Err(validation_error("Weighted action cluster names must not be empty"));
            }

            if clusters.iter().any(|cluster| cluster.weight == 0) {
                return Err(validation_error(
                    "Weighted action cluster weights must be greater than zero",
                ));
            }
        }
        RouteActionDefinition::Redirect { host_redirect, path_redirect, .. } => {
            if host_redirect.as_ref().map(|s| s.trim().is_empty()).unwrap_or(false)
                || path_redirect.as_ref().map(|s| s.trim().is_empty()).unwrap_or(false)
            {
                return Err(validation_error("Redirect action values must not be empty strings"));
            }
        }
    }

    Ok(())
}

fn validation_error(message: impl Into<String>) -> ApiError {
    ApiError::from(Error::validation(message.into()))
}

fn ensure_valid_uri_template(template: &str) -> Result<(), ApiError> {
    let config = UriTemplateMatchConfig { path_template: template.to_string() };

    if config.encode_to_vec().is_empty() {
        Err(validation_error("Invalid URI template"))
    } else {
        Ok(())
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, Json};
    use serde_json::json;
    use sqlx::Executor;
    use std::sync::Arc;

    use crate::config::SimpleXdsConfig;
    use crate::storage::{create_pool, CreateClusterRequest, DatabaseConfig};
    use crate::xds::filters::http::{
        local_rate_limit::{
            FractionalPercentDenominator, LocalRateLimitConfig, RuntimeFractionalPercentConfig,
            TokenBucketConfig,
        },
        HttpScopedConfig,
    };
    use crate::xds::XdsState;

    async fn setup_state() -> ApiState {
        let pool = create_pool(&DatabaseConfig {
            url: "sqlite://:memory:".to_string(),
            auto_migrate: false,
            ..Default::default()
        })
        .await
        .expect("pool");

        pool.execute(
            r#"
            CREATE TABLE IF NOT EXISTS clusters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                service_name TEXT NOT NULL,
                configuration TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(name, version)
            );

            CREATE TABLE IF NOT EXISTS routes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path_prefix TEXT NOT NULL,
                cluster_name TEXT NOT NULL,
                configuration TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(name, version)
            );
        "#,
        )
        .await
        .expect("create tables");

        let state = XdsState::with_database(SimpleXdsConfig::default(), pool.clone());
        let api_state = ApiState { xds_state: Arc::new(state) };

        // Seed a cluster for route references
        let cluster_repo =
            api_state.xds_state.cluster_repository.as_ref().cloned().expect("cluster repo");

        cluster_repo
            .create(CreateClusterRequest {
                name: "api-cluster".into(),
                service_name: "api-cluster".into(),
                configuration: json!({
                    "endpoints": ["127.0.0.1:8080"]
                }),
            })
            .await
            .expect("seed cluster");

        cluster_repo
            .create(CreateClusterRequest {
                name: "shadow".into(),
                service_name: "shadow".into(),
                configuration: json!({
                    "endpoints": ["127.0.0.1:8181"]
                }),
            })
            .await
            .expect("seed shadow cluster");

        api_state
    }

    fn sample_route_definition() -> RouteDefinition {
        RouteDefinition {
            name: "primary-routes".into(),
            virtual_hosts: vec![VirtualHostDefinition {
                name: "default".into(),
                domains: vec!["*".into()],
                routes: vec![RouteRuleDefinition {
                    name: Some("api".into()),
                    r#match: RouteMatchDefinition {
                        path: PathMatchDefinition::Prefix { value: "/api".into() },
                        headers: vec![],
                        query_parameters: vec![],
                    },
                    action: RouteActionDefinition::Forward {
                        cluster: "api-cluster".into(),
                        timeout_seconds: Some(5),
                        prefix_rewrite: None,
                        template_rewrite: None,
                    },
                    typed_per_filter_config: HashMap::new(),
                }],
                typed_per_filter_config: HashMap::new(),
            }],
        }
    }

    #[tokio::test]
    async fn create_route_persists_configuration() {
        let state = setup_state().await;

        let payload = sample_route_definition();
        let (status, Json(created)) =
            create_route_handler(State(state.clone()), Json(payload.clone()))
                .await
                .expect("create route");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.name, "primary-routes");
        assert_eq!(created.config.virtual_hosts.len(), 1);

        let repo = state.xds_state.route_repository.as_ref().cloned().expect("route repo");
        let stored = repo.get_by_name("primary-routes").await.expect("stored route");
        assert_eq!(stored.path_prefix, "/api");
        assert!(stored.cluster_name.contains("api-cluster"));
    }

    #[tokio::test]
    async fn list_routes_returns_entries() {
        let state = setup_state().await;

        let payload = sample_route_definition();
        let (status, _) =
            create_route_handler(State(state.clone()), Json(payload)).await.expect("create route");
        assert_eq!(status, StatusCode::CREATED);

        let response = list_routes_handler(State(state), Query(ListRoutesQuery::default()))
            .await
            .expect("list routes");

        assert_eq!(response.0.len(), 1);
        assert_eq!(response.0[0].name, "primary-routes");
    }

    #[tokio::test]
    async fn get_route_returns_definition() {
        let state = setup_state().await;
        let payload = sample_route_definition();
        let (status, _) =
            create_route_handler(State(state.clone()), Json(payload)).await.expect("create route");
        assert_eq!(status, StatusCode::CREATED);

        let response = get_route_handler(State(state), Path("primary-routes".into()))
            .await
            .expect("get route");

        assert_eq!(response.0.name, "primary-routes");
        assert_eq!(response.0.config.virtual_hosts[0].routes.len(), 1);
    }

    #[tokio::test]
    async fn update_route_applies_changes() {
        let state = setup_state().await;
        let mut payload = sample_route_definition();
        let (status, _) = create_route_handler(State(state.clone()), Json(payload.clone()))
            .await
            .expect("create route");
        assert_eq!(status, StatusCode::CREATED);

        payload.virtual_hosts[0].routes[0].action = RouteActionDefinition::Weighted {
            clusters: vec![
                WeightedClusterDefinition {
                    name: "api-cluster".into(),
                    weight: 60,
                    typed_per_filter_config: HashMap::new(),
                },
                WeightedClusterDefinition {
                    name: "shadow".into(),
                    weight: 40,
                    typed_per_filter_config: HashMap::new(),
                },
            ],
            total_weight: Some(100),
        };
        payload.virtual_hosts[0].routes[0].typed_per_filter_config.insert(
            "envoy.filters.http.local_ratelimit".into(),
            HttpScopedConfig::LocalRateLimit(LocalRateLimitConfig {
                stat_prefix: "per_route".into(),
                token_bucket: Some(TokenBucketConfig {
                    max_tokens: 10,
                    tokens_per_fill: Some(10),
                    fill_interval_ms: 60_000,
                }),
                status_code: Some(429),
                filter_enabled: Some(RuntimeFractionalPercentConfig {
                    runtime_key: None,
                    numerator: 100,
                    denominator: FractionalPercentDenominator::Hundred,
                }),
                filter_enforced: Some(RuntimeFractionalPercentConfig {
                    runtime_key: None,
                    numerator: 100,
                    denominator: FractionalPercentDenominator::Hundred,
                }),
                per_downstream_connection: Some(false),
                rate_limited_as_resource_exhausted: None,
                max_dynamic_descriptors: None,
                always_consume_default_token_bucket: Some(false),
            }),
        );

        let response = update_route_handler(
            State(state.clone()),
            Path("primary-routes".into()),
            Json(payload.clone()),
        )
        .await
        .expect("update route");

        assert!(response.0.cluster_targets.contains("api-cluster"));
        if let Some(HttpScopedConfig::LocalRateLimit(cfg)) = response.0.config.virtual_hosts[0]
            .routes[0]
            .typed_per_filter_config
            .get("envoy.filters.http.local_ratelimit")
        {
            let bucket = cfg.token_bucket.as_ref().expect("route-level token bucket present");
            assert_eq!(bucket.max_tokens, 10);
            assert_eq!(bucket.tokens_per_fill, Some(10));
        } else {
            panic!("expected local rate limit override in response");
        }

        let repo = state.xds_state.route_repository.as_ref().cloned().expect("route repo");
        let stored = repo.get_by_name("primary-routes").await.expect("stored route");
        let stored_config: XdsRouteConfig = serde_json::from_str(&stored.configuration).unwrap();
        assert!(stored_config.virtual_hosts[0].routes[0]
            .typed_per_filter_config
            .contains_key("envoy.filters.http.local_ratelimit"));
        assert_eq!(stored.version, 2);
    }

    #[tokio::test]
    async fn delete_route_removes_row() {
        let state = setup_state().await;
        let payload = sample_route_definition();
        let (status, _) =
            create_route_handler(State(state.clone()), Json(payload)).await.expect("create route");
        assert_eq!(status, StatusCode::CREATED);

        let status = delete_route_handler(State(state.clone()), Path("primary-routes".into()))
            .await
            .expect("delete route");

        assert_eq!(status, StatusCode::NO_CONTENT);

        let repo = state.xds_state.route_repository.as_ref().cloned().expect("route repo");
        assert!(repo.get_by_name("primary-routes").await.is_err());
    }

    #[tokio::test]
    async fn template_route_supports_rewrite() {
        let state = setup_state().await;

        let mut payload = sample_route_definition();
        payload.name = "template-route".into();
        payload.virtual_hosts[0].routes[0].r#match.path =
            PathMatchDefinition::Template { template: "/api/v1/users/{user_id}".into() };
        payload.virtual_hosts[0].routes[0].action = RouteActionDefinition::Forward {
            cluster: "api-cluster".into(),
            timeout_seconds: Some(5),
            prefix_rewrite: None,
            template_rewrite: Some("/users/{user_id}".into()),
        };

        let (status, Json(created)) =
            create_route_handler(State(state.clone()), Json(payload.clone()))
                .await
                .expect("create template route");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.name, "template-route");
        let route = &created.config.virtual_hosts[0].routes[0];
        assert!(matches!(route.r#match.path, PathMatchDefinition::Template { .. }));
        if let RouteActionDefinition::Forward { template_rewrite, .. } = &route.action {
            assert_eq!(template_rewrite.as_deref(), Some("/users/{user_id}"));
        } else {
            panic!("expected forward action");
        }

        let repo = state.xds_state.route_repository.as_ref().cloned().expect("route repo");
        let stored = repo.get_by_name("template-route").await.expect("stored template route");
        assert_eq!(stored.path_prefix, "template:/api/v1/users/{user_id}".to_string());
    }
}
