# Return Type Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove `JSON<T>` return annotations, enforce the same return-type validation for HTTP handlers and normal functions, and require explicit `$mod` visibility when user-defined type names are referenced.

**Architecture:** Tighten the parser so return annotations only accept a single type name, then centralize runtime return validation so both `$fn` and HTTP handlers use the same rules for built-in and user-defined types. Finally, migrate `my-app`, spec fixtures, and docs so all examples follow the new return-type model.

**Tech Stack:** Rust, Zun parser/runtime, Cargo integration tests, spec fixtures, Markdown docs

---

### Task 1: Write Failing Parser Tests For `JSON<T>`

**Files:**
- Create: `tests/spec/invalid/syntax/function_json_generic_return_type.zun`
- Create: `tests/spec/invalid/syntax/function_json_generic_return_type.zun.error`
- Create: `tests/spec/invalid/syntax/http_json_generic_return_type.zun`
- Create: `tests/spec/invalid/syntax/http_json_generic_return_type.zun.error`

- [ ] **Step 1: Write the failing fixtures**

```zun
$fn get_user() -> JSON<User> {
    $# 1;
}
```

```text
ZUN-P...
```

```zun
$GET("/users") list_users() -> JSON<List> {
    $# [];
}
```

```text
ZUN-P...
```

- [ ] **Step 2: Run the syntax fixtures to verify they fail for the right reason**

Run: `cargo test --test integration_suite -- --nocapture`
Expected: the new invalid syntax fixtures fail because parser still accepts `JSON<T>`

- [ ] **Step 3: Commit the red fixtures**

```bash
git add tests/spec/invalid/syntax/function_json_generic_return_type.zun tests/spec/invalid/syntax/function_json_generic_return_type.zun.error tests/spec/invalid/syntax/http_json_generic_return_type.zun tests/spec/invalid/syntax/http_json_generic_return_type.zun.error
git commit -m "test: add failing fixtures for generic return annotations"
```

### Task 2: Reject Generic Return Annotations In The Parser

**Files:**
- Modify: `crates/zun-frontend/src/parser/stmt/functions.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/http.rs`
- Modify: `crates/zun-frontend/src/parser/literal.rs`
- Test: `tests/spec/invalid/syntax/function_json_generic_return_type.zun`
- Test: `tests/spec/invalid/syntax/http_json_generic_return_type.zun`

- [ ] **Step 1: Remove `<...>` return-type parsing and replace it with a direct syntax error**

```rust
let return_type = if !self.at_end() && self.peek().typ == Type::Arrow {
    self.advance();
    if self.at_end() || self.peek().typ != Type::Ident {
        return Err(Error::InvalidStatement(None));
    }
    let ty = self.advance().literal.clone();
    if !self.at_end() && self.peek().typ == Type::Lt {
        return Err(Error::Parse(crate::error::ParseError {
            message: "generic return types like JSON<T> are not supported".to_string(),
            line: 1,
            column: 1,
            found: Some("<".to_string()),
            expected: Some("single type name".to_string()),
        }));
    }
    Some(ty)
} else {
    None
};
```

- [ ] **Step 2: Run the new syntax fixtures**

Run: `cargo test --test integration_suite -- --nocapture`
Expected: the two new invalid syntax fixtures now pass

- [ ] **Step 3: Commit the parser change**

```bash
git add crates/zun-frontend/src/parser/stmt/functions.rs crates/zun-frontend/src/parser/stmt/http.rs crates/zun-frontend/src/parser/literal.rs tests/spec/invalid/syntax/function_json_generic_return_type.zun tests/spec/invalid/syntax/function_json_generic_return_type.zun.error tests/spec/invalid/syntax/http_json_generic_return_type.zun tests/spec/invalid/syntax/http_json_generic_return_type.zun.error
git commit -m "feat: reject generic return annotations"
```

### Task 3: Write Failing Runtime Tests For HTTP Return Validation

**Files:**
- Create: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun`
- Create: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun.error`
- Create: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun`
- Create: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Add one fixture where HTTP returns the wrong built-in type**

```zun
$GET("/users/:id") get_user(id) -> Int {
    $# {"id": id};
}
```

- [ ] **Step 2: Add one fixture where HTTP references a user type without explicit import**

```zun
$GET("/users/:id") get_user(id) -> User {
    $# {"id": id};
}
```

- [ ] **Step 3: Add an integration test that boots a route with `-> Int` and expects a runtime error on request**

```rust
#[test]
fn http_handler_return_type_is_validated_at_request_time() {
    // build temp project
    // GET /users/1 with -> Int but returns {"id": "1"}
    // expect HTTP 500 and error body mentioning return type mismatch
}
```

- [ ] **Step 4: Run the targeted tests to verify they fail for the expected reason**

Run: `cargo test --test integration_suite http_handler_return_type_is_validated_at_request_time -- --nocapture`
Expected: FAIL because handlers still bypass return validation

- [ ] **Step 5: Commit the red tests**

```bash
git add tests/spec/invalid/http/http_handler_return_type_mismatch.zun tests/spec/invalid/http/http_handler_return_type_mismatch.zun.error tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error tests/integration_suite.rs
git commit -m "test: add failing http return validation coverage"
```

### Task 4: Centralize Return Validation For Functions And HTTP Handlers

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/runtime/http.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Test: `tests/spec/invalid/http/http_handler_return_type_mismatch.zun`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Extract return-type validation into a shared helper usable from `$fn` and HTTP**

```rust
pub fn validate_declared_return_type(
    name: &str,
    expected_type: Option<&str>,
    value: Option<&ZunValue>,
    context: &RuntimeContext,
    visible_types: impl Fn(&str) -> bool,
) -> Result<(), Error> {
    // built-in type checks
    // user-defined type checks
}
```

- [ ] **Step 2: Make `call_fn` and `call_module_fn` delegate to the shared helper**

```rust
validate_declared_return_type(
    &fn_def.name,
    fn_def.return_type.as_deref(),
    val.as_ref(),
    context,
    |_| true,
)?;
```

- [ ] **Step 3: Pass route name, route return type, and module-visible type check into HTTP execution**

```rust
validate_declared_return_type(
    &route.name,
    route.return_type.as_deref(),
    result.as_ref(),
    &runtime_context,
    |type_name| route.module_env.contains_key(type_name),
)?;
```

- [ ] **Step 4: Run the targeted HTTP runtime test**

Run: `cargo test --test integration_suite http_handler_return_type_is_validated_at_request_time -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the shared runtime validation**

```bash
git add crates/zun-runtime/src/interpreter/exec/functions.rs crates/zun-runtime/src/interpreter/mod.rs crates/zun-runtime/src/runtime/http.rs tests/spec/invalid/http/http_handler_return_type_mismatch.zun tests/spec/invalid/http/http_handler_return_type_mismatch.zun.error tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error tests/integration_suite.rs
git commit -m "feat: validate http handler return types"
```

### Task 5: Enforce Explicit Visibility For User-Defined Return Types

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/interpreter/value.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/modules.rs`
- Test: `tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun`

- [ ] **Step 1: Decide the visibility source for user-defined types**

```rust
let visible_user_type = visible_types(expected_type) || context.lookup_type(expected_type).is_some();
if !visible_user_type {
    return Err(Error::Interpreter(format!(
        "function '{}' references unknown return type '{}'",
        name, expected_type
    )));
}
```

- [ ] **Step 2: Require actual `TypedInstance` name equality for user-defined return values**

```rust
match value {
    Some(ZunValue::TypedInstance { type_name, .. }) if type_name == expected_type => Ok(()),
    Some(other) => Err(Error::Interpreter(format!(
        "function '{}' expects return type '{}' but got '{}'",
        name,
        expected_type,
        other.type_name()
    ))),
    None => Err(...),
}
```

- [ ] **Step 3: Run the unknown-type and mismatch fixtures**

Run: `cargo test --test integration_suite -- --nocapture`
Expected: the new invalid fixtures pass with explicit errors

- [ ] **Step 4: Commit the visibility enforcement**

```bash
git add crates/zun-runtime/src/interpreter/exec/functions.rs crates/zun-runtime/src/interpreter/value.rs crates/zun-runtime/src/interpreter/exec/modules.rs tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun.error
git commit -m "feat: require explicit user type visibility in return annotations"
```

### Task 6: Migrate `my-app` To The New Return-Type Rules

**Files:**
- Modify: `my-app/routers/users_routers.zun`
- Modify: `my-app/services/users_services.zun`
- Modify: `my-app/data/users_data.zun`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Remove `JSON<T>` from `my-app` and add explicit `$mod data.users_data;` where `User` is referenced**

```zun
$mod data.users_data;
$mod services.users_services;

$GET("/users") getUsers() -> JSON {
    $# users_services.find_all_users();
}

$GET("/users/:id") getUserById(id) -> User {
    $# users_services.find_user_by_id(id);
}
```

- [ ] **Step 2: Resolve the not-found path so it does not violate `-> User`**

```zun
$fn find_user_by_id(id) -> User {
    $for user in all_users() {
        $if user.id.to_str() == id {
            $# user;
        }
    }
    $throw "user not found";
}
```

- [ ] **Step 3: Update the integration test expectations for not-found behavior if needed**

```rust
let response = http_get(&base_url, "/v1/api/users/999");
assert_eq!(response.status, 500);
assert!(response.body.contains("user not found"));
```

- [ ] **Step 4: Run the `my-app` integration test**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`
Expected: PASS with the new route/service contract

- [ ] **Step 5: Commit the app migration**

```bash
git add my-app/router/users_routers.zun my-app/services/users_services.zun my-app/data/users_data.zun tests/integration_suite.rs
git commit -m "refactor: migrate my-app to simplified return annotations"
```

### Task 7: Sync Spec And Reference Docs

**Files:**
- Modify: `docs/spec/syntax.md`
- Modify: `docs/spec/semantics.md`
- Modify: `docs/reference/functions.md`
- Modify: `docs/reference/http.md`
- Modify: `docs/reference/types.md`
- Modify: `docs/guide/15-http-basics.md`
- Modify: `docs/CHANGELOG.md`

- [ ] **Step 1: Remove all `JSON<T>` examples and replace them with `JSON`, `List`, or user-defined type names**

```md
- 想表达 JSON 响应格式：写 `-> JSON`
- 想表达返回用户类型：写 `-> User`
- `JSON<User>`、`JSON<List>` 不再支持
```

- [ ] **Step 2: Document the new explicit-import rule for user-defined types**

```md
如果某个文件要在返回类型位置写 `User`，则必须先通过 `$mod` 导入定义该类型的模块。
```

- [ ] **Step 3: Document that HTTP handlers now run the same return validation as `$fn`**

```md
HTTP handler 的 `-> Type` 会在请求执行时进行运行时校验；不匹配时返回运行时错误。
```

- [ ] **Step 4: Run a grep to verify `JSON<` examples are gone from user-facing docs**

Run: `rg -n "JSON<" docs my-app tests -g '!docs/guide/book/**'`
Expected: only historical plan/spec files may remain

- [ ] **Step 5: Commit the doc sync**

```bash
git add docs/spec/syntax.md docs/spec/semantics.md docs/reference/functions.md docs/reference/http.md docs/reference/types.md docs/guide/15-http-basics.md docs/CHANGELOG.md
git commit -m "docs: remove generic return annotation guidance"
```

### Task 8: Final Verification

**Files:**
- Modify: `tests/integration_suite.rs`
- Modify: `tests/spec/invalid/syntax/*.zun*`
- Modify: `tests/spec/invalid/http/*.zun*`
- Modify: `my-app/*.zun`

- [ ] **Step 1: Run the focused regression suite**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the full integration suite**

Run: `cargo test --test integration_suite -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run the full project tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 4: Commit the verified final state**

```bash
git add .
git commit -m "feat: simplify return type semantics"
```
