//! Platform API OpenAPI import handlers with x-flowplane-* tag support

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, Request, StatusCode},
    Json,
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_yaml;
use tracing::info;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    api::platform_api_definitions::{
        ApiDefinition, ApiPolicies, ApiRoute, AuthenticationPolicy, CorsPolicy, RateLimitPolicy,
        UpstreamConfig, UpstreamEndpoint,
    },
    api::{error::ApiError, routes::ApiState},
};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct OpenApiImportQuery {
    /// Name for the imported API definition
    pub name: String,

    /// Optional version override (defaults to OpenAPI version)
    #[serde(default)]
    pub version: Option<String>,

    /// Optional base path override (defaults to server URL path)
    #[serde(default)]
    pub base_path: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiImportResponse {
    /// API definition ID
    pub id: String,

    /// API name
    pub name: String,

    /// API version
    pub version: String,

    /// Base path
    pub base_path: String,

    /// Upstream configuration
    pub upstream: UpstreamConfig,

    /// Routes extracted from OpenAPI paths
    pub routes: Vec<ApiRoute>,

    /// Policies extracted from x-flowplane-* tags
    pub policies: Option<ApiPolicies>,

    /// OpenAPI metadata preserved
    pub metadata: Option<Value>,

    /// Warnings about unrecognized tags or issues
    pub warnings: Option<Vec<String>>,

    /// Creation timestamp
    pub created_at: String,
}

/// Extract x-flowplane-* tags from OpenAPI operation
fn extract_flowplane_policies(operation: &Value) -> (Option<ApiPolicies>, Vec<String>) {
    let mut policies = ApiPolicies {
        rate_limit: None,
        authentication: None,
        authorization: None,
        cors: None,
        circuit_breaker: None,
        retry: None,
        timeout: None,
    };
    let mut warnings = Vec::new();

    // Check for x-flowplane-ratelimit
    if let Some(rate_limit) = operation.get("x-flowplane-ratelimit") {
        if let Some(requests) = rate_limit.get("requests").and_then(|r| r.as_u64()) {
            if let Some(interval) = rate_limit.get("interval").and_then(|i| i.as_str()) {
                policies.rate_limit = Some(RateLimitPolicy {
                    requests: requests as u32,
                    interval: interval.to_string(),
                    key_by: rate_limit.get("keyBy").and_then(|k| k.as_str()).map(|s| s.to_string()),
                });
            }
        } else {
            warnings.push("Invalid x-flowplane-ratelimit format".to_string());
        }
    }

    // Check for x-flowplane-jwt-auth
    if let Some(auth) = operation.get("x-flowplane-jwt-auth") {
        let required = auth.get("required").and_then(|r| r.as_bool()).unwrap_or(true);
        let config = json!({
            "issuer": auth.get("issuer"),
            "audience": auth.get("audience"),
        });

        policies.authentication = Some(AuthenticationPolicy {
            auth_type: "jwt".to_string(),
            required,
            config: Some(config),
        });
    }

    // Check for x-flowplane-cors
    if let Some(cors) = operation.get("x-flowplane-cors") {
        let origins = cors
            .get("origins")
            .and_then(|o| o.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["*".to_string()]);

        let methods = cors
            .get("methods")
            .and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["GET".to_string(), "POST".to_string()]);

        let headers = cors
            .get("headers")
            .and_then(|h| h.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_else(|| vec!["Content-Type".to_string(), "Authorization".to_string()]);

        policies.cors = Some(CorsPolicy {
            origins,
            methods,
            headers,
            allow_credentials: cors
                .get("allowCredentials")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            max_age: cors.get("maxAge").and_then(|m| m.as_u64()).map(|u| u as u32),
        });
    }

    // Check for unknown x-flowplane-* tags
    for (key, _value) in operation.as_object().unwrap_or(&serde_json::Map::new()) {
        if key.starts_with("x-flowplane-") {
            let known_tags = ["x-flowplane-ratelimit", "x-flowplane-jwt-auth", "x-flowplane-cors"];
            if !known_tags.contains(&key.as_str()) {
                warnings.push(format!("Unknown flowplane tag: {}", key));
            }
        }
    }

    let has_policies = policies.rate_limit.is_some()
        || policies.authentication.is_some()
        || policies.cors.is_some();

    (if has_policies { Some(policies) } else { None }, warnings)
}

/// Parse OpenAPI document and convert to Platform API definition
fn openapi_to_api_definition(
    spec: &Value,
    name: String,
    version_override: Option<String>,
    base_path_override: Option<String>,
) -> Result<(ApiDefinition, Vec<String>), ApiError> {
    let mut warnings = Vec::new();

    // Extract version
    let version = version_override.unwrap_or_else(|| {
        spec.get("info")
            .and_then(|i| i.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string()
    });

    // Extract base path from servers or use override
    let base_path = base_path_override.unwrap_or_else(|| {
        spec.get("servers")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|server| server.get("url"))
            .and_then(|url| url.as_str())
            .and_then(|url| {
                // Extract path from URL
                if let Ok(parsed) = url::Url::parse(url) {
                    Some(parsed.path().to_string())
                } else if url.starts_with('/') {
                    Some(url.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "/".to_string())
    });

    // Extract upstream from servers
    let upstream = spec
        .get("servers")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .and_then(|server| server.get("url"))
        .and_then(|url| url.as_str())
        .and_then(|url| {
            if let Ok(parsed) = url::Url::parse(url) {
                let host = parsed.host_str()?;
                let port =
                    parsed.port().unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });
                let tls = parsed.scheme() == "https";

                Some(UpstreamConfig {
                    service: format!("{}-backend", name),
                    endpoints: vec![UpstreamEndpoint { host: host.to_string(), port, weight: 100 }],
                    tls,
                    load_balancing: "ROUND_ROBIN".to_string(),
                })
            } else {
                None
            }
        })
        .unwrap_or_else(|| UpstreamConfig {
            service: format!("{}-backend", name),
            endpoints: vec![UpstreamEndpoint {
                host: "backend.example.com".to_string(),
                port: 80,
                weight: 100,
            }],
            tls: false,
            load_balancing: "ROUND_ROBIN".to_string(),
        });

    // Extract routes and policies from paths
    let mut routes = Vec::new();
    let mut global_policies: Option<ApiPolicies> = None;

    if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
        for (path, path_item) in paths {
            // Process each HTTP method
            for method in &["get", "post", "put", "delete", "patch", "options", "head"] {
                if let Some(operation) = path_item.get(method) {
                    let description = operation
                        .get("summary")
                        .or_else(|| operation.get("description"))
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string());

                    // Extract policies from x-flowplane-* tags
                    let (policies, mut op_warnings) = extract_flowplane_policies(operation);
                    warnings.append(&mut op_warnings);

                    // Merge policies into global if this is the first route
                    if routes.is_empty() && policies.is_some() {
                        global_policies = policies.clone();
                    }

                    routes.push(ApiRoute {
                        path: path.clone(),
                        methods: vec![method.to_uppercase()],
                        description,
                        policies,
                    });
                }
            }
        }
    }

    let api = ApiDefinition {
        name: name.clone(),
        version,
        base_path,
        upstream,
        routes,
        policies: global_policies,
        metadata: Some(json!({
            "openapi_version": spec.get("openapi").and_then(|v| v.as_str()).unwrap_or("3.0.0"),
            "info": spec.get("info"),
        })),
    };

    Ok((api, warnings))
}

/// Import OpenAPI specification to Platform API definition
#[utoipa::path(
    post,
    path = "/api/v1/platform/import/openapi",
    params(OpenApiImportQuery),
    request_body = String,
    responses(
        (status = 201, description = "API definition created from OpenAPI", body = OpenApiImportResponse),
        (status = 400, description = "Invalid OpenAPI specification"),
        (status = 403, description = "Insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    tag = "platform-import"
)]
pub async fn import_openapi_handler(
    State(_state): State<ApiState>,
    Query(params): Query<OpenApiImportQuery>,
    request: Request<Body>,
) -> Result<(StatusCode, Json<OpenApiImportResponse>), ApiError> {
    let (parts, body) = request.into_parts();
    let collected = body
        .collect()
        .await
        .map_err(|err| ApiError::BadRequest(format!("Failed to read body: {}", err)))?;

    let bytes = collected.to_bytes();

    // Determine content type and parse accordingly
    let content_type = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let spec: Value = if content_type.contains("yaml") {
        serde_yaml::from_slice(&bytes)
            .map_err(|err| ApiError::BadRequest(format!("Invalid YAML: {}", err)))?
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|err| ApiError::BadRequest(format!("Invalid JSON: {}", err)))?
    };

    // Validate it's an OpenAPI 3.x document
    if !spec.get("openapi").and_then(|v| v.as_str()).map(|v| v.starts_with("3.")).unwrap_or(false) {
        return Err(ApiError::BadRequest(
            "Only OpenAPI 3.x specifications are supported".to_string(),
        ));
    }

    // Convert to API definition
    let (api_def, warnings) =
        openapi_to_api_definition(&spec, params.name.clone(), params.version, params.base_path)?;

    // Generate ID for the API
    let api_id = Uuid::new_v4().to_string();

    info!("Imported OpenAPI spec '{}' as Platform API definition '{}'", params.name, api_id);

    // Create response
    let response = OpenApiImportResponse {
        id: api_id,
        name: api_def.name,
        version: api_def.version,
        base_path: api_def.base_path,
        upstream: api_def.upstream,
        routes: api_def.routes,
        policies: api_def.policies,
        metadata: api_def.metadata,
        warnings: if warnings.is_empty() { None } else { Some(warnings) },
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Redirect handler for old gateway endpoint
pub async fn redirect_gateway_import_handler(
    Query(params): Query<OpenApiImportQuery>,
) -> Result<axum::response::Response, ApiError> {
    let new_location = format!("/api/v1/platform/import/openapi?name={}", params.name);

    Ok(axum::response::Response::builder()
        .status(StatusCode::PERMANENT_REDIRECT)
        .header(header::LOCATION, new_location)
        .body(Body::empty())
        .unwrap())
}
