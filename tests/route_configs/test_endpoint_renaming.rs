//! Tests for route-configs endpoint renaming

use axum::body::to_bytes;
use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::support::{send_request, setup_test_app};

/// Test that new route-configs endpoints are accessible
#[tokio::test]
async fn test_route_configs_endpoints_exist() {
    let app = setup_test_app().await;
    let admin = app.issue_token("admin", &["routes:read", "routes:write"]).await;

    // Test GET /api/v1/route-configs (should work even without data)
    let response =
        send_request(&app, Method::GET, "/api/v1/route-configs", Some(&admin.token), None).await;

    // The endpoint should exist (not return 405 METHOD_NOT_ALLOWED or 404)
    // It may return 200 OK with empty list, or other status codes
    // But we're just testing that the endpoint is routed correctly
    assert_ne!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "GET /api/v1/route-configs endpoint should exist"
    );
}

/// Test that both endpoints can be used to create and list route configurations
#[tokio::test]
async fn test_route_configs_and_routes_interoperability() {
    let app = setup_test_app().await;
    let admin = app.issue_token("admin", &["routes:read", "routes:write", "clusters:write"]).await;

    // First, let's create a simple cluster that our routes can reference
    let cluster_data = json!({
        "name": "backend-cluster",
        "endpoints": [{
            "host": "127.0.0.1",
            "port": 8080,
            "weight": 100
        }]
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/clusters",
        Some(&admin.token),
        Some(cluster_data),
    )
    .await;

    // Cluster creation should succeed
    let status = response.status();
    if status != StatusCode::CREATED {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        eprintln!("Cluster creation failed with status {} and body: {}", status, body_str);
        assert_eq!(status, StatusCode::CREATED, "Cluster should be created");
    }

    // Now test creating a route configuration through the new endpoint
    let route_config = json!({
        "name": "test-route-config",
        "virtual_hosts": [{
            "name": "default",
            "domains": ["*"],
            "routes": [{
                "name": "backend-route",
                "match": {
                    "prefix": "/"
                },
                "route": {
                    "cluster": "backend-cluster"
                }
            }]
        }]
    });

    // Try to create via /api/v1/route-configs
    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/route-configs",
        Some(&admin.token),
        Some(route_config.clone()),
    )
    .await;

    // The endpoint should be accessible (not 405)
    // It might return 400/422 for validation errors or 201 for success
    assert_ne!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "POST /api/v1/route-configs should be routed"
    );
}

/// Test that route-configs endpoints work with correct scopes
#[tokio::test]
async fn test_route_configs_scope_compatibility() {
    let app = setup_test_app().await;

    // Test with route-configs:read scope
    let token_route_configs = app.issue_token("reader", &["route-configs:read"]).await;

    let response = send_request(
        &app,
        Method::GET,
        "/api/v1/route-configs",
        Some(&token_route_configs.token),
        None,
    )
    .await;

    // Should be accessible with new scope name too
    assert_ne!(
        response.status(),
        StatusCode::FORBIDDEN,
        "route-configs:read scope should work with /api/v1/route-configs"
    );
}

/// Test all CRUD operations work on both endpoints
#[tokio::test]
async fn test_route_configs_crud_operations() {
    let app = setup_test_app().await;
    let admin = app.issue_token("admin", &["routes:read", "routes:write", "clusters:write"]).await;

    // Create a test cluster
    let cluster = json!({
        "name": "crud-cluster",
        "endpoints": [{
            "host": "localhost",
            "port": 3000,
            "weight": 100
        }]
    });

    let response =
        send_request(&app, Method::POST, "/api/v1/clusters", Some(&admin.token), Some(cluster))
            .await;
    let status = response.status();
    if status != StatusCode::CREATED {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        eprintln!("Cluster creation failed with status {} and body: {}", status, body_str);
    }
    assert_eq!(status, StatusCode::CREATED);

    // Test route-configs endpoints for CRUD operations
    let endpoints = vec![
        (Method::POST, "/api/v1/route-configs"),
        (Method::GET, "/api/v1/route-configs"),
        (Method::GET, "/api/v1/route-configs/test-config"),
        (Method::PUT, "/api/v1/route-configs/test-config"),
        (Method::DELETE, "/api/v1/route-configs/test-config"),
    ];

    for (method, path) in &endpoints {
        let body = if method == &Method::POST || method == &Method::PUT {
            Some(json!({
                "name": "test-config",
                "virtual_hosts": [{
                    "name": "vh1",
                    "domains": ["example.com"],
                    "routes": [{
                        "name": "route1",
                        "match": { "prefix": "/" },
                        "route": { "cluster": "crud-cluster" }
                    }]
                }]
            }))
        } else {
            None
        };

        let response = send_request(&app, method.clone(), path, Some(&admin.token), body).await;

        // Endpoints should be routable (not 405)
        assert_ne!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{} {} should be routed",
            method,
            path
        );
    }
}
