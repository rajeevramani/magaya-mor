//! Tests for Platform API service abstraction endpoints

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::support::{read_json, send_request, setup_platform_api_app};

#[tokio::test]
async fn test_create_service_transforms_to_cluster() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["services:read", "services:write", "clusters:read", "clusters:write"],
        )
        .await;

    // Create a service using Platform API abstraction
    let service_payload = json!({
        "name": "payment-service",
        "endpoints": [
            {
                "host": "payment-1.internal",
                "port": 8080,
                "weight": 50
            },
            {
                "host": "payment-2.internal",
                "port": 8080,
                "weight": 50
            }
        ],
        "loadBalancing": "round_robin",
        "healthCheck": {
            "path": "/health",
            "interval": 10,
            "timeout": 5,
            "healthyThreshold": 2,
            "unhealthyThreshold": 3
        },
        "circuitBreaker": {
            "maxRequests": 100,
            "intervalMs": 10000,
            "consecutiveErrors": 5
        }
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED, "Service should be created");
    let body: serde_json::Value = read_json(response).await;

    // Should return service representation
    assert_eq!(body.get("name").unwrap(), "payment-service");
    assert!(body.get("clusterId").is_some(), "Should have underlying cluster ID");

    // Verify cluster was created in Native API
    let cluster_response = send_request(
        &app,
        Method::GET,
        "/api/v1/clusters/payment-service",
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(cluster_response.status(), StatusCode::OK, "Cluster should exist");
}

#[tokio::test]
async fn test_list_services_shows_platform_view() {
    let app = setup_platform_api_app().await;
    let token =
        app.issue_token("admin", &["services:read", "services:write", "clusters:write"]).await;

    // Create services
    let service1 = json!({
        "name": "auth-service",
        "endpoints": [{"host": "auth.internal", "port": 9000, "weight": 100}],
        "loadBalancing": "round_robin"
    });

    let service2 = json!({
        "name": "user-service",
        "endpoints": [{"host": "user.internal", "port": 9001, "weight": 100}],
        "loadBalancing": "least_request"
    });

    send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service1),
    )
    .await;

    send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service2),
    )
    .await;

    // List services
    let response =
        send_request(&app, Method::GET, "/api/v1/platform/services", Some(&token.token), None)
            .await;

    assert_eq!(response.status(), StatusCode::OK);
    let services: Vec<serde_json::Value> = read_json(response).await;

    assert!(services.len() >= 2, "Should have at least 2 services");

    let service_names: Vec<String> = services
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect();

    assert!(service_names.contains(&"auth-service".to_string()));
    assert!(service_names.contains(&"user-service".to_string()));
}

#[tokio::test]
async fn test_get_service_by_name() {
    let app = setup_platform_api_app().await;
    let token =
        app.issue_token("admin", &["services:read", "services:write", "clusters:write"]).await;

    // Create a service
    let service_payload = json!({
        "name": "inventory-service",
        "endpoints": [{"host": "inventory.internal", "port": 8080, "weight": 100}],
        "loadBalancing": "round_robin",
        "healthCheck": {
            "path": "/healthz",
            "interval": 5
        }
    });

    send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    // Get service by name
    let response = send_request(
        &app,
        Method::GET,
        "/api/v1/platform/services/inventory-service",
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let service: serde_json::Value = read_json(response).await;

    assert_eq!(service.get("name").unwrap(), "inventory-service");
    assert!(service.get("healthCheck").is_some(), "Should include health check config");
}

#[tokio::test]
async fn test_update_service() {
    let app = setup_platform_api_app().await;
    let token =
        app.issue_token("admin", &["services:read", "services:write", "clusters:write"]).await;

    // Create initial service
    let service_payload = json!({
        "name": "cache-service",
        "endpoints": [{"host": "cache.internal", "port": 6379, "weight": 100}],
        "loadBalancing": "round_robin"
    });

    send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    // Update service
    let updated_payload = json!({
        "name": "cache-service",
        "endpoints": [
            {"host": "cache-1.internal", "port": 6379, "weight": 60},
            {"host": "cache-2.internal", "port": 6379, "weight": 40}
        ],
        "loadBalancing": "least_request",
        "circuitBreaker": {
            "maxRequests": 50
        }
    });

    let response = send_request(
        &app,
        Method::PUT,
        "/api/v1/platform/services/cache-service",
        Some(&token.token),
        Some(updated_payload),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK, "Service should be updated");
    let service: serde_json::Value = read_json(response).await;

    let endpoints = service.get("endpoints").unwrap().as_array().unwrap();
    assert_eq!(endpoints.len(), 2, "Should have 2 endpoints after update");
    assert_eq!(service.get("loadBalancing").unwrap(), "least_request");
}

#[tokio::test]
async fn test_delete_service() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["services:read", "services:write", "clusters:read", "clusters:write"],
        )
        .await;

    // Create a service
    let service_payload = json!({
        "name": "temp-service",
        "endpoints": [{"host": "temp.internal", "port": 8080, "weight": 100}],
        "loadBalancing": "round_robin"
    });

    send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(service_payload),
    )
    .await;

    // Delete the service
    let response = send_request(
        &app,
        Method::DELETE,
        "/api/v1/platform/services/temp-service",
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT, "Service should be deleted");

    // Verify it's gone
    let get_response = send_request(
        &app,
        Method::GET,
        "/api/v1/platform/services/temp-service",
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND, "Service should not exist");
}

#[tokio::test]
async fn test_service_validation() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["services:read", "services:write", "clusters:read", "clusters:write"],
        )
        .await;

    // Invalid service - no endpoints
    let invalid_payload = json!({
        "name": "invalid-service",
        "endpoints": [],
        "loadBalancing": "round_robin"
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&token.token),
        Some(invalid_payload),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject service without endpoints"
    );
}

#[tokio::test]
async fn test_service_authorization() {
    let app = setup_platform_api_app().await;

    // Token without services:write scope
    let read_only_token = app.issue_token("reader", &["services:read"]).await;

    let service_payload = json!({
        "name": "protected-service",
        "endpoints": [{"host": "protected.internal", "port": 8080, "weight": 100}],
        "loadBalancing": "round_robin"
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/services",
        Some(&read_only_token.token),
        Some(service_payload),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Should not allow creation without write scope"
    );
}
