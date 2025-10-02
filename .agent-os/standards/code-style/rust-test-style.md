# Rust Test Style Guide

Consistent patterns for unit, integration, and end-to-end tests in Rust.

## Structure
- Unit tests: colocate in `mod tests` within the source file.
- Integration tests: add files under `tests/` using public APIs.
- E2E/system tests: place in `tests/e2e/` with helpers in `tests/e2e/support/`.

## Async Tests
- Use `#[tokio::test]` (prefer `flavor = "multi_thread"` if running concurrent tasks).
- Apply timeouts for network/IO: `tokio::time::timeout(Duration::from_secs(…))`.
- Avoid fixed sleeps; poll for readiness or use signaling (e.g., oneshot channels).

## Assertions & Errors
- Test functions may return `anyhow::Result<()>` for ergonomic `?` usage.
- Prefer `assert_eq!`/`assert_ne!` and meaningful messages over `assert!(cond)`.
- For floats, use approximate comparison helpers.

## Isolation & Fixtures
- Use temporary directories/files with `tempfile` or `std::env::temp_dir()` and unique names.
- Don’t rely on global state. Pass explicit config/addresses/ports into helpers.
- Reuse provided helpers when present (e.g., `ControlPlaneHandle`, `EnvoyHandle`) to start/stop subsystems and avoid leaks.

## Logging in Tests
- Initialize `tracing_subscriber` once per test binary when diagnosing failures. Keep default quiet in CI; enable via `RUST_LOG` when needed.

## Determinism & Flake Resistance
- Avoid depending on wall-clock sleeps; prefer readiness checks and bounded retries.
- Ensure external ports are configurable and unique per test run to avoid conflicts.

## Doc Tests
- Provide runnable examples in `///` docs. Use `no_run` or `ignore` for network examples that shouldn’t execute.

## Flowplane-Specific Notes
- Follow `docs/contributing.md`: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` locally.
- Default to testing API boundary validations and xDS translation results with serde roundtrips where applicable.

