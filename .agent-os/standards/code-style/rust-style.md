# Rust Style Guide

This guide defines Rust-specific conventions for Agent OS projects. When Rust rules conflict with General Formatting, Rust rules take precedence (e.g., indentation).

## Tooling & Enforcement
- Formatting: run `cargo fmt` (respect repository `rustfmt.toml` if present).
- Linting: run `cargo clippy -- -D warnings` with no allow-by-default warnings in CI.
- Edition: use the edition set in `Cargo.toml` (prefer stable latest for new crates).
- IDE: prefer rust-analyzer defaults; avoid custom formatting overrides.

## Indentation & Layout
- Indentation is 4 spaces (rustfmt default). This overrides the General Formatting 2-space rule for Rust.
- Line width: follow `rustfmt` (typically 100); break method chains and builders accordingly.
- One item per `use` line after grouping; prefer `rustfmt`-driven import reordering.

## Naming
- Functions, methods, variables: `snake_case`.
- Types, traits, structs, enums: `PascalCase`.
- Constants and statics: `SCREAMING_SNAKE_CASE`.
- Feature flags: `kebab-case` in `Cargo.toml`.

## Clippy
- Ensure all clippy errors are cleaned up

## Modules & Imports
- Prefer explicit imports; avoid glob imports (`use crate::*`) except in tests/benches.
- Import order: standard lib, external crates, then `crate`/`super` paths.
- Keep modules small and cohesive; avoid large `mod.rs` files. Use a file-per-module layout.
- Re-export sparingly from `lib.rs` to maintain a clear public API surface.

## Errors
- Libraries: define domain errors with `thiserror`. Binaries and tests: use `anyhow` for flexible error contexts.
- Avoid `unwrap()`/`expect()` outside tests/examples. Use `?` with context: `anyhow::Context` to describe the failing operation.
- Map external errors into domain errors near boundaries.

## Utoipa
- Make sure all Utoipa documentation text reflects the APIs they represent

## Concurrency & Async
- Use `tokio` for async; annotate tests with `#[tokio::test]`. Prefer `multi_thread` flavor if tasks block.
- Add timeouts for external IO and tests (`tokio::time::timeout`). Handle cancellation (`select!{}`).
- Don’t detach tasks unless intentional; propagate errors from spawned tasks.

## Serialization
- Use `serde` derives. Prefer explicit field names and `#[serde(rename_all = "snake_case")]` for externally visible models.
- Consider `#[serde(deny_unknown_fields)]` for API boundary structs to fail fast on unexpected input.

## Logging & Tracing
- Use `tracing` for logs and spans. Avoid `println!` outside examples/tests.
- Instrument async entry points with `#[tracing::instrument(skip_all, fields(...))]` where helpful.

## Unsafe
- `#![forbid(unsafe_code)]` in libraries unless a compelling, reviewed justification exists. If used, isolate, document invariants and safety rationale.

## Performance & Allocations
- Prefer iterator adapters to intermediate `Vec`s. Avoid needless cloning; use references and `Cow` where appropriate.
- Use `SmallVec`, `FxHashMap`, or feature flags only when profiling indicates value.

## Public API & Features
- Keep breaking changes behind feature flags until stabilized.
- Make default features minimal; additive features should compose cleanly.

## Repository Conventions (Flowplane)
- Follow `docs/contributing.md` commands before PRs: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`.
- Use structured config models (avoid raw base64 for Envoy typed configs unless well-documented).
- Keep validation at API boundaries to surface errors early.

