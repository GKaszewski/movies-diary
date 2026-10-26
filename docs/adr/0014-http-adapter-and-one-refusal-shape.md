# `http-axum` is the driving adapter, and every JSON refusal has one shape

The old `presentation` crate was renamed `http-axum` and now reads as what it is: the HTTP driving
adapter. It is split into three modules by audience: `api` (JSON under `/api/v1`), `html` (the
classic server-rendered UI, with its askama templates absorbed into `html/templates`) and
`federation` (ActivityPub-facing and federation-gated routes, compiled only with the `federation`
feature, ADR-0009). `router.rs` assembles them; handlers reach use cases only through
`app_ctx.deps` (ADR-0006).

A JSON client previously met several failure bodies: `{"error": "..."}` from `ApiError`, bare
status codes with empty bodies from handlers that returned `StatusCode::NOT_FOUND`, plain text from
`domain_error_response`, axum's plain-text rejections for malformed `Json`/`Query`/`Path`, and the
rate limiter's own 429 text. The refusals were documented without a body schema.

## Decision

**One body.** `api_types::ErrorResponse { error, code }`. `error` keeps its name and meaning (the
SPA and existing clients read it); `code` is an additive, stable machine-readable field:
`NOT_FOUND`, `INVALID_INPUT`, `UNAUTHORIZED`, `INVALID_CREDENTIALS` (wrong password on a re-auth
check; unlike `UNAUTHORIZED` it must not trigger a token refresh), `FORBIDDEN`, `CONFLICT`, `RATE_LIMITED`, `INTERNAL`.

**One constructor.** `errors::refuse(status, code, message) -> Response` is the only place a JSON
error body is built. `ApiError` maps every `DomainError` variant to it with the status mapping that
existed before (`InvalidRating`/`ValidationError` 400, `NotFound` 404, `Unauthorized` 401,
`Forbidden` 403, `InvalidCredentials` 401 (`INVALID_CREDENTIALS`), `InfrastructureError` 500 with the generic message `internal server error` and the
cause logged, never sent).

**Extractors.** `extractors::Body<T>`, `Params<T>` and `PathId<T>` wrap axum's `Json`, `Query` and
`Path` and turn their rejections into `refuse(.., INVALID_INPUT, ..)`. The status is whatever axum
chose for the rejection (400 for syntax errors, 415 wrong content type, 422 well-formed JSON of the
wrong shape, 413 oversize), so clients that already handled those statuses are unaffected; only the
body changed. JSON API handlers use them instead of the axum originals.

**Rate limiting.** The 429 of the `/api/v1` limiters goes through `refuse` (`RATE_LIMITED`). The
HTML and ActivityPub limiters keep axum-governor's default response.

**HTML is unchanged.** Classic handlers keep `domain_error_response` (plain status text / HTML
pages), redirects and statuses. ActivityPub-facing routes keep their status-only errors. Only JSON
responses were unified.

**Not covered.** Unmatched `/api/v1` paths (404) and wrong methods (405) are produced by the router
itself and still return axum's empty bodies.

**The spec says so.** Every operation documents at least one 4xx, every authenticated operation
documents 401, and every 4xx/5xx references `ErrorResponse`. `tests/openapi_contract_test.rs`
builds the spec from `http_axum::openapi::spec()` (no bootstrap) and fails when a new endpoint
skips this, together with a test that every 2xx JSON response declares a schema.

## Consequences

- Statuses of existing JSON endpoints are unchanged; error bodies that were empty or plain text on
  `/api/v1` are now `{"error", "code"}` JSON. Bodies that were already `{"error"}` gained `code`.
- `cargo test -p http-axum` alone does not build with default features because the dev-dependency
  on `bootstrap` needs its `federation` feature to match; `cargo test -p http-axum
  --no-default-features --test openapi_contract_test` runs the contract test standalone, and
  `cargo test --workspace` runs it with federation on.
