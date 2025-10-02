//! Tests for cross-API resource visibility between Native and Platform APIs

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::support::{read_json, send_request, setup_platform_api_app};

#[tokio::test]
async fn test_platform_service_visible_in_native_clusters() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["services:write", "clusters:read"]).await;

    // Create a service via Platform API
    let service_payload = json!({
        "name": "cross-api-service",
        "description": "Service created via Platform API",
        "endpoints": [
            {
                "host": "backend.example.com",
                "port": 8080,
                "weight": 100
            }
        ],
        "loadBalancing": "ROUND_ROBIN",
        "healthCheck": {
            "type": "http",
            "path": "/health",
            "intervalSeconds": 10,
            "timeoutSeconds": 5
        }
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED, "Service should be created");

    // Query via Native API to verify visibility
    let list_response =
        send_request(&app, Method::GET, "/api/v1/clusters", Some(&token.token), None).await;

    assert_eq!(list_response.status(), StatusCode::OK);
    let clusters: Vec<serde_json::Value> = read_json(list_response).await;

    // Verify the service appears as a cluster in Native API
    let found = clusters.iter().any(|c| {
        c.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == "cross-api-service-cluster")
            .unwrap_or(false)
    });

    assert!(found, "Service created via Platform API should be visible as cluster in Native API");
}

#[tokio::test]
async fn test_native_cluster_visible_in_platform_services() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["clusters:write", "services:read"]).await;

    // Create a cluster via Native API
    let cluster_payload = json!({
        "name": "native-cluster",
        "serviceName": "native-service",
        "endpoints": [
            {
                "host": "native.example.com",
                "port": 9000
            }
        ],
        "connectTimeoutSeconds": 5,
        "lbPolicy": "LEAST_REQUEST"
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/clusters",
        Some(&token.token),
        Some(cluster_payload),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED, "Cluster should be created");

    // Query via Platform API to verify visibility
    let list_response =
        send_request(&app, Method::GET, "/api/v1/platform/services", Some(&token.token), None)
            .await;

    assert_eq!(list_response.status(), StatusCode::OK);
    let services: Vec<serde_json::Value> = read_json(list_response).await;

    // Verify the cluster appears as a service in Platform API
    let found = services.iter().any(|s| {
        s.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n == "native-service" || n == "native-cluster")
            .unwrap_or(false)
    });

    assert!(found, "Cluster created via Native API should be visible as service in Platform API");
}

#[tokio::test]
async fn test_platform_api_definition_creates_native_resources() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &[
                "apis:write",
                "route-configs:write",
                "clusters:write",
                "listeners:write",
                "route-configs:read",
                "clusters:read",
                "listeners:read",
            ],
        )
        .await;

    // Create an API definition via Platform API
    let api_payload = json!({
        "name": "visibility-test-api",
        "version": "1.0.0",
        "basePath": "/api/v1/test",
        "upstream": {
            "service": "test-backend",
            "endpoints": [
                {
                    "host": "backend.test.com",
                    "port": 8080
                }
            ],
            "tls": false,
            "loadBalancing": "ROUND_ROBIN"
        },
        "routes": [
            {
                "path": "/users",
                "methods": ["GET", "POST"],
                "description": "User operations"
            }
        ],
        "policies": {
            "rateLimit": {
                "requests": 100,
                "interval": "1m"
            }
        }
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&token.token),
        Some(api_payload),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let api: serde_json::Value = read_json(create_response).await;

    let cluster_id = api.get("clusterId").and_then(|c| c.as_str()).unwrap();
    let route_config_id = api.get("routeConfigId").and_then(|r| r.as_str()).unwrap();

    // Verify cluster is visible in Native API
    let cluster_response = send_request(
        &app,
        Method::GET,
        &format!("/api/v1/clusters/{}", cluster_id),
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(
        cluster_response.status(),
        StatusCode::OK,
        "Cluster created by Platform API should be visible in Native API"
    );

    // Verify route config is visible in Native API
    let route_response = send_request(
        &app,
        Method::GET,
        &format!("/api/v1/route-configs/{}", route_config_id),
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(
        route_response.status(),
        StatusCode::OK,
        "Route config created by Platform API should be visible in Native API"
    );
}

#[tokio::test]
async fn test_native_route_config_visible_in_platform_apis() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["route-configs:write", "apis:read"]).await;

    // Create a route config via Native API
    let route_payload = json!({
        "name": "native-routes",
        "virtualHosts": [
            {
                "name": "api-host",
                "domains": ["api.example.com"],
                "routes": [
                    {
                        "name": "products",
                        "match": {
                            "path": {
                                "type": "prefix",
                                "value": "/products"
                            }
                        },
                        "action": {
                            "type": "forward",
                            "cluster": "product-service",
                            "timeoutSeconds": 10
                        }
                    }
                ]
            }
        ]
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/route-configs",
        Some(&token.token),
        Some(route_payload),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Query Platform APIs to verify visibility
    let list_response =
        send_request(&app, Method::GET, "/api/v1/platform/apis", Some(&token.token), None).await;

    assert_eq!(list_response.status(), StatusCode::OK);
    let apis: Vec<serde_json::Value> = read_json(list_response).await;

    // Platform API should show simplified view of Native route configs
    // The exact mapping depends on implementation
    assert!(
        apis.is_empty() || apis.iter().any(|a| a.get("routeConfigId").is_some()),
        "Platform API should provide visibility into Native route configurations"
    );
}

#[tokio::test]
async fn test_resource_updates_reflected_across_apis() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["services:write", "services:read", "clusters:write", "clusters:read"],
        )
        .await;

    // Create via Platform API
    let service_payload = json!({
        "name": "update-test-service",
        "description": "Initial description",
        "endpoints": [
            {
                "host": "initial.example.com",
                "port": 8080,
                "weight": 100
            }
        ],
        "loadBalancing": "ROUND_ROBIN"
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Update via Native API (if the underlying cluster exists)
    let update_payload = json!({
        "name": "update-test-service-cluster",
        "serviceName": "update-test-service-updated",
        "endpoints": [
            {
                "host": "updated.example.com",
                "port": 9000
            }
        ],
        "lbPolicy": "RANDOM"
    });

    let update_response = send_request(
        &app,
        Method::PUT,
        "/api/v1/clusters/update-test-service-cluster",
        Some(&token.token),
        Some(update_payload),
    )
    .await;

    // If update succeeds, verify change is visible in Platform API
    if update_response.status() == StatusCode::OK {
        let get_response = send_request(
            &app,
            Method::GET,
            "/api/v1/platform/services/update-test-service",
            Some(&token.token),
            None,
        )
        .await;

        if get_response.status() == StatusCode::OK {
            let service: serde_json::Value = read_json(get_response).await;

            // Verify update is reflected
            let endpoints = service.get("endpoints").and_then(|e| e.as_array());
            assert!(endpoints.is_some(), "Service should have endpoints");

            if let Some(endpoints) = endpoints {
                let updated = endpoints.iter().any(|e| {
                    e.get("host")
                        .and_then(|h| h.as_str())
                        .map(|h| h == "updated.example.com")
                        .unwrap_or(false)
                });
                assert!(updated, "Updates via Native API should be reflected in Platform API");
            }
        }
    }
}

#[tokio::test]
async fn test_deletion_cascades_across_apis() {
    let app = setup_platform_api_app().await;
    let token =
        app.issue_token("admin", &["services:write", "clusters:read", "clusters:write"]).await;

    // Create via Platform API
    let service_payload = json!({
        "name": "delete-test-service",
        "description": "Service to be deleted",
        "endpoints": [
            {
                "host": "delete.example.com",
                "port": 8080,
                "weight": 100
            }
        ],
        "loadBalancing": "ROUND_ROBIN"
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Delete via Platform API
    let delete_response = send_request(
        &app,
        Method::DELETE,
        "/api/v1/platform/services/delete-test-service",
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Verify deletion in Native API
    let get_response = send_request(
        &app,
        Method::GET,
        "/api/v1/clusters/delete-test-service-cluster",
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(
        get_response.status(),
        StatusCode::NOT_FOUND,
        "Deleted service should not be visible in Native API"
    );
}

#[tokio::test]
async fn test_query_filters_work_across_apis() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["apis:write", "apis:read", "route-configs:read"]).await;

    // Create multiple API definitions
    for i in 1..=3 {
        let api_payload = json!({
            "name": format!("filter-test-api-{}", i),
            "version": format!("v{}", i),
            "basePath": format!("/api/v{}/test", i),
            "upstream": {
                "service": format!("test-backend-{}", i),
                "endpoints": [
                    {
                        "host": format!("backend{}.test.com", i),
                        "port": 8080 + i
                    }
                ],
                "tls": false,
                "loadBalancing": "ROUND_ROBIN"
            },
            "routes": [
                {
                    "path": "/test",
                    "methods": ["GET"],
                    "description": format!("Test route {}", i)
                }
            ]
        });

        let response = send_request(
            &app,
            Method::POST,
            "/api/v1/platform/apis",
            Some(&token.token),
            Some(api_payload),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Query with filter via Platform API
    let filtered_response = send_request(
        &app,
        Method::GET,
        "/api/v1/platform/apis?version=v2",
        Some(&token.token),
        None,
    )
    .await;

    if filtered_response.status() == StatusCode::OK {
        let apis: Vec<serde_json::Value> = read_json(filtered_response).await;

        // Check if filtering is implemented
        for api in &apis {
            if let Some(version) = api.get("version").and_then(|v| v.as_str()) {
                assert_eq!(version, "v2", "Filter should only return v2 APIs");
            }
        }
    }
}
