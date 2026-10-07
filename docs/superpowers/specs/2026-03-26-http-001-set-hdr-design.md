# HTTP-001 `@SET_HDR` Design

**Goal:** Add declarative response header support for Zun HTTP routes via `@SET_HDR({ "Header-Name": "value", ... })`, with block-level inheritance and route-level override.

## Scope

This design only covers `HTTP-001`.

Included:
- `@SET_HDR` tokenization and parsing
- AST support on HTTP routes and HTTP blocks
- Duplicate and invalid-position diagnostics
- Runtime header merge rules
- Axum response header emission
- Targeted parser/runtime/backend tests

Excluded:
- `@CORS`
- Global `@` annotation handling beyond reserving the shared token entrypoint
- Full HTTP integration suite expansion from `HTTP-003`

## Current State

The current codebase supports:
- HTTP route declarations via `$GET/$POST/$PUT/$DEL/$PATCH`
- HTTP blocks via `$HTTP`
- Request header reads via `$HDR("Name")`
- Response status/body via `$RES(status, body)`

The current codebase does not support:
- Any `@` annotation token
- Response header writes
- Route metadata for response headers
- Real HTTP assertions for emitted custom response headers

## User-Facing Syntax

Route-level:

```zun
@SET_HDR({ "Cache-Control": "max-age=3600" })
$GET("/api/data") getData() {
    $# {"data": 42};
}
```

Block-level:

```zun
@SET_HDR({ "X-Frame-Options": "DENY" })
$HTTP("/api") {
    $GET("/users") getUsers() { $# []; }
}
```

Mixed:

```zun
@SET_HDR({ "Cache-Control": "no-cache" })
$HTTP("/api") {
    @SET_HDR({ "Cache-Control": "max-age=3600", "X-Route": "static" })
    $GET("/static") getStatic() { $# {"ok": true}; }
}
```

## Semantics

### Allowed positions

`@SET_HDR` is only valid immediately before:
- an HTTP route declaration
- an `$HTTP { ... }` block
- an `$HTTP(...).link(...)` block statement

It is invalid before:
- `$main()`
- normal `$fn`
- variable declarations
- arbitrary statements

### Header merge rules

For a route nested under an annotated `$HTTP` block:
- block-level headers apply to every child route
- route-level headers override block-level headers with the same name
- different header names accumulate

Header name comparison is case-insensitive for duplicate detection and override behavior.

### Duplicate rules

Within the same annotation target:
- duplicate header names are a hard error
- this applies separately to a route node and a block node

Examples:
- valid: block has `X-A`, route has `X-A` because route overrides block
- same-name `@SET_HDR` entries are allowed; later declarations override earlier ones and emit `ZUN-C003` warning

### Validation timing

Parsing stage:
- illegal position -> `ZUN-P006`
- wrong argument shape -> `ZUN-P007`
- duplicate header names on the same node -> `ZUN-P007`

Startup/backend stage:
- invalid HTTP header name or value rejected by Axum/HTTP types -> `ZUN-C003`
- startup aborts instead of silently skipping

This deliberately differs from the issue draft's warning behavior. The approved behavior for this task is fail-fast.

## Architecture

### Frontend

Add shared `@` token support now so `HTTP-002` can reuse it later:
- `Type::At`
- `Type::AtSetHdr`

Extend AST with:

```rust
pub struct SetHdrEntry {
    pub name: String,
    pub value: String,
}
```

New fields:
- `HttpFnStmt.headers: Vec<SetHdrEntry>`
- `HttpBlockStmt.headers: Vec<SetHdrEntry>`

Parser behavior:
- consume zero or more `@SET_HDR` annotations before `parse_http_fn()`
- consume zero or more `@SET_HDR` annotations before `parse_http_block()`
- reject `@SET_HDR` anywhere else through a top-level parser guard

### Runtime

Extend `HttpRoute` with:

```rust
pub response_headers: Vec<(String, String)>
```

Route registration rules:
- top-level route gets its parsed headers directly
- block child routes receive merged headers computed during block handling
- linked module routes inherit block headers from the linking site

Merge algorithm:
1. Start with block headers in declaration order.
2. Overlay route headers by normalized header name.
3. Preserve one final value per header name.

### Backend

`build_http_response(...)` should accept the resolved route headers and append them to the produced `axum::response::Response`.

Header conversion rules:
- parse header names with `HeaderName::try_from`
- parse values with `HeaderValue::try_from`
- any invalid entry emits a startup warning tagged as `ZUN-C003` and is skipped

The backend should consume already-resolved route metadata. It should not implement language-level precedence logic.

## Files To Change

Frontend:
- `crates/zun-frontend/src/token/token.rs`
- `crates/zun-frontend/src/lexer/lexer.rs`
- `crates/zun-frontend/src/ast/ast.rs`
- `crates/zun-frontend/src/parser/stmt/http.rs`
- `crates/zun-frontend/src/parser/stmt/mod.rs`
- `crates/zun-frontend/src/diagnostics/codes.rs`

Runtime:
- `crates/zun-runtime/src/interpreter/mod.rs`
- `crates/zun-runtime/src/interpreter/exec/http.rs`

CLI/backend:
- `crates/zun-cli/src/backends/axum_backend.rs`

Tests:
- existing frontend/runtime unit tests near touched modules
- `tests/integration_suite.rs` for route metadata coverage
- new Rust HTTP integration tests for emitted response headers

Docs:
- `docs/CHANGELOG.md`

## Testing Strategy

Parser and AST:
- route-level annotation parses into `HttpFnStmt.headers`
- block-level annotation parses into `HttpBlockStmt.headers`
- invalid placement fails with `ZUN-P006`
- invalid argument shape fails with `ZUN-P007`
- same-node duplicate header names fail

Runtime metadata:
- block headers propagate to child routes
- route headers override same-name block headers
- `$HTTP.link()` routes inherit block headers

Real HTTP behavior:
- single route-level header is present in HTTP response
- multiple headers accumulate
- block-level header reaches every child route
- block + route same-name override returns route value
- no `@SET_HDR` keeps current behavior unchanged

## Risks And Constraints

- The current parser only recognizes raw HTTP statements. Adding annotation pre-consumption must not break unannotated `$GET/$HTTP` parsing.
- `$HTTP(...).link(...)` currently builds routes after module loading. Block headers must be applied after linked routes are materialized.
- Header duplicate comparison should normalize names consistently or override behavior will be inconsistent.
- Real HTTP tests need random ports to avoid CI conflicts.

## Rollout Order

1. Frontend token/AST/parser/diagnostics
2. Runtime route metadata and block/link merge
3. Axum response header emission plus startup validation
4. Parser/runtime tests
5. Real HTTP integration tests
6. Changelog update
