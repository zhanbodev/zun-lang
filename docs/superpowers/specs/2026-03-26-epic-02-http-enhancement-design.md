# EPIC-02 HTTP Enhancement Design

## Summary

EPIC-02 extends Zun's HTTP server with two new declarative annotation capabilities:

- `@SET_HDR({ "Header-Name": "value", ... })` for static response headers on HTTP blocks and routes
- `@CORS(...)` for static CORS policy on global, block, and route scopes

The implementation keeps annotations separate from ordinary `$` language statements. Annotations are parsed only as prefix metadata for `$main()`, `$HTTP {}`, and HTTP route declarations, then resolved into runtime route configuration during HTTP registration.

The Epic will be delivered in the documented order:

1. HTTP-001: finish `@SET_HDR`
2. HTTP-002: add `@CORS`
3. HTTP-003: add real HTTP integration coverage

## Goals

- Keep existing HTTP behavior fully backward compatible when no annotation is declared
- Introduce `@` annotations as a distinct syntax family, decoupled from normal statements and keywords
- Finish the partially implemented `@SET_HDR` pipeline through Axum response emission
- Add route, block, and global CORS configuration with explicit precedence rules
- Add stable Rust HTTP integration tests that validate real response headers and status codes

## Non-Goals

- Dynamic annotation expressions
- Arbitrary annotation support outside HTTP-related nodes
- Refactoring the full serve architecture
- Merging CORS fields across scopes
- Global `@SET_HDR`

## Current State

- The frontend already contains partial `@SET_HDR` token and parser support
- Runtime route registration already carries `response_headers`
- Axum response building does not yet write those headers into real HTTP responses
- `@CORS` data structures, parser support, runtime propagation, and Axum integration are still missing
- Existing tests mostly cover runtime/test-mode route registration, not real HTTP requests

## Language Design

### Annotation Model

Annotations are a separate syntax family from ordinary `$` keywords and statements.

- Lexer recognizes `@`-prefixed tokens using longest-match-first:
  - `@CORS` -> `AtCors`
  - `@SET_HDR` -> `AtSetHdr`
  - `@` -> `At`
- Parser only accepts annotation tokens in explicit prefix positions
- Parsed annotations attach to host nodes instead of becoming standalone executable statements

This keeps annotation registration decoupled from normal statement parsing and preserves room for future `@...` extensions.

### `@SET_HDR`

Supported scopes:

- block-level: before `$HTTP(...) { ... }` or `$HTTP(...).link(...)`
- route-level: before `$GET/$POST/$PUT/$DEL/$PATCH`

Not supported:

- global-level before `$main()`
- ordinary functions
- variable declarations
- arbitrary statement positions

Data model:

```rust
pub struct SetHdrEntry {
    pub name: String,
    pub value: String,
}
```

Host nodes:

- `HttpBlockStmt.headers: Vec<SetHdrEntry>`
- `HttpFnStmt.headers: Vec<SetHdrEntry>`

Semantics:

- No annotation means no custom response headers are added
- Block-level headers apply to all routes in the block
- Route-level headers override same-name block-level headers
- Different header names stack

### `@CORS`

Supported scopes:

- global-level: immediately before `$main()`
- block-level: before `$HTTP(...) { ... }` or `$HTTP(...).link(...)`
- route-level: before `$GET/$POST/$PUT/$DEL/$PATCH`

Data model:

```rust
pub struct CorsConfig {
    pub allow_all: bool,
    pub origins: Vec<String>,
    pub methods: Vec<String>,
    pub headers: Vec<String>,
    pub max_age: Option<i64>,
    pub credentials: bool,
}
```

Host nodes:

- `Program.global_cors: Option<CorsConfig>`
- `HttpBlockStmt.cors: Option<CorsConfig>`
- `HttpFnStmt.cors: Option<CorsConfig>`

Accepted forms:

- `@CORS("*")`
- `@CORS({ origins: [...], methods: [...], headers: [...], max_age: ..., credentials: ... })`

Semantics:

- No annotation means no CORS layer is applied
- Precedence is route-level > block-level > global-level
- The winner fully replaces lower scopes
- CORS fields are never merged across scopes

## Parser And Diagnostics

### Position Rules

- `@SET_HDR` is valid only before `$HTTP` blocks or HTTP route declarations
- `@CORS` is valid only before `$main()`, `$HTTP` blocks, or HTTP route declarations
- Annotation parsing is order-insensitive when both `@SET_HDR` and `@CORS` appear before the same host node

### Diagnostic Rules

`@SET_HDR`:

- `ZUN-P006`: invalid position
- `ZUN-P007`: invalid syntax or invalid same-node duplication
- `ZUN-C003`: invalid response header name at startup, warning and skip

`@CORS`:

- `ZUN-P003`: invalid position
- `ZUN-P004`: duplicate `@CORS` on the same node
- `ZUN-P005`: invalid syntax
- `ZUN-C002`: invalid startup configuration

The parser remains strict: annotations require static literals and do not evaluate runtime expressions.

## Runtime Design

### Route Registration

`HttpRoute` gains:

```rust
pub cors: Option<CorsConfig>
```

`RuntimeContext` gains:

```rust
pub global_cors: Option<CorsConfig>
```

Propagation rules:

- `handle_http_fn()` writes route-level `headers` and `cors` directly into `HttpRoute`
- `handle_http_block()` merges block-level headers into each child route and fills block-level CORS only when a child route does not define its own CORS
- linked module routes inherit block-level headers and block-level CORS from the parent `$HTTP(...).link(...)`
- global CORS is stored in `RuntimeContext` when `$main()` is interpreted

### Default Behavior

When no annotation is declared:

- `@SET_HDR`: no custom response headers are added
- `@CORS`: no CORS headers are emitted and no extra CORS handling is enabled

This is an explicit opt-in design. Existing HTTP scripts remain unchanged.

## Axum Backend Design

The backend remains structurally the same. The Epic adds static HTTP configuration at route registration time without redesigning the serve pipeline.

### Response Headers

`build_http_response(...)` will accept the resolved route headers and append valid headers to the produced `axum::response::Response`.

Validation:

- invalid header names or values are skipped
- invalid names emit `ZUN-C003` warning
- valid headers are appended to every successful HTTP response for that route

### CORS

`tower-http` gains the `cors` feature.

For each route, the backend resolves effective CORS as:

1. route-level `route.cors`
2. fallback to `context.global_cors()`
3. otherwise none

Block-level CORS is already folded into `route.cors` during route registration, so Axum only needs to resolve route-vs-global.

Each route gets its own `CorsLayer` built from the resolved static config. OPTIONS preflight handling is delegated to `tower_http::cors::CorsLayer`.

Startup validation:

- `credentials: true` with wildcard origins is an error
- malformed origins, methods, or headers become warnings and are skipped
- empty effective origin sets degrade to no CORS for that route

## Testing Strategy

### Runtime/Test-Mode Tests

Keep and extend runtime-level tests for:

- annotation parsing
- route registration
- block-level inheritance
- linked module propagation
- precedence behavior without booting a real server

### Real HTTP Integration Tests

Add a dedicated Rust integration test group for real HTTP requests.

Scope:

- path params
- query params
- request body decoding
- `$HDR`
- `$RES` status behavior
- `@SET_HDR` response headers
- `@CORS` global, block, and route precedence
- `@CORS` + `@SET_HDR` coexistence

Test harness requirements:

- use random available ports to avoid collisions
- boot a real Zun HTTP server
- issue real requests with an HTTP client
- assert actual response headers and status codes

This is required because `@SET_HDR` and `@CORS` cannot be fully validated through test-mode route registration alone.

## Documentation Updates

The implementation must update:

- `docs/spec/security-model.md`
- `docs/CHANGELOG.md`

Security model changes are limited to documenting new HTTP response-surface behavior and CORS policy handling. The Epic does not expand filesystem, environment, or outbound network capabilities.

## Delivery Plan

### HTTP-001

- finish Axum response header emission
- add startup validation for response headers
- keep existing parser/runtime direction
- verify linked module inheritance and override semantics

### HTTP-002

- add annotation tokens, AST fields, parser support, and diagnostics for `@CORS`
- propagate global, block, and route CORS into runtime registration
- build Axum `CorsLayer` from static config

### HTTP-003

- add real HTTP integration harness
- cover regression matrix for precedence, compatibility, and coexistence

## Risks And Mitigations

- Partial existing `@SET_HDR` work may differ from issue prose
  - Mitigation: trust current implementation plus tests where they already exist, then align docs and runtime behavior
- Route-specific CORS layers may be awkward in Axum composition
  - Mitigation: keep resolution local to each route registration instead of adding a global dynamic middleware
- Startup validation may silently diverge from issue requirements
  - Mitigation: encode warning/error expectations in integration tests

## Acceptance Summary

The Epic is complete when:

- `@SET_HDR` is fully emitted in real HTTP responses
- `@CORS` works at global, block, and route scopes with the documented precedence
- scripts without annotations behave exactly as before
- real HTTP integration tests cover the documented behavior matrix
