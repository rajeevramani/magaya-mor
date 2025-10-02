//! Transformers for converting between Native and Platform API representations

use serde_json::{json, Value};

use crate::{
    api::handlers::ClusterResponse,
    api::platform_service_handlers::{
        LoadBalancingStrategy, ServiceCircuitBreaker, ServiceDefinition, ServiceEndpoint,
        ServiceHealthCheck, ServiceOutlierDetection, ServiceResponse,
    },
    xds::{
        CircuitBreakerThresholdsSpec, ClusterSpec, EndpointSpec, HealthCheckSpec,
        OutlierDetectionSpec,
    },
};

/// Transform a Native API cluster to a Platform API service
pub fn cluster_to_service(cluster: &ClusterSpec) -> ServiceResponse {
    let endpoints: Vec<ServiceEndpoint> = cluster
        .endpoints
        .iter()
        .map(|ep| {
            // Handle EndpointSpec enum variants
            match ep {
                EndpointSpec::String(s) => {
                    // Parse host:port format from string
                    if let Some(colon_pos) = s.rfind(':') {
                        let host = s[..colon_pos].to_string();
                        let port = s[colon_pos + 1..].parse().unwrap_or(80);
                        ServiceEndpoint { host, port, weight: 100, metadata: Default::default() }
                    } else {
                        ServiceEndpoint {
                            host: s.clone(),
                            port: 80,
                            weight: 100,
                            metadata: Default::default(),
                        }
                    }
                }
                EndpointSpec::Address { host, port } => ServiceEndpoint {
                    host: host.clone(),
                    port: *port,
                    weight: 100,
                    metadata: Default::default(),
                },
            }
        })
        .collect();

    // Parse load balancing policy
    let load_balancing = match cluster.lb_policy.as_deref() {
        Some("ROUND_ROBIN") => LoadBalancingStrategy::RoundRobin,
        Some("RANDOM") => LoadBalancingStrategy::Random,
        Some("LEAST_REQUEST") => LoadBalancingStrategy::LeastRequest,
        _ => LoadBalancingStrategy::RoundRobin,
    };

    // Convert health checks
    let health_check = cluster.health_checks.first().map(|hc| match hc {
        HealthCheckSpec::Http { path, interval_seconds, timeout_seconds, .. } => {
            ServiceHealthCheck {
                path: path.clone(),
                interval: interval_seconds.unwrap_or(10) as u32,
                timeout: timeout_seconds.unwrap_or(5) as u32,
                healthy_threshold: 3,
                unhealthy_threshold: 3,
            }
        }
        HealthCheckSpec::Tcp { interval_seconds, timeout_seconds, .. } => ServiceHealthCheck {
            path: "/".to_string(), // TCP checks don't have a path, use default
            interval: interval_seconds.unwrap_or(10) as u32,
            timeout: timeout_seconds.unwrap_or(5) as u32,
            healthy_threshold: 3,
            unhealthy_threshold: 3,
        },
    });

    // Convert circuit breakers
    let circuit_breaker = cluster.circuit_breakers.as_ref().and_then(|cb| {
        let default_thresholds = cb.default.as_ref().or(cb.high.as_ref())?;
        Some(ServiceCircuitBreaker {
            max_requests: default_thresholds.max_requests,
            max_pending_requests: default_thresholds.max_pending_requests,
            max_connections: default_thresholds.max_connections,
            max_retries: default_thresholds.max_retries,
            consecutive_errors: Some(5), // Default value
            interval_ms: Some(10000),    // Default value (10 seconds)
        })
    });

    // Convert outlier detection
    let outlier_detection = cluster.outlier_detection.as_ref().map(|od| {
        ServiceOutlierDetection {
            consecutive_5xx: od.consecutive_5xx,
            interval_ms: od.interval_seconds.map(|s| s * 1000), // Convert seconds to milliseconds
            base_ejection_time_ms: od.base_ejection_time_seconds.map(|s| s * 1000),
            max_ejection_percent: od.max_ejection_percent,
            min_healthy_percent: None, // Default
        }
    });

    // Generate cluster ID from cluster name
    let cluster_id = format!(
        "{}-cluster",
        generate_id_from_name(cluster.lb_policy.as_deref().unwrap_or("unknown"))
    );

    ServiceResponse {
        name: "cluster".to_string(), // ClusterSpec doesn't have a name field
        cluster_id,
        endpoints,
        load_balancing,
        health_check,
        circuit_breaker,
        outlier_detection,
        metadata: Some(json!({
            "source": "native_api",
            "cluster_name": cluster.lb_policy.as_deref().unwrap_or("unknown"),
        })),
    }
}

/// Transform a Platform API service to a Native API cluster response
pub fn service_to_cluster_response(
    service: &ServiceDefinition,
    cluster_name: &str,
) -> ClusterResponse {
    let endpoints = service
        .endpoints
        .iter()
        .map(|ep| EndpointSpec::Address { host: ep.host.clone(), port: ep.port })
        .collect();

    // Convert load balancing strategy enum to string
    let lb_policy = match service.load_balancing {
        LoadBalancingStrategy::RoundRobin => Some("ROUND_ROBIN".to_string()),
        LoadBalancingStrategy::Random => Some("RANDOM".to_string()),
        LoadBalancingStrategy::LeastRequest => Some("LEAST_REQUEST".to_string()),
        LoadBalancingStrategy::RingHash => Some("RING_HASH".to_string()),
        LoadBalancingStrategy::Maglev => Some("MAGLEV".to_string()),
    };

    let health_checks = service
        .health_check
        .as_ref()
        .map(|hc| {
            vec![HealthCheckSpec::Http {
                path: hc.path.clone(),
                host: None,
                method: None,
                interval_seconds: Some(hc.interval as u64),
                timeout_seconds: Some(hc.timeout as u64),
                unhealthy_threshold: Some(hc.unhealthy_threshold),
                healthy_threshold: Some(hc.healthy_threshold),
                expected_statuses: None,
            }]
        })
        .unwrap_or_default();

    let circuit_breakers =
        service.circuit_breaker.as_ref().map(|cb| crate::xds::CircuitBreakersSpec {
            default: Some(CircuitBreakerThresholdsSpec {
                max_connections: cb.max_connections,
                max_pending_requests: cb.max_pending_requests,
                max_requests: cb.max_requests,
                max_retries: cb.max_retries,
            }),
            high: None,
        });

    let outlier_detection = service.outlier_detection.as_ref().map(|od| {
        OutlierDetectionSpec {
            consecutive_5xx: od.consecutive_5xx,
            interval_seconds: od.interval_ms.map(|ms| ms / 1000), // Convert milliseconds to seconds
            base_ejection_time_seconds: od.base_ejection_time_ms.map(|ms| ms / 1000),
            max_ejection_percent: od.max_ejection_percent,
        }
    });

    // Create the ClusterSpec
    let config = ClusterSpec {
        connect_timeout_seconds: Some(5), // Default
        endpoints,
        use_tls: Some(false), // Default, could be derived from port
        tls_server_name: None,
        dns_lookup_family: None,
        lb_policy,
        least_request: None,
        ring_hash: None,
        maglev: None,
        circuit_breakers,
        health_checks,
        outlier_detection,
    };

    ClusterResponse { name: cluster_name.to_string(), service_name: service.name.clone(), config }
}

/// Transform route configurations to simplified API definition view
pub fn routes_to_api_summary(route_name: &str, route_spec: &Value) -> Value {
    // Extract virtual hosts and routes
    let virtual_hosts = route_spec
        .get("virtual_hosts")
        .and_then(|vh| vh.as_array())
        .map(|vhs| vhs.to_vec())
        .unwrap_or_default();

    let mut all_routes = Vec::new();
    let mut domains = Vec::new();

    for vh in &virtual_hosts {
        if let Some(vh_domains) = vh.get("domains").and_then(|d| d.as_array()) {
            for domain in vh_domains {
                if let Some(d) = domain.as_str() {
                    domains.push(d.to_string());
                }
            }
        }

        if let Some(routes) = vh.get("routes").and_then(|r| r.as_array()) {
            all_routes.extend(routes.iter().cloned());
        }
    }

    json!({
        "id": route_name,
        "name": route_name,
        "domains": domains,
        "routeCount": all_routes.len(),
        "routes": all_routes,
        "source": "native_api",
    })
}

/// Check if a cluster represents a Platform API service
pub fn is_platform_service_cluster(cluster_name: &str) -> bool {
    // Platform services typically have a "-cluster" suffix
    cluster_name.ends_with("-cluster") || cluster_name.contains("-service")
}

/// Extract service name from cluster name
pub fn cluster_name_to_service_name(cluster_name: &str) -> String {
    if cluster_name.ends_with("-cluster") {
        cluster_name.trim_end_matches("-cluster").to_string()
    } else {
        cluster_name.to_string()
    }
}

/// Check if a route configuration represents a Platform API definition
pub fn is_platform_api_routes(route_name: &str) -> bool {
    // Platform API definitions typically have a "-routes" suffix
    route_name.ends_with("-routes") || route_name.contains("-api-")
}

/// Transform multiple clusters to service list with filtering
pub fn clusters_to_services(clusters: Vec<ClusterSpec>) -> Vec<ServiceResponse> {
    clusters.into_iter().map(|cluster| cluster_to_service(&cluster)).collect()
}

/// Create metadata for cross-API tracking
pub fn create_cross_api_metadata(source: &str, original_name: &str) -> Value {
    json!({
        "source": source,
        "original_name": original_name,
        "created_via": source,
        "managed_by": "flowplane",
        "cross_api_visible": true,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
}

/// Merge Platform API policies into Native API filter configuration
pub fn policies_to_filters(policies: &crate::api::platform_api_definitions::ApiPolicies) -> Value {
    let mut filters = json!({});

    // Rate limiting filter
    if let Some(rate_limit) = &policies.rate_limit {
        filters["envoy.filters.http.ratelimit"] = json!({
            "domain": "flowplane",
            "stage": 0,
            "request_type": "both",
            "timeout": "0.025s",
            "rate_limit_service": {
                "transport_api_version": "V3",
                "grpc_service": {
                    "envoy_grpc": {
                        "cluster_name": "rate_limit_cluster"
                    }
                }
            },
            "descriptors": [{
                "entries": [{
                    "key": "rate_limit",
                    "value": format!("{}/{}", rate_limit.requests, rate_limit.interval)
                }]
            }]
        });
    }

    // CORS filter
    if let Some(cors) = &policies.cors {
        filters["envoy.filters.http.cors"] = json!({
            "allow_origin_string_match": cors.origins.iter().map(|o| {
                json!({"exact": o})
            }).collect::<Vec<_>>(),
            "allow_methods": cors.methods.join(", "),
            "allow_headers": cors.headers.join(", "),
            "allow_credentials": cors.allow_credentials,
            "max_age": cors.max_age.map(|age| age.to_string()),
        });
    }

    // JWT authentication filter
    if let Some(auth) = &policies.authentication {
        if auth.auth_type == "jwt" {
            filters["envoy.filters.http.jwt_authn"] = json!({
                "providers": {
                    "provider": auth.config.clone().unwrap_or_else(|| json!({}))
                },
                "rules": [{
                    "match": {"prefix": "/"},
                    "requires": if auth.required {
                        json!({"provider_name": "provider"})
                    } else {
                        json!({})
                    }
                }]
            });
        }
    }

    filters
}

// Helper function to generate a simple ID from a name
fn generate_id_from_name(name: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash % 1000000)
}
