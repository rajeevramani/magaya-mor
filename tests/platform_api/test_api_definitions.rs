//! Tests for Platform API definitions endpoints

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::support::{read_json, send_request, setup_platform_api_app};

#[tokio::test]
async fn test_create_api_definition() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["apis:write", "route-configs:write", "listeners:write", "clusters:write"],
        )
        .await;

    // Create an API definition using Platform API
    let api_payload = json!({
        "name": "users-api",
        "version": "v1",
        "basePath": "/api/v1/users",
        "upstream": {
            "service": "user-service",
            "endpoints": [
                {
                    "host": "user-service.internal",
                    "port": 8080
                }
            ]
        },
        "routes": [
            {
                "path": "/",
                "methods": ["GET", "POST"],
                "description": "List and create users"
            },
            {
                "path": "/{id}",
                "methods": ["GET", "PUT", "DELETE"],
                "description": "User CRUD operations"
            }
        ],
        "policies": {
            "rateLimit": {
                "requests": 100,
                "interval": "1m"
            },
            "authentication": {
                "type": "jwt",
                "required": true
            },
            "cors": {
                "origins": ["https://example.com"],
                "methods": ["GET", "POST", "PUT", "DELETE"],
                "headers": ["Content-Type", "Authorization"]
            }
        }
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&token.token),
        Some(api_payload),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED, "API definition should be created");
    let body: serde_json::Value = read_json(response).await;

    assert_eq!(body.get("name").unwrap(), "users-api");
    assert_eq!(body.get("version").unwrap(), "v1");
    assert!(body.get("id").is_some(), "Should have an ID");
    assert!(body.get("routeConfigId").is_some(), "Should have created route config");
    assert!(body.get("listenerId").is_some(), "Should have created listener");
}

#[tokio::test]
async fn test_list_api_definitions() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &[
                "apis:read",
                "apis:write",
                "route-configs:write",
                "listeners:write",
                "clusters:write",
            ],
        )
        .await;

    // Create multiple API definitions
    let api1 = json!({
        "name": "orders-api",
        "version": "v1",
        "basePath": "/api/v1/orders",
        "upstream": {
            "service": "order-service",
            "endpoints": [{"host": "order-service.internal", "port": 8080}]
        },
        "routes": [
            {"path": "/", "methods": ["GET", "POST"]}
        ]
    });

    let api2 = json!({
        "name": "products-api",
        "version": "v2",
        "basePath": "/api/v2/products",
        "upstream": {
            "service": "product-service",
            "endpoints": [{"host": "product-service.internal", "port": 9000}]
        },
        "routes": [
            {"path": "/", "methods": ["GET"]},
            {"path": "/search", "methods": ["GET"]}
        ]
    });

    send_request(&app, Method::POST, "/api/v1/platform/apis", Some(&token.token), Some(api1)).await;

    send_request(&app, Method::POST, "/api/v1/platform/apis", Some(&token.token), Some(api2)).await;

    // List API definitions
    let response =
        send_request(&app, Method::GET, "/api/v1/platform/apis", Some(&token.token), None).await;

    assert_eq!(response.status(), StatusCode::OK);
    let apis: Vec<serde_json::Value> = read_json(response).await;

    assert!(apis.len() >= 2, "Should have at least 2 API definitions");

    let api_names: Vec<String> = apis
        .iter()
        .filter_map(|a| a.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect();

    assert!(api_names.contains(&"orders-api".to_string()));
    assert!(api_names.contains(&"products-api".to_string()));
}

#[tokio::test]
async fn test_get_api_definition_by_id() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &[
                "apis:read",
                "apis:write",
                "route-configs:write",
                "listeners:write",
                "clusters:write",
            ],
        )
        .await;

    // Create an API definition
    let api_payload = json!({
        "name": "inventory-api",
        "version": "v1",
        "basePath": "/api/v1/inventory",
        "upstream": {
            "service": "inventory-service",
            "endpoints": [{"host": "inventory.internal", "port": 8080}]
        },
        "routes": [
            {"path": "/items", "methods": ["GET", "POST"]},
            {"path": "/items/{id}", "methods": ["GET", "PUT", "DELETE"]}
        ],
        "policies": {
            "rateLimit": {
                "requests": 50,
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

    let created: serde_json::Value = read_json(create_response).await;
    let api_id = created.get("id").unwrap().as_str().unwrap();

    // Get API definition by ID
    let response = send_request(
        &app,
        Method::GET,
        &format!("/api/v1/platform/apis/{}", api_id),
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let api: serde_json::Value = read_json(response).await;

    assert_eq!(api.get("name").unwrap(), "inventory-api");
    assert_eq!(api.get("version").unwrap(), "v1");
    assert!(api.get("policies").is_some(), "Should include policies");
}

#[tokio::test]
async fn test_update_api_definition() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &[
                "apis:read",
                "apis:write",
                "route-configs:write",
                "listeners:write",
                "clusters:write",
            ],
        )
        .await;

    // Create initial API definition
    let api_payload = json!({
        "name": "payment-api",
        "version": "v1",
        "basePath": "/api/v1/payments",
        "upstream": {
            "service": "payment-service",
            "endpoints": [{"host": "payment.internal", "port": 8080}]
        },
        "routes": [
            {"path": "/", "methods": ["GET"]}
        ]
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&token.token),
        Some(api_payload),
    )
    .await;

    let created: serde_json::Value = read_json(create_response).await;
    let api_id = created.get("id").unwrap().as_str().unwrap();

    // Update API definition
    let updated_payload = json!({
        "name": "payment-api",
        "version": "v2",
        "basePath": "/api/v2/payments",
        "upstream": {
            "service": "payment-service",
            "endpoints": [
                {"host": "payment-1.internal", "port": 8080},
                {"host": "payment-2.internal", "port": 8080}
            ]
        },
        "routes": [
            {"path": "/", "methods": ["GET", "POST"]},
            {"path": "/refund", "methods": ["POST"]}
        ],
        "policies": {
            "authentication": {
                "type": "oauth2",
                "required": true
            }
        }
    });

    let response = send_request(
        &app,
        Method::PUT,
        &format!("/api/v1/platform/apis/{}", api_id),
        Some(&token.token),
        Some(updated_payload),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK, "API definition should be updated");
    let api: serde_json::Value = read_json(response).await;

    assert_eq!(api.get("version").unwrap(), "v2");
    assert_eq!(api.get("basePath").unwrap(), "/api/v2/payments");

    let routes = api.get("routes").unwrap().as_array().unwrap();
    assert_eq!(routes.len(), 2, "Should have 2 routes after update");
}

#[tokio::test]
async fn test_delete_api_definition() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &[
                "apis:read",
                "apis:write",
                "route-configs:write",
                "listeners:write",
                "clusters:write",
            ],
        )
        .await;

    // Create an API definition
    let api_payload = json!({
        "name": "temp-api",
        "version": "v1",
        "basePath": "/api/v1/temp",
        "upstream": {
            "service": "temp-service",
            "endpoints": [{"host": "temp.internal", "port": 8080}]
        },
        "routes": [
            {"path": "/", "methods": ["GET"]}
        ]
    });

    let create_response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&token.token),
        Some(api_payload),
    )
    .await;

    let created: serde_json::Value = read_json(create_response).await;
    let api_id = created.get("id").unwrap().as_str().unwrap();

    // Delete the API definition
    let response = send_request(
        &app,
        Method::DELETE,
        &format!("/api/v1/platform/apis/{}", api_id),
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT, "API definition should be deleted");

    // Verify it's gone
    let get_response = send_request(
        &app,
        Method::GET,
        &format!("/api/v1/platform/apis/{}", api_id),
        Some(&token.token),
        None,
    )
    .await;

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND, "API definition should not exist");
}

#[tokio::test]
async fn test_api_definition_validation() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["apis:write", "route-configs:write", "listeners:write", "clusters:write"],
        )
        .await;

    // Invalid API definition - no upstream
    let invalid_payload = json!({
        "name": "invalid-api",
        "version": "v1",
        "basePath": "/api/v1/invalid",
        "routes": [
            {"path": "/", "methods": ["GET"]}
        ]
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&token.token),
        Some(invalid_payload),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Should reject API definition without upstream"
    );
}

#[tokio::test]
async fn test_api_definition_with_complex_policies() {
    let app = setup_platform_api_app().await;
    let token = app
        .issue_token(
            "admin",
            &["apis:write", "route-configs:write", "listeners:write", "clusters:write"],
        )
        .await;

    // API definition with complex policies
    let api_payload = json!({
        "name": "secure-api",
        "version": "v1",
        "basePath": "/api/v1/secure",
        "upstream": {
            "service": "secure-service",
            "endpoints": [{"host": "secure.internal", "port": 8443}],
            "tls": true
        },
        "routes": [
            {
                "path": "/public",
                "methods": ["GET"],
                "policies": {
                    "authentication": {
                        "required": false
                    }
                }
            },
            {
                "path": "/private",
                "methods": ["GET", "POST"],
                "policies": {
                    "authentication": {
                        "required": true
                    },
                    "authorization": {
                        "roles": ["admin", "user"]
                    }
                }
            }
        ],
        "policies": {
            "rateLimit": {
                "requests": 100,
                "interval": "1m",
                "keyBy": "client_ip"
            },
            "circuitBreaker": {
                "maxRequests": 50,
                "intervalMs": 10000,
                "consecutiveErrors": 5
            },
            "retry": {
                "attempts": 3,
                "backoff": "exponential"
            },
            "timeout": {
                "request": 30,
                "idle": 60
            }
        }
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&token.token),
        Some(api_payload),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED, "Should create API with complex policies");
    let body: serde_json::Value = read_json(response).await;

    assert!(body.get("policies").is_some(), "Should preserve complex policies");
    assert_eq!(body.get("name").unwrap(), "secure-api");
}

#[tokio::test]
async fn test_api_definition_authorization() {
    let app = setup_platform_api_app().await;

    // Token without apis:write scope
    let read_only_token = app.issue_token("reader", &["apis:read"]).await;

    let api_payload = json!({
        "name": "protected-api",
        "version": "v1",
        "basePath": "/api/v1/protected",
        "upstream": {
            "service": "protected-service",
            "endpoints": [{"host": "protected.internal", "port": 8080}]
        },
        "routes": [
            {"path": "/", "methods": ["GET"]}
        ]
    });

    let response = send_request(
        &app,
        Method::POST,
        "/api/v1/platform/apis",
        Some(&read_only_token.token),
        Some(api_payload),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Should not allow creation without write scope"
    );
}
