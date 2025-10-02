use axum::Router;
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

#[allow(unused_imports)]
use crate::api::auth_handlers::{CreateTokenBody, UpdateTokenBody};
#[allow(unused_imports)]
use crate::api::handlers::{
    CircuitBreakerThresholdsRequest, CircuitBreakersRequest, ClusterResponse, CreateClusterBody,
    EndpointRequest, HealthCheckRequest, OutlierDetectionRequest,
};
#[allow(unused_imports)]
use crate::auth::{models::PersonalAccessToken, token_service::TokenSecretResponse};
#[allow(unused_imports)]
use crate::xds::{
    CircuitBreakerThresholdsSpec, CircuitBreakersSpec, ClusterSpec, EndpointSpec, HealthCheckSpec,
    OutlierDetectionSpec,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::auth_handlers::create_token_handler,
        crate::api::auth_handlers::list_tokens_handler,
        crate::api::auth_handlers::get_token_handler,
        crate::api::auth_handlers::update_token_handler,
        crate::api::auth_handlers::revoke_token_handler,
        crate::api::auth_handlers::rotate_token_handler,
        crate::api::handlers::create_cluster_handler,
        crate::api::handlers::list_clusters_handler,
        crate::api::handlers::get_cluster_handler,
        crate::api::handlers::update_cluster_handler,
        crate::api::handlers::delete_cluster_handler,
        crate::api::route_handlers::create_route_handler,
        crate::api::route_handlers::list_routes_handler,
        crate::api::route_handlers::get_route_handler,
        crate::api::route_handlers::update_route_handler,
        crate::api::route_handlers::delete_route_handler,
        crate::api::listener_handlers::create_listener_handler,
        crate::api::listener_handlers::list_listeners_handler,
        crate::api::listener_handlers::get_listener_handler,
        crate::api::listener_handlers::update_listener_handler,
        crate::api::listener_handlers::delete_listener_handler,
        crate::api::gateway_handlers::create_gateway_from_openapi_handler,
        crate::api::platform_api_definitions::create_api_definition_handler,
        crate::api::platform_api_definitions::list_api_definitions_handler,
        crate::api::platform_api_definitions::get_api_definition_by_id_handler,
        crate::api::platform_api_definitions::update_api_definition_handler,
        crate::api::platform_api_definitions::delete_api_definition_handler,
        crate::api::platform_service_handlers::create_service_handler,
        crate::api::platform_service_handlers::list_services_handler,
        crate::api::platform_service_handlers::get_service_handler,
        crate::api::platform_service_handlers::update_service_handler,
        crate::api::platform_service_handlers::delete_service_handler,
        crate::api::platform_openapi_handlers::import_openapi_handler
    ),
    components(
        schemas(
            CreateClusterBody,
            EndpointRequest,
            HealthCheckRequest,
            CircuitBreakersRequest,
            CircuitBreakerThresholdsRequest,
            OutlierDetectionRequest,
            ClusterResponse,
            CreateTokenBody,
            UpdateTokenBody,
            PersonalAccessToken,
            TokenSecretResponse,
            ClusterSpec,
            EndpointSpec,
            CircuitBreakersSpec,
            CircuitBreakerThresholdsSpec,
            HealthCheckSpec,
            OutlierDetectionSpec,
            crate::api::route_handlers::RouteDefinition,
            crate::api::route_handlers::VirtualHostDefinition,
            crate::api::route_handlers::RouteRuleDefinition,
            crate::api::route_handlers::RouteMatchDefinition,
            crate::api::route_handlers::PathMatchDefinition,
            crate::api::route_handlers::RouteActionDefinition,
            crate::api::route_handlers::WeightedClusterDefinition,
            crate::api::route_handlers::RouteResponse,
            crate::api::listener_handlers::ListenerResponse,
            crate::api::listener_handlers::CreateListenerBody,
            crate::api::listener_handlers::UpdateListenerBody,
            crate::api::gateway_handlers::GatewayQuery,
            crate::api::gateway_handlers::OpenApiSpecBody,
            crate::openapi::GatewaySummary,
            crate::api::platform_api_definitions::ApiDefinition,
            crate::api::platform_api_definitions::UpstreamConfig,
            crate::api::platform_api_definitions::UpstreamEndpoint,
            crate::api::platform_api_definitions::ApiRoute,
            crate::api::platform_api_definitions::ApiPolicies,
            crate::api::platform_api_definitions::RateLimitPolicy,
            crate::api::platform_api_definitions::AuthenticationPolicy,
            crate::api::platform_api_definitions::AuthorizationPolicy,
            crate::api::platform_api_definitions::CorsPolicy,
            crate::api::platform_api_definitions::CircuitBreakerPolicy,
            crate::api::platform_api_definitions::RetryPolicy,
            crate::api::platform_api_definitions::TimeoutPolicy,
            crate::api::platform_api_definitions::ApiDefinitionResponse,
            crate::api::platform_api_definitions::ListApisQuery,
            crate::api::platform_service_handlers::ServiceDefinition,
            crate::api::platform_service_handlers::ServiceEndpoint,
            crate::api::platform_service_handlers::ServiceHealthCheck,
            crate::api::platform_service_handlers::ServiceCircuitBreaker,
            crate::api::platform_service_handlers::ServiceOutlierDetection,
            crate::api::platform_service_handlers::ServiceResponse,
            crate::api::platform_service_handlers::LoadBalancingStrategy,
            crate::api::platform_openapi_handlers::OpenApiImportQuery
        )
    ),
    tags(
        (name = "tokens", description = "Personal access token management"),
        (name = "clusters", description = "Native API - Envoy cluster management"),
        (name = "route-configs", description = "Native API - Envoy route configuration management"),
        (name = "listeners", description = "Native API - Envoy listener management"),
        (name = "platform-apis", description = "Platform API - API gateway definitions"),
        (name = "platform-services", description = "Platform API - Backend service definitions"),
        (name = "platform-import", description = "Platform API - OpenAPI specification import"),
        (name = "gateways", description = "Legacy - Gateway import endpoints (deprecated)")
    ),
    security(
        ("bearerAuth" = [])
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
    }
}

pub fn docs_router() -> Router {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::openapi::{schema::Schema, RefOr};

    #[test]
    fn openapi_includes_cluster_contract() {
        let openapi = ApiDoc::openapi();

        // Validate schema requirements.
        let schemas = openapi.components.as_ref().expect("components").schemas.clone();

        let request_schema = schemas.get("CreateClusterBody").expect("CreateClusterBody schema");
        let request_object = match request_schema {
            RefOr::T(Schema::Object(obj)) => obj,
            RefOr::T(_) => panic!("expected object schema"),
            RefOr::Ref(_) => panic!("expected inline schema, found ref"),
        };

        let required = request_object.required.clone();
        assert!(required.contains(&"name".to_string()));
        assert!(required.contains(&"endpoints".to_string()));
        assert!(!required.contains(&"serviceName".to_string()));

        // Ensure Native API endpoints are documented.
        assert!(openapi.paths.paths.contains_key("/api/v1/clusters"));
        assert!(openapi.paths.paths.contains_key("/api/v1/clusters/{name}"));
        assert!(openapi.paths.paths.contains_key("/api/v1/route-configs"));
        assert!(openapi.paths.paths.contains_key("/api/v1/route-configs/{name}"));
        assert!(openapi.paths.paths.contains_key("/api/v1/listeners"));
        assert!(openapi.paths.paths.contains_key("/api/v1/listeners/{name}"));

        // Ensure Platform API endpoints are documented.
        assert!(openapi.paths.paths.contains_key("/api/v1/platform/apis"));
        assert!(openapi.paths.paths.contains_key("/api/v1/platform/apis/{id}"));
        assert!(openapi.paths.paths.contains_key("/api/v1/platform/services"));
        assert!(openapi.paths.paths.contains_key("/api/v1/platform/services/{name}"));
        assert!(openapi.paths.paths.contains_key("/api/v1/platform/import/openapi"));

        // Ensure token endpoints are documented.
        assert!(openapi.paths.paths.contains_key("/api/v1/tokens"));
    }
}
