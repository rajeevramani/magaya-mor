# Rust Docs Style Guide

Guidelines for documenting Rust crates, modules, and items.

## Comment Types
- `///` item docs: describe public types, functions, modules.
- `//!` crate/module docs: high-level overview, architecture, and examples.
- Keep prose concise and task-oriented.

## Recommended Sections
- Summary: a one-line description starting with a verb or noun phrase.
- Details: clarify behavior, invariants, and noteworthy edge-cases.
- Examples: minimal, runnable `rust` code blocks. Hide setup with `#` lines when needed.
- Errors: list major error variants/conditions.
- Panics: when and why an API panics (ideally never in library code).
- Safety: required for any `unsafe` function or when invariants matter.

## Examples
```rust
/// Parses a config string into a type.
///
/// # Examples
/// ```
/// # use my_crate::Config;
/// let cfg: Config = "port=8080".parse().unwrap();
/// assert_eq!(cfg.port, 8080);
/// ```
```
- Use `no_run` for network/IO-heavy examples; prefer real, fast examples when possible.

## Intra-doc Links
- Link to items with `[Type]` or `[module::Item]` using Rustdoc’s resolver.
- Prefer linking to crate items instead of external URLs when possible.

## Configuration & Env Docs
- Document environment variables and config fields with names, defaults, units, and examples.
- For Flowplane: reference the `DATABASE_URL` requirement and relevant config structs.

## Tracing & Observability
- Document emitted spans/fields when they matter to operators.

## Style & Tone
- Present tense, active voice. Avoid redundancy; let code express obvious details.
- Keep lines reasonably short; let `rustfmt` reflow long use cases.

## API Documentation Standards
Standards for documenting HTTP APIs built with Axum and described via `utoipa`.

- Annotations: document every handler with `#[utoipa::path(...)]` including `method`, `path`, `params`, `request_body`, `responses`, and `tag`.
- Schemas: derive `utoipa::ToSchema` for all request/response structs and `utoipa::IntoParams` for query/path structs. Use `#[serde(rename_all = "camelCase")]` for external payloads.
- Casing: external JSON uses `camelCase` except Envoy filter blocks which may be `snake_case`. Call out exceptions explicitly in docs.
- Status codes: choose precise codes. Examples:
  - 200 for read, 201 for create (consider a `Location` header), 202 for async acceptance, 204 for delete with no body.
  - 400 validation, 401 unauthenticated, 403 forbidden, 404 not found, 409 conflict, 429 rate limited, 5xx for server faults.
- Error envelope: expose a single error schema and reuse it in all error responses. Example pattern:
  - Type: `HttpError { error: String, message: String }` with `ToSchema`.
  - In `#[utoipa::path] responses(...)`, reference `body = HttpError` for error statuses.
  - Map domain errors to HTTP codes at the boundary; avoid leaking internal messages.
- Parameters: document defaults and formats. Use `IntoParams` with `#[param(required = false, example = ...)]`. Clarify pagination (`limit`, `offset` or cursor), filtering, sorting, and allowed enums.
- Examples: include a compact JSON example for request/response bodies and a curl snippet for critical routes. Prefer real values; redact secrets.
- Content types: state response `content_type` when not `application/json` (e.g., YAML, binary). For multiple formats, document selection (e.g., `?format=yaml|json`).
- Tags and grouping: group endpoints by domain (e.g., `platform-api`) using `tag = "..."` and keep names stable.
- Security: declare security schemes (e.g., bearer token) at the top-level OpenAPI and reference with `security(...)` in `utoipa::path` where required. Document required scopes/roles in the endpoint description.
- Versioning and deprecation:
  - Version in the path (`/api/v1/...`). Avoid breaking changes within a version; add fields in a backward-compatible way.
  - Mark deprecated endpoints/fields with `#[deprecated]` and note alternatives in the description.
- Headers: document important headers (`Idempotency-Key`, `Location`, `Retry-After`) via `#[utoipa::path(responses(..., headers(...)))]` when relevant.
- Consistency checks: ensure every public API has corresponding OpenAPI coverage so `/api-docs/openapi.json` and `/swagger-ui` reflect reality. CI should fail on missing schemas for new endpoints.

### Minimal Example
```rust
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateThingRequest { name: String }

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThingResponse { id: String, name: String }

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HttpError { error: String, message: String }

#[utoipa::path(
  post,
  path = "/api/v1/things",
  request_body = CreateThingRequest,
  responses(
    (status = 201, description = "Created", body = ThingResponse),
    (status = 400, description = "Invalid request", body = HttpError),
    (status = 409, description = "Conflict", body = HttpError)
  ),
  tag = "things",
  security(("bearerAuth" = []))
)]
async fn create_thing(Json(_req): axum::Json<CreateThingRequest>) -> axum::Json<ThingResponse> {
  unimplemented!()
}
```

### Reviewer Checklist
- Endpoint has `#[utoipa::path]` with correct method, path, tags, params, and responses.
- All request/response/error types derive `ToSchema`; query/path types derive `IntoParams`.
- Status codes are correct; error responses reference the shared error schema.
- Examples are present and valid; non-JSON content types documented.
- Security requirements are documented; version prefix is correct; deprecation is noted if applicable.
