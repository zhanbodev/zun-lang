# HTTP Handler Return Type Check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `RuntimeMode::Test` fail fast when an HTTP handler's declared return type does not match the value produced by the handler body, while unifying return-type validation for normal `$fn` and HTTP handlers.

**Architecture:** Keep `serve` behavior unchanged and add a bootstrap-time probe only for test-mode route registration. Extract one shared runtime return-type validator that normal `$fn` calls and HTTP handlers both use, then run HTTP handlers once in an isolated cloned context with synthetic inputs so invalid spec fixtures fail during script execution instead of silently registering bad routes.

**Tech Stack:** Rust, Zun runtime/interpreter, Cargo spec tests, Markdown issue tracking

---

## Research Summary

- `tests/spec/mod.rs` only boots invalid samples with `RuntimeMode::Test`; it does not execute HTTP requests.
- [`crates/zun-runtime/src/interpreter/exec/http.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/exec/http.rs) currently only builds `HttpRoute` and calls `context.register_http_route(...)`.
- [`crates/zun-runtime/src/interpreter/exec/functions.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/exec/functions.rs) already contains normal-function return validation logic, but it is private, built-in-only, and only reachable from `$fn` call paths.
- Existing test fixtures such as [`tests/fixtures/http/echo_route.zun`](/Users/liangzhanbo/CodeStudio/zun/tests/fixtures/http/echo_route.zun) prove that test mode is used for route registration; any bootstrap-time probe must seed params, `__headers__`, and `body` so unrelated fixtures do not fail.

## File Map

- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/http.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `tests/integration_suite.rs`
- Modify: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun`
- Modify: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun.error`
- Modify: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun`
- Modify: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error`
- Create: `tests/spec/valid/http/http_handler_user_return_type.zun`
- Create: `tests/spec/valid/http/http_handler_user_return_type.zun.stdout`

### Task 1: Lock The Failing Regression Fixtures

**Files:**
- Modify: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun`
- Modify: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun.error`
- Modify: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun`
- Modify: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error`
- Test: `tests/spec_suite.rs`

- [ ] **Step 1: Verify the built-in mismatch fixture stays minimal**

```zun
$GET("/users/:id") get_user(id) -> Int {
    $# {"id": id};
}
```

```text
http handler 'get_user' expects return type 'Int' but got 'Map'
```

- [ ] **Step 2: Verify the unknown user-type fixture stays minimal**

```zun
$GET("/users/:id") get_user(id) -> User {
    $# {"id": id};
}
```

```text
http handler 'get_user' references unknown return type 'User'
```

- [ ] **Step 3: Run the failing spec suite before implementation**

Run: `cargo test --test spec_suite invalid_spec_samples_fail_with_expected_error -- --nocapture`
Expected: FAIL because `http_handler_return_type_mismatch.zun` still registers successfully

- [ ] **Step 4: Commit only if the fixtures needed adjustment**

```bash
git add tests/spec/invalid/http/http_handler_return_type_mismatch.zun tests/spec/invalid/http/http_handler_return_type_mismatch.zun.error tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error
git commit -m "test: lock http return type regression fixtures"
```

### Task 2: Extract One Shared Return-Type Validator

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`

- [ ] **Step 1: Replace the private function-only checker with a shared helper**

```rust
pub fn validate_declared_return_type(
    kind: &str,
    name: &str,
    expected_type: Option<&str>,
    value: Option<&ZunValue>,
    context: &RuntimeContext,
) -> Result<(), Error> {
    // Validate built-in names such as Int/Float/String/Bool/Json/List/Map/Response.
    // Validate user-defined names via TypedInstance + context type visibility.
    // Validate List<T> recursively.
}
```

- [ ] **Step 2: Keep `call_fn` delegating to the shared helper**

```rust
match flow {
    Flow::Return(val) => {
        validate_declared_return_type(
            "function",
            &fn_decl.name,
            fn_decl.return_type.as_deref(),
            val.as_ref(),
            context,
        )?;
        Ok(val)
    }
    Flow::Normal => Ok(None),
    // unchanged branches omitted
}
```

- [ ] **Step 3: Keep `call_module_fn` delegating to the same helper**

```rust
match flow {
    Flow::Return(val) => {
        validate_declared_return_type(
            "function",
            &fn_decl.name,
            fn_decl.return_type.as_deref(),
            val.as_ref(),
            context,
        )?;
        Ok(val)
    }
    Flow::Normal => Ok(None),
    // unchanged branches omitted
}
```

- [ ] **Step 4: Add focused unit tests in the same module for built-in, missing-return, and user-type cases**

```rust
#[test]
fn shared_return_type_validator_accepts_matching_int() {
    let context = RuntimeContext::new(RuntimeMode::Test, std::path::PathBuf::from("."));
    let value = ZunValue::Int(1);
    assert!(validate_declared_return_type(
        "function",
        "f",
        Some("Int"),
        Some(&value),
        &context,
    )
    .is_ok());
}
```

- [ ] **Step 5: Run the nearest existing function regression coverage**

Run: `cargo test --test integration_suite runtime_fixtures_cover_core_semantics -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit the shared validator refactor**

```bash
git add crates/zun-runtime/src/interpreter/exec/functions.rs crates/zun-runtime/src/interpreter/mod.rs
git commit -m "refactor: share runtime return type validation"
```

### Task 3: Probe HTTP Handlers During Test-Mode Registration

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/http.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Add a helper that executes one handler body with synthetic request state**

```rust
fn probe_http_handler_return_type(
    route: &HttpRoute,
    context: &RuntimeContext,
) -> Result<(), Error> {
    let mut probe_context = context.clone();
    let mut probe_state = ProgramState::new();

    probe_state.env.extend(route.module_env.clone());
    probe_state.fns.extend(route.module_fns.clone());

    for param in &route.params {
        probe_state
            .env
            .insert(param.clone(), ZunValue::Str("__dummy__".to_string()));
    }

    probe_state.env.insert(
        "__headers__".to_string(),
        ZunValue::Json(indexmap::IndexMap::new()),
    );
    probe_state
        .env
        .insert("body".to_string(), ZunValue::Null);

    let (_should_continue, result, error) =
        exec_http_handler(&route.body, &mut probe_state, &mut probe_context);
    if let Some(error) = error {
        return Err(Error::Interpreter(error));
    }

    validate_declared_return_type(
        "http handler",
        &route.name,
        route.return_type.as_deref(),
        result.as_ref(),
        &probe_context,
    )
}
```

- [ ] **Step 2: Probe top-level `$GET/$POST/...` handlers before registration when in `RuntimeMode::Test`**

```rust
if matches!(context.mode(), RuntimeMode::Test) {
    probe_http_handler_return_type(&route, context)?;
}
context.register_http_route(route);
```

- [ ] **Step 3: Reuse the same probe path for `$HTTP { ... }` routes and linked module routes**

```rust
for route in module_routes {
    if matches!(context.mode(), RuntimeMode::Test) {
        probe_http_handler_return_type(&route, context)?;
    }
    context.register_http_route(route);
}
```

- [ ] **Step 4: Add focused integration coverage for bootstrap-time mismatch failure**

```rust
#[test]
fn http_handler_return_type_mismatch_fails_in_test_mode_bootstrap() {
    let outcome = run_fixture(
        "spec/invalid/http/http_handler_return_type_mismatch.zun",
        RuntimeMode::Test,
    );

    let error = outcome
        .error
        .expect("http return type mismatch fixture should fail");
    assert!(error.contains(
        "http handler 'get_user' expects return type 'Int' but got 'Map'"
    ));
}
```

- [ ] **Step 5: Run the new bootstrap regression test**

Run: `cargo test --test integration_suite http_handler_return_type_mismatch_fails_in_test_mode_bootstrap -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit the test-mode HTTP probe**

```bash
git add crates/zun-runtime/src/interpreter/exec/http.rs crates/zun-runtime/src/interpreter/mod.rs tests/integration_suite.rs
git commit -m "feat: probe http handler return types in test mode"
```

### Task 4: Add Positive Coverage For User Types And Probe Safety

**Files:**
- Create: `tests/spec/valid/http/http_handler_user_return_type.zun`
- Create: `tests/spec/valid/http/http_handler_user_return_type.zun.stdout`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Add a valid HTTP fixture that returns a visible user-defined type**

```zun
$Type User {
    id: Int,
    name: String,
}

$GET("/users/:id") get_user(id) -> User {
    $# User {
        id: 1,
        name: "Ada",
    };
}
```

```text
[INFO] HTTP route registered: GET /users/:id
```

- [ ] **Step 2: Add an integration test that confirms visible user return types pass**

```rust
#[test]
fn http_handler_user_return_type_bootstraps_in_test_mode() {
    let outcome = run_fixture(
        "spec/valid/http/http_handler_user_return_type.zun",
        RuntimeMode::Test,
    );
    assert!(outcome.error.is_none(), "visible user return type should pass");
}
```

- [ ] **Step 3: Add an integration test that proves synthetic probe inputs do not break existing HTTP fixtures**

```rust
#[test]
fn http_fixture_still_bootstraps_when_probe_inputs_are_sufficient() {
    let outcome = run_fixture("fixtures/http/echo_route.zun", RuntimeMode::Test);
    assert!(outcome.error.is_none(), "http fixture should still execute");
    assert_eq!(outcome.context.routes().len(), 2);
}
```

- [ ] **Step 4: Run the valid-spec and probe-safety tests**

Run: `cargo test --test spec_suite valid_spec_samples_pass -- --nocapture`
Expected: PASS

Run: `cargo test --test integration_suite http_handler_user_return_type_bootstraps_in_test_mode -- --nocapture`
Expected: PASS

Run: `cargo test --test integration_suite http_fixture_still_bootstraps_when_probe_inputs_are_sufficient -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the positive coverage**

```bash
git add tests/spec/valid/http/http_handler_user_return_type.zun tests/spec/valid/http/http_handler_user_return_type.zun.stdout tests/integration_suite.rs
git commit -m "test: cover shared http return type validation"
```

### Task 5: Verify End To End And Close The Bug

**Files:**
- Modify: `ISSUES/BUG-001-http-handler-return-type-check.md`

- [ ] **Step 1: Run the invalid spec suite**

Run: `cargo test --test spec_suite invalid_spec_samples_fail_with_expected_error -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the narrow HTTP regression slice**

Run: `cargo test --test integration_suite http_ -- --nocapture`
Expected: existing HTTP integration tests remain green

- [ ] **Step 3: Update the issue status after verification**

```md
## 状态
已修复（Test 模式下已实现 bootstrap-time return type 校验）
```

- [ ] **Step 4: Commit the issue closeout**

```bash
git add ISSUES/BUG-001-http-handler-return-type-check.md
git commit -m "docs: close bug 001 after http return type fix"
```

## Risks And Decisions

- Probe-time execution can trigger side effects. Keep the probe isolated to `RuntimeMode::Test`, use a cloned context, and seed only the minimum synthetic request data needed for route bootstrap.
- `body` must be seeded to `Null`, not omitted, otherwise fixtures that read `body` will fail for unrelated undefined-variable reasons.
- User-defined return-type validation is intentionally included here because the shared validator is the whole point of the refactor. Do not add unrelated parser or syntax cleanup work in the same patch.
