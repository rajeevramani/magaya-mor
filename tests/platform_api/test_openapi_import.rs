//! Tests for Platform API OpenAPI import functionality

use axum::http::{Method, StatusCode};

use super::support::{read_json, send_request_with_body, setup_platform_api_app};

#[tokio::test]
async fn test_openapi_import_at_platform_endpoint() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["apis:write", "import:write"]).await;

    let openapi_spec = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
servers:
  - url: https://api.example.com
paths:
  /users:
    get:
      summary: List users
      responses:
        '200':
          description: Success
    post:
      summary: Create user
      responses:
        '201':
          description: Created
  /users/{id}:
    get:
      summary: Get user
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Success
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/platform/import/openapi?name=test-api",
        Some(&token.token),
        openapi_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED, "OpenAPI import should succeed");
    let body: serde_json::Value = read_json(response).await;

    assert!(body.get("id").is_some(), "Should return API definition ID");
    assert_eq!(body.get("name").unwrap(), "test-api");
    assert!(body.get("routes").unwrap().as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn test_openapi_import_with_flowplane_tags() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["apis:write", "import:write"]).await;

    let openapi_spec = r#"
openapi: 3.0.0
info:
  title: Test API with Flowplane Tags
  version: 1.0.0
servers:
  - url: https://api.example.com
paths:
  /protected:
    get:
      summary: Protected endpoint
      x-flowplane-ratelimit:
        requests: 100
        interval: "1m"
      x-flowplane-jwt-auth:
        required: true
        issuer: "https://auth.example.com"
      x-flowplane-cors:
        origins: ["https://app.example.com"]
        methods: ["GET", "POST"]
      responses:
        '200':
          description: Success
  /public:
    get:
      summary: Public endpoint
      x-flowplane-cors:
        origins: ["*"]
        methods: ["GET"]
      responses:
        '200':
          description: Success
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/platform/import/openapi?name=tagged-api",
        Some(&token.token),
        openapi_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED, "Import with tags should succeed");
    let body: serde_json::Value = read_json(response).await;

    // Verify policies were extracted from x-flowplane tags
    let policies = body.get("policies").unwrap();
    assert!(policies.get("rateLimit").is_some(), "Should extract rate limit policy");
    assert!(policies.get("authentication").is_some(), "Should extract auth policy");
    assert!(policies.get("cors").is_some(), "Should extract CORS policy");
}

#[tokio::test]
async fn test_openapi_import_validation() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["apis:write", "import:write"]).await;

    // Invalid OpenAPI spec
    let invalid_spec = r#"
not: a valid openapi spec
invalid: yaml
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/platform/import/openapi?name=invalid",
        Some(&token.token),
        invalid_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST, "Should reject invalid OpenAPI spec");
}

#[tokio::test]
async fn test_openapi_import_with_invalid_filter_tags() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["apis:write", "import:write"]).await;

    let openapi_spec = r#"
openapi: 3.0.0
info:
  title: API with Invalid Tags
  version: 1.0.0
paths:
  /test:
    get:
      x-flowplane-invalid-filter:
        unknown: "value"
      responses:
        '200':
          description: Success
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/platform/import/openapi?name=invalid-tags",
        Some(&token.token),
        openapi_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    // Should still create but log warning about unknown filter
    assert_eq!(response.status(), StatusCode::CREATED, "Should create API even with unknown tags");

    let body: serde_json::Value = read_json(response).await;
    assert!(body.get("warnings").is_some(), "Should include warnings about unknown tags");
}

#[tokio::test]
async fn test_openapi_import_authorization() {
    let app = setup_platform_api_app().await;
    let read_token = app.issue_token("reader", &["apis:read"]).await;

    let openapi_spec = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: Success
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/platform/import/openapi?name=unauthorized",
        Some(&read_token.token),
        openapi_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN, "Should require import:write scope");
}

#[tokio::test]
async fn test_redirect_from_old_gateway_endpoint() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["gateways:import"]).await;

    let openapi_spec = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: Success
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/gateways/openapi?name=legacy",
        Some(&token.token),
        openapi_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    // Should redirect to new platform endpoint
    assert!(
        response.status() == StatusCode::MOVED_PERMANENTLY
            || response.status() == StatusCode::PERMANENT_REDIRECT,
        "Old endpoint should redirect to new location"
    );

    let location = response.headers().get("location");
    assert!(location.is_some(), "Should include Location header");
    assert!(
        location.unwrap().to_str().unwrap().contains("/api/v1/platform/import/openapi"),
        "Should redirect to platform import endpoint"
    );
}

#[tokio::test]
async fn test_openapi_import_response_format() {
    let app = setup_platform_api_app().await;
    let token = app.issue_token("admin", &["apis:write", "import:write"]).await;

    let openapi_spec = r#"
openapi: 3.0.0
info:
  title: Complete API
  version: 2.0.0
  description: API with complete information
  contact:
    email: api@example.com
servers:
  - url: https://api.example.com
    description: Production server
paths:
  /items:
    get:
      summary: List items
      operationId: listItems
      tags:
        - Items
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: array
                items:
                  type: object
"#;

    let response = send_request_with_body(
        &app,
        Method::POST,
        "/api/v1/platform/import/openapi?name=complete-api",
        Some(&token.token),
        openapi_spec.as_bytes().to_vec(),
        "application/yaml",
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value = read_json(response).await;

    // Verify new response format includes all expected fields
    assert_eq!(body.get("name").unwrap(), "complete-api");
    assert_eq!(body.get("version").unwrap(), "2.0.0");
    assert!(body.get("id").is_some(), "Should have unique ID");
    assert!(body.get("basePath").is_some(), "Should extract base path");
    assert!(body.get("upstream").is_some(), "Should create upstream config");
    assert!(body.get("routes").is_some(), "Should create routes");
    assert!(body.get("createdAt").is_some(), "Should include creation timestamp");
    assert!(body.get("metadata").is_some(), "Should preserve OpenAPI metadata");
}
