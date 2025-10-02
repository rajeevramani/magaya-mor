//! Axum middleware for authentication and authorization.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Extension, State},
    http::{header::AUTHORIZATION, Method, Request},
    middleware::Next,
    response::Response,
};

use crate::api::error::ApiError;
use crate::auth::auth_service::AuthService;
use crate::auth::models::{AuthContext, AuthError};
use tracing::{field, info_span, warn};

pub type AuthServiceState = Arc<AuthService>;
pub type ScopeState = Arc<Vec<String>>;

/// Middleware entry point that authenticates requests using the configured [`AuthService`].
pub async fn authenticate(
    State(auth_service): State<AuthServiceState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let correlation_id = uuid::Uuid::new_v4();
    let span = info_span!(
        "auth_middleware.authenticate",
        http.method = %method,
        http.path = %path,
        auth.token_id = field::Empty,
        correlation_id = %correlation_id
    );
    let _guard = span.enter();

    let header =
        request.headers().get(AUTHORIZATION).and_then(|value| value.to_str().ok()).unwrap_or("");

    match auth_service.authenticate(header).await {
        Ok(context) => {
            tracing::Span::current().record("auth.token_id", field::display(&context.token_id));
            request.extensions_mut().insert(context);
            Ok(next.run(request).await)
        }
        Err(err) => {
            warn!(%correlation_id, error = %err, "authentication failed");
            Err(map_auth_error(err))
        }
    }
}

/// Middleware entry point that verifies the caller has the required scopes.
pub async fn ensure_scopes(
    State(required_scopes): State<ScopeState>,
    Extension(context): Extension<AuthContext>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let required_summary =
        required_scopes.iter().map(|scope| scope.as_str()).collect::<Vec<_>>().join(" ");
    let granted_summary =
        context.scopes().map(|scope| scope.as_str()).collect::<Vec<_>>().join(" ");
    let correlation_id = uuid::Uuid::new_v4();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let span = info_span!(
        "auth_middleware.ensure_scopes",
        http.method = %method,
        http.path = %path,
        auth.token_id = %context.token_id,
        required_scopes = %required_summary,
        correlation_id = %correlation_id
    );
    let _guard = span.enter();

    // Check if the user has the required scopes
    let has_required_scopes = required_scopes.iter().all(|scope| context.has_scope(scope));

    if has_required_scopes {
        return Ok(next.run(request).await);
    }

    warn!(
        %correlation_id,
        required = %required_summary,
        granted = %granted_summary,
        "scope check failed"
    );
    Err(ApiError::forbidden("forbidden: missing required scope"))
}

fn map_auth_error(err: AuthError) -> ApiError {
    match err {
        AuthError::MissingBearer
        | AuthError::MalformedBearer
        | AuthError::TokenNotFound
        | AuthError::InactiveToken
        | AuthError::ExpiredToken => ApiError::unauthorized(err.to_string()),
        AuthError::Forbidden => ApiError::forbidden(err.to_string()),
        AuthError::Persistence(inner) => {
            ApiError::service_unavailable(format!("auth service unavailable: {}", inner))
        }
    }
}
