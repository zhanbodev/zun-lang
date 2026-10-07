# EPIC-02 HTTP Enhancement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete EPIC-02 by finishing `@SET_HDR`, adding `@CORS`, and adding real HTTP integration coverage without changing default HTTP behavior for scripts that declare no annotations.

**Architecture:** Reuse the existing `@SET_HDR` parser/runtime slice, extend the same annotation pipeline with `@CORS`, and keep configuration static by attaching annotation data to `Program`, `HttpBlockStmt`, `HttpFnStmt`, and `HttpRoute`. The Axum backend remains the single place where response headers and CORS are materialized into real HTTP responses, while test-mode runtime assertions continue to cover registration semantics and new Rust integration tests cover live HTTP behavior.

**Tech Stack:** Rust, Zun frontend/parser, Zun runtime, Axum, tower-http CORS, tokio, Rust integration tests

---

### Task 1: Finish `@SET_HDR` Through Real HTTP Responses

**Files:**
- Modify: `crates/zun-cli/src/backends/axum_backend.rs`
- Modify: `crates/zun-cli/Cargo.toml`
- Modify: `tests/integration_suite.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing backend response-header tests**

```rust
#[tokio::test]
async fn set_hdr_route_headers_are_emitted_in_real_http_responses() {
    let source = r#"
@SET_HDR({ "Cache-Control": "max-age=60" })
$GET("/items") list() -> JSON {
    $# {"ok": true};
}

$main() {
    $serve(0);
}
"#;

    let response = support::http::spawn_and_get(source, "/items").await;
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "max-age=60"
    );
}
```

```rust
#[tokio::test]
async fn set_hdr_block_and_route_headers_merge_in_real_http_responses() {
    let source = r#"
@SET_HDR({ "Cache-Control": "no-cache" })
$HTTP("/api") {
    @SET_HDR({ "Cache-Control": "max-age=3600", "X-Route": "static" })
    $GET("/items") list() -> JSON { $# {"ok": true}; }
}

$main() {
    $serve(0);
}
"#;

    let response = support::http::spawn_and_get(source, "/api/items").await;
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "max-age=3600"
    );
    assert_eq!(response.headers().get("x-route").unwrap(), "static");
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test --test integration_suite set_hdr -- --nocapture`
Expected: FAIL because `build_http_response(...)` does not yet append `route.response_headers` to the real Axum response.

- [ ] **Step 3: Implement minimal backend header emission and validation**

```rust
use axum::http::header::{HeaderName, HeaderValue};

fn build_http_response(
    result: Option<ZunValue>,
    return_type: &str,
    response_headers: &[(String, String)],
) -> axum::response::Response {
    let mut response = match result {
        Some(ZunValue::Response { status, body }) => {
            let json_body = body.map(|b| value_to_json(&b)).unwrap_or(JsonValue::Null);
            let status_code = axum::http::StatusCode::from_u16(status)
                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
            (status_code, axum::response::Json(json_body)).into_response()
        }
        Some(ZunValue::Html(html_val)) => {
            let html_content = match *html_val {
                ZunValue::Str(s) => s,
                v => v.to_string(),
            };
            axum::response::Html(html_content).into_response()
        }
        other => build_default_response(other, return_type),
    };

    for (name, value) in response_headers {
        let Ok(name) = HeaderName::try_from(name.as_str()) else {
            eprintln!("warning[ZUN-C003]: invalid response header name skipped: {:?}", name);
            continue;
        };
        let Ok(value) = HeaderValue::try_from(value.as_str()) else {
            eprintln!("warning[ZUN-C003]: invalid response header value skipped for {}", name);
            continue;
        };
        response.headers_mut().append(name, value);
    }

    response
}
```

```rust
// call site
build_http_response(result, return_type, &route_definition.response_headers)
```

- [ ] **Step 4: Run the focused tests and confirm they pass**

Run: `cargo test --test integration_suite set_hdr -- --nocapture`
Expected: PASS for real response header emission and existing runtime-level header propagation coverage.

- [ ] **Step 5: Commit the `@SET_HDR` backend slice**

```bash
git add crates/zun-cli/src/backends/axum_backend.rs crates/zun-cli/Cargo.toml tests/integration_suite.rs
git commit -m "feat: emit @SET_HDR response headers"
```

### Task 2: Add `@CORS` Tokens, AST, Parser, And Runtime Propagation

**Files:**
- Modify: `crates/zun-frontend/src/token/token.rs`
- Modify: `crates/zun-frontend/src/lexer/lexer.rs`
- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/mod.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/http.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/mod.rs`
- Modify: `crates/zun-frontend/src/diagnostics/codes.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/http.rs`
- Test: frontend parser tests near `crates/zun-frontend/src/parser/mod.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing parser and runtime propagation tests**

```rust
#[test]
fn parse_global_block_and_route_cors_annotations() {
    let source = r#"
@CORS("*")
$main() { $serve(8080); }

@CORS({ origins: ["https://admin.example.com"], methods: ["GET"] })
$HTTP("/admin") {
    @CORS({ origins: ["https://trusted.example.com"], methods: ["GET", "POST"] })
    $GET("/stats") stats() -> JSON { $# {"ok": true}; }
}
"#;

    let program = crate::parser::parse(source).expect("program should parse");
    assert!(!program.is_empty());
}
```

```rust
#[test]
fn parse_duplicate_cors_annotation_fails() {
    let source = r#"
@CORS("*")
@CORS("*")
$GET("/items") list() -> JSON { $# {"ok": true}; }
"#;

    let error = crate::parser::parse(source).expect_err("program should fail");
    assert!(error.to_string().contains("ZUN-P004"));
}
```

```rust
#[test]
fn http_block_cors_fills_child_route_when_route_has_no_override() {
    let outcome = support::run_fixture("fixtures/http/cors_block.zun", RuntimeMode::Test);
    let route = support::route_by_signature(outcome.context.routes(), "GET", "/api/items")
        .expect("route should exist");
    assert!(route.cors.is_some());
}
```

- [ ] **Step 2: Add the minimal failing fixture**

```zun
@CORS({ origins: ["https://admin.example.com"], methods: ["GET"] })
$HTTP("/api") {
    $GET("/items") list() -> JSON { $# {"ok": true}; }
}
```

- [ ] **Step 3: Run the focused tests and confirm they fail**

Run: `cargo test -p zun-frontend cors -- --nocapture`
Expected: FAIL because `@CORS` tokenization, AST fields, diagnostics, and parser support do not exist yet.

Run: `cargo test --test integration_suite cors -- --nocapture`
Expected: FAIL because `HttpRoute.cors` and `RuntimeContext.global_cors` do not exist yet.

- [ ] **Step 4: Implement the minimal frontend and runtime propagation**

```rust
// token.rs
pub enum Type {
    // ...
    At,
    AtCors,
    AtSetHdr,
}
```

```rust
// ast.rs
pub struct CorsConfig {
    pub allow_all: bool,
    pub origins: Vec<String>,
    pub methods: Vec<String>,
    pub headers: Vec<String>,
    pub max_age: Option<i64>,
    pub credentials: bool,
}

pub struct Program {
    pub global_cors: Option<CorsConfig>,
    pub statements: Vec<Stmt>,
}
```

```rust
// interpreter/mod.rs
pub struct HttpRoute {
    // ...
    pub cors: Option<crate::ast::CorsConfig>,
}
```

```rust
// runtime/context.rs
pub fn set_global_cors(&mut self, cors: Option<crate::ast::CorsConfig>) {
    self.global_cors = cors;
}

pub fn global_cors(&self) -> Option<&crate::ast::CorsConfig> {
    self.global_cors.as_ref()
}
```

```rust
// interpreter/exec/http.rs
let route = HttpRoute {
    // ...
    cors: stmt.cors.clone(),
    response_headers: headers_to_pairs(&stmt.headers),
    // ...
};
```

```rust
// block propagation
let resolved_cors = route.cors.clone().or_else(|| stmt.cors.clone());
```

- [ ] **Step 5: Run the focused tests and confirm they pass**

Run: `cargo test -p zun-frontend cors -- --nocapture`
Expected: PASS for parser and diagnostic coverage.

Run: `cargo test --test integration_suite cors -- --nocapture`
Expected: PASS for runtime registration and propagation coverage.

- [ ] **Step 6: Commit the `@CORS` parser/runtime slice**

```bash
git add crates/zun-frontend/src/token/token.rs crates/zun-frontend/src/lexer/lexer.rs crates/zun-frontend/src/ast/ast.rs crates/zun-frontend/src/parser/mod.rs crates/zun-frontend/src/parser/stmt/http.rs crates/zun-frontend/src/parser/stmt/mod.rs crates/zun-frontend/src/diagnostics/codes.rs crates/zun-runtime/src/runtime/context.rs crates/zun-runtime/src/interpreter/mod.rs crates/zun-runtime/src/interpreter/exec/http.rs tests/integration_suite.rs
git commit -m "feat: parse and propagate @CORS annotations"
```

### Task 3: Materialize `@CORS` In Axum

**Files:**
- Modify: `crates/zun-cli/Cargo.toml`
- Modify: `crates/zun-cli/src/backends/axum_backend.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing live HTTP CORS tests**

```rust
#[tokio::test]
async fn cors_global_allow_all_handles_preflight() {
    let source = r#"
@CORS("*")
$GET("/items") list() -> JSON { $# {"ok": true}; }
$main() { $serve(0); }
"#;

    let response = support::http::spawn_and_options(
        source,
        "/items",
        "https://app.example.com",
        "GET",
    )
    .await;

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("access-control-allow-origin").unwrap(),
        "*"
    );
}
```

```rust
#[tokio::test]
async fn cors_route_overrides_global() {
    let source = r#"
@CORS("*")
$HTTP("/api") {
    @CORS({ origins: ["https://trusted.example.com"], methods: ["GET"] })
    $GET("/items") list() -> JSON { $# {"ok": true}; }
}
$main() { $serve(0); }
"#;

    let ok = support::http::spawn_and_options(
        source,
        "/api/items",
        "https://trusted.example.com",
        "GET",
    )
    .await;
    assert_eq!(
        ok.headers().get("access-control-allow-origin").unwrap(),
        "https://trusted.example.com"
    );
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test --test integration_suite cors_ -- --nocapture`
Expected: FAIL because `tower-http` does not yet enable the `cors` feature and no route-specific `CorsLayer` exists.

- [ ] **Step 3: Implement minimal Axum CORS resolution and validation**

```toml
tower-http = { version = "0.5", features = ["cors", "fs"] }
```

```rust
fn resolve_cors<'a>(
    route_cors: Option<&'a zun::ast::CorsConfig>,
    global_cors: Option<&'a zun::ast::CorsConfig>,
) -> Option<&'a zun::ast::CorsConfig> {
    route_cors.or(global_cors)
}
```

```rust
fn build_cors_layer(config: &zun::ast::CorsConfig) -> Option<tower_http::cors::CorsLayer> {
    use tower_http::cors::{Any, CorsLayer};

    if config.allow_all {
        let layer = CorsLayer::new().allow_origin(Any);
        return Some(if config.credentials {
            layer.allow_credentials(true)
        } else {
            layer
        });
    }

    // map explicit origins/methods/headers here and drop invalid entries
    Some(CorsLayer::new())
}
```

```rust
if let Some(cors) = resolve_cors(route_definition.cors.as_ref(), handler_context.global_cors()) {
    if let Some(layer) = build_cors_layer(cors) {
        router = router.route_layer(layer);
    }
}
```

- [ ] **Step 4: Run the focused tests and confirm they pass**

Run: `cargo test --test integration_suite cors_ -- --nocapture`
Expected: PASS for allow-all preflight and route-over-global precedence behavior.

- [ ] **Step 5: Commit the Axum CORS slice**

```bash
git add crates/zun-cli/Cargo.toml crates/zun-cli/src/backends/axum_backend.rs tests/integration_suite.rs
git commit -m "feat: serve @CORS annotations with axum"
```

### Task 4: Add HTTP Integration Harness And Regression Matrix

**Files:**
- Modify: `tests/support/mod.rs`
- Modify: `tests/integration_suite.rs`
- Create: `tests/fixtures/http/cors_block.zun`
- Create: `tests/fixtures/http/cors_override.zun`
- Modify: `docs/CHANGELOG.md`
- Modify: `docs/spec/security-model.md`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing harness helpers**

```rust
pub async fn spawn_and_get(source: &str, path: &str) -> reqwest::Response {
    unimplemented!("start Zun server on a random port, issue GET, return response");
}

pub async fn spawn_and_options(
    source: &str,
    path: &str,
    origin: &str,
    request_method: &str,
) -> reqwest::Response {
    unimplemented!("start Zun server on a random port, issue OPTIONS preflight, return response");
}
```

- [ ] **Step 2: Run the full integration suite and confirm the new harness-dependent tests fail**

Run: `cargo test --test integration_suite -- --nocapture`
Expected: FAIL because the HTTP test harness does not yet exist or cannot start real servers.

- [ ] **Step 3: Implement the minimal harness and regression coverage**

```rust
pub async fn spawn_server(source: &str) -> TestServer {
    let port = reserve_random_port();
    let project_dir = write_temp_project(source);
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_zun-cli"))
        .arg("serve")
        .arg(project_dir.join("main.zun"))
        .env("ZUN_TEST_PORT", port.to_string())
        .spawn()
        .expect("server should start");

    wait_for_http_ready(port).await;
    TestServer { child, port, project_dir }
}
```

```rust
#[tokio::test]
async fn cors_and_set_hdr_coexist_without_conflict() {
    let source = r#"
@CORS("*")
$HTTP("/api") {
    @SET_HDR({ "X-App": "zun" })
    $GET("/items") list() -> JSON { $# {"ok": true}; }
}
$main() { $serve(0); }
"#;

    let response = support::http::spawn_and_get(source, "/api/items").await;
    assert_eq!(response.headers().get("x-app").unwrap(), "zun");
    assert_eq!(
        response.headers().get("access-control-allow-origin").unwrap(),
        "*"
    );
}
```

- [ ] **Step 4: Update user-visible docs**

```md
## Unreleased

- HTTP: added declarative `@SET_HDR(...)` response headers for HTTP blocks and routes
- HTTP: added declarative `@CORS(...)` configuration for global, block, and route scopes
- Tests: added live HTTP integration coverage for headers, CORS, params, and status behavior
```

```md
- HTTP response headers and CORS policy are now configurable through static route annotations.
- These features do not expand filesystem or outbound network permissions; they only affect HTTP responses emitted by `serve` mode.
```

- [ ] **Step 5: Run the full verification set and confirm it passes**

Run: `cargo test -p zun-frontend -- --nocapture`
Expected: PASS

Run: `cargo test --test integration_suite -- --nocapture`
Expected: PASS

Run: `cargo test -- --nocapture`
Expected: PASS for the full workspace regression set.

- [ ] **Step 6: Commit the HTTP integration and documentation slice**

```bash
git add tests/support/mod.rs tests/integration_suite.rs tests/fixtures/http/cors_block.zun tests/fixtures/http/cors_override.zun docs/CHANGELOG.md docs/spec/security-model.md
git commit -m "test: add EPIC-02 HTTP integration coverage"
```
