use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};

use crate::auth::{
    auth_service::AuthService,
    middleware::{authenticate, ensure_scopes, ScopeState},
};
use crate::storage::repository_simple::AuditLogRepository;
use crate::xds::XdsState;

use super::{
    auth_handlers::{
        create_token_handler, get_token_handler, list_tokens_handler, revoke_token_handler,
        rotate_token_handler, update_token_handler,
    },
    docs,
    handlers::{
        create_cluster_handler, delete_cluster_handler, get_cluster_handler, list_clusters_handler,
        update_cluster_handler,
    },
    listener_handlers::{
        create_listener_handler, delete_listener_handler, get_listener_handler,
        list_listeners_handler, update_listener_handler,
    },
    platform_api_definitions::{
        create_api_definition_handler, delete_api_definition_handler,
        get_api_definition_by_id_handler, list_api_definitions_handler,
        update_api_definition_handler,
    },
    platform_openapi_handlers::{import_openapi_handler, redirect_gateway_import_handler},
    platform_service_handlers::{
        create_service_handler, delete_service_handler, get_service_handler, list_services_handler,
        update_service_handler,
    },
    route_handlers::{
        create_route_handler, delete_route_handler, get_route_handler, list_routes_handler,
        update_route_handler,
    },
};

#[derive(Clone)]
pub struct ApiState {
    pub xds_state: Arc<XdsState>,
}

pub fn build_router(state: Arc<XdsState>) -> Router {
    let api_state = ApiState { xds_state: state.clone() };

    let cluster_repo = match &state.cluster_repository {
        Some(repo) => repo.clone(),
        None => return docs::docs_router(),
    };

    let auth_layer = {
        let pool = cluster_repo.pool().clone();
        let audit_repository = Arc::new(AuditLogRepository::new(pool.clone()));
        let auth_service = Arc::new(AuthService::with_sqlx(pool, audit_repository));
        middleware::from_fn_with_state(auth_service, authenticate)
    };

    let scope_layer = |scopes: Vec<&str>| {
        let required: ScopeState =
            Arc::new(scopes.into_iter().map(|scope| scope.to_string()).collect());
        middleware::from_fn_with_state(required, ensure_scopes)
    };

    let secured_api = Router::new()
        .merge(
            Router::new()
                .route("/api/v1/tokens", get(list_tokens_handler))
                .route_layer(scope_layer(vec!["tokens:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/tokens", post(create_token_handler))
                .route_layer(scope_layer(vec!["tokens:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/tokens/{id}", get(get_token_handler))
                .route_layer(scope_layer(vec!["tokens:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/tokens/{id}", patch(update_token_handler))
                .route_layer(scope_layer(vec!["tokens:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/tokens/{id}", delete(revoke_token_handler))
                .route_layer(scope_layer(vec!["tokens:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/tokens/{id}/rotate", post(rotate_token_handler))
                .route_layer(scope_layer(vec!["tokens:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/clusters", get(list_clusters_handler))
                .route_layer(scope_layer(vec!["clusters:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/clusters", post(create_cluster_handler))
                .route_layer(scope_layer(vec!["clusters:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/clusters/{name}", get(get_cluster_handler))
                .route_layer(scope_layer(vec!["clusters:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/clusters/{name}", put(update_cluster_handler))
                .route_layer(scope_layer(vec!["clusters:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/clusters/{name}", delete(delete_cluster_handler))
                .route_layer(scope_layer(vec!["clusters:write"])),
        )
        // Route-configs endpoints (aligned with Envoy)
        .merge(
            Router::new()
                .route("/api/v1/route-configs", get(list_routes_handler))
                .route_layer(scope_layer(vec!["route-configs:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/route-configs", post(create_route_handler))
                .route_layer(scope_layer(vec!["route-configs:write"])),
        )
        // Route-configs endpoints by name
        .merge(
            Router::new()
                .route("/api/v1/route-configs/{name}", get(get_route_handler))
                .route_layer(scope_layer(vec!["route-configs:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/route-configs/{name}", put(update_route_handler))
                .route_layer(scope_layer(vec!["route-configs:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/route-configs/{name}", delete(delete_route_handler))
                .route_layer(scope_layer(vec!["route-configs:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/listeners", get(list_listeners_handler))
                .route_layer(scope_layer(vec!["listeners:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/listeners", post(create_listener_handler))
                .route_layer(scope_layer(vec!["listeners:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/listeners/{name}", get(get_listener_handler))
                .route_layer(scope_layer(vec!["listeners:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/listeners/{name}", put(update_listener_handler))
                .route_layer(scope_layer(vec!["listeners:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/listeners/{name}", delete(delete_listener_handler))
                .route_layer(scope_layer(vec!["listeners:write"])),
        )
        // Platform API definitions endpoints
        .merge(
            Router::new()
                .route("/api/v1/platform/apis", get(list_api_definitions_handler))
                .route_layer(scope_layer(vec!["apis:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/apis", post(create_api_definition_handler))
                .route_layer(scope_layer(vec![
                    "apis:write",
                    "route-configs:write",
                    "listeners:write",
                    "clusters:write",
                ])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/apis/{id}", get(get_api_definition_by_id_handler))
                .route_layer(scope_layer(vec!["apis:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/apis/{id}", put(update_api_definition_handler))
                .route_layer(scope_layer(vec![
                    "apis:write",
                    "route-configs:write",
                    "listeners:write",
                    "clusters:write",
                ])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/apis/{id}", delete(delete_api_definition_handler))
                .route_layer(scope_layer(vec![
                    "apis:write",
                    "route-configs:write",
                    "listeners:write",
                    "clusters:write",
                ])),
        )
        // Platform API OpenAPI import endpoint
        .merge(
            Router::new()
                .route("/api/v1/platform/import/openapi", post(import_openapi_handler))
                .route_layer(scope_layer(vec!["apis:write", "import:write"])),
        )
        // Redirect from old gateway endpoint
        .merge(
            Router::new()
                .route("/api/v1/gateways/openapi", post(redirect_gateway_import_handler))
                .route_layer(scope_layer(vec!["gateways:import"])),
        )
        // Platform API service endpoints
        .merge(
            Router::new()
                .route("/api/v1/platform/services", get(list_services_handler))
                .route_layer(scope_layer(vec!["services:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/services", post(create_service_handler))
                .route_layer(scope_layer(vec!["services:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/services/{name}", get(get_service_handler))
                .route_layer(scope_layer(vec!["services:read"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/services/{name}", put(update_service_handler))
                .route_layer(scope_layer(vec!["services:write"])),
        )
        .merge(
            Router::new()
                .route("/api/v1/platform/services/{name}", delete(delete_service_handler))
                .route_layer(scope_layer(vec!["services:write"])),
        )
        .with_state(api_state)
        .layer(auth_layer);

    secured_api.merge(docs::docs_router())
}
