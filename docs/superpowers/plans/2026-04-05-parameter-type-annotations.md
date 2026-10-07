# Parameter Type Annotations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `TypeExpr`-backed parameter annotations for ordinary functions, HTTP handlers, and anonymous functions, with runtime validation at argument bind time.

**Architecture:** Introduce one shared `FnParam` AST model for function-like declarations, then reuse the existing `TypeExpr` parser and validator so all parameter-bearing call paths bind through one typed helper. Keep variadic parameters name-only in this round and avoid implicit `Map/JSON -> $Type` coercion.

**Tech Stack:** Rust, Zun frontend/runtime, Cargo unit tests, Cargo integration tests

---

## Research Summary

- [`crates/zun-frontend/src/ast/ast.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/ast/ast.rs) still stores function-like parameters as `Vec<String>`, which cannot carry parameter type annotations.
- [`crates/zun-frontend/src/parser/stmt/functions.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/stmt/functions.rs), [`crates/zun-frontend/src/parser/stmt/http.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/stmt/http.rs), and [`crates/zun-frontend/src/parser/literal.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/literal.rs) each parse parameters independently and must be aligned on one grammar.
- [`crates/zun-runtime/src/interpreter/exec/functions.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/exec/functions.rs) currently binds arguments directly into env without type checks; it is the right shared insertion point for ordinary, module, and anonymous function calls.
- [`crates/zun-runtime/src/runtime/http.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/runtime/http.rs) binds route params directly into env and will need the same parameter helper to validate HTTP handler parameters consistently.

## File Map

- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/functions.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/http.rs`
- Modify: `crates/zun-frontend/src/parser/literal.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/runtime/http.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/literals.rs`
- Modify: `tests/integration_suite.rs`

### Task 1: Lock Parameter Annotation Behavior With Red Integration Tests

**Files:**
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Add one passing and one failing test for ordinary function parameters**

```rust
#[test]
fn typed_function_parameter_accepts_matching_value() {
    let project_dir = write_temp_project(
        "zun-typed-param-fn-ok",
        "name = \"typed-param-fn-ok\"\nversion = \"0.1.0\"\nentry = \"main.zun\"\n",
        &[(
            "main.zun",
            r#"$fn echo_id(id: Int) -> Int {
    $# id;
}

$>> echo_id(7);
"#,
        )],
    );

    let outcome = run_program_at_path(&project_dir.join("main.zun"), RuntimeMode::Test);
    assert!(outcome.error.is_none(), "function call should succeed: {:?}", outcome.error);
    assert_eq!(outcome.stdout, "7\n");
    fs::remove_dir_all(&project_dir).expect("cleanup");
}

#[test]
fn typed_function_parameter_rejects_wrong_value() {
    let project_dir = write_temp_project(
        "zun-typed-param-fn-bad",
        "name = \"typed-param-fn-bad\"\nversion = \"0.1.0\"\nentry = \"main.zun\"\n",
        &[(
            "main.zun",
            r#"$fn echo_id(id: Int) -> Int {
    $# id;
}

$>> echo_id("bad");
"#,
        )],
    );

    let outcome = run_program_at_path(&project_dir.join("main.zun"), RuntimeMode::Test);
    let error = outcome.error.expect("function call should fail");
    assert!(error.contains("parameter 'id' expects type 'Int', got 'String'"), "error={error}");
    fs::remove_dir_all(&project_dir).expect("cleanup");
}
```

- [ ] **Step 2: Add one passing and one failing test for HTTP handler parameters**

```rust
#[test]
fn typed_http_handler_parameter_accepts_matching_value() {
    let outcome = run_fixture("fixtures/http/typed_param_handler_ok.zun", RuntimeMode::Test);
    assert!(outcome.error.is_none(), "http handler should bootstrap");
}

#[test]
fn typed_http_handler_parameter_rejects_wrong_value() {
    let outcome = run_fixture("fixtures/http/typed_param_handler_bad.zun", RuntimeMode::Test);
    let error = outcome.error.expect("http handler should fail");
    assert!(error.contains("parameter 'id' expects type 'Int', got 'String'"), "error={error}");
}
```

- [ ] **Step 3: Add one passing and one failing test for anonymous function parameters, plus a `$Type` parameter rejection**

```rust
#[test]
fn typed_fn_literal_parameter_accepts_matching_value() {
    let project_dir = write_temp_project(
        "zun-typed-param-literal-ok",
        "name = \"typed-param-literal-ok\"\nversion = \"0.1.0\"\nentry = \"main.zun\"\n",
        &[(
            "main.zun",
            r#"$ echo = $fn(user_id: Int) -> Int {
    $# user_id;
};

$>> echo(9);
"#,
        )],
    );

    let outcome = run_program_at_path(&project_dir.join("main.zun"), RuntimeMode::Test);
    assert!(outcome.error.is_none(), "fn literal should succeed: {:?}", outcome.error);
    assert_eq!(outcome.stdout, "9\n");
    fs::remove_dir_all(&project_dir).expect("cleanup");
}

#[test]
fn typed_function_parameter_rejects_bare_map_for_user() {
    let project_dir = write_temp_project(
        "zun-typed-param-user-bad",
        "name = \"typed-param-user-bad\"\nversion = \"0.1.0\"\nentry = \"main.zun\"\n",
        &[(
            "main.zun",
            r#"$Type User {
    id: Int
}

$fn show_user(user: User) -> Int {
    $# user.id;
}

$>> show_user({"id": 1});
"#,
        )],
    );

    let outcome = run_program_at_path(&project_dir.join("main.zun"), RuntimeMode::Test);
    let error = outcome.error.expect("typed user parameter should fail");
    assert!(error.contains("parameter 'user' expects type 'User', got 'Map'"), "error={error}");
    fs::remove_dir_all(&project_dir).expect("cleanup");
}
```

- [ ] **Step 4: Run the new targeted tests to confirm they fail before implementation**

Run: `cargo test --test integration_suite typed_function_parameter_ typed_http_handler_parameter_ typed_fn_literal_parameter_ -- --nocapture`
Expected: FAIL because parameter annotations are not parsed or enforced yet

### Task 2: Introduce Shared `FnParam` AST And Parser Support

**Files:**
- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/functions.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/http.rs`
- Modify: `crates/zun-frontend/src/parser/literal.rs`

- [ ] **Step 1: Add `FnParam` and migrate function-like AST nodes away from `Vec<String>`**

```rust
#[derive(Debug, Clone)]
pub struct FnParam {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
}

pub struct FnDeclStmt {
    pub params: Vec<FnParam>,
    pub variadic_param: Option<String>,
    ...
}
```

- [ ] **Step 2: Parse ordinary function parameters with optional `: TypeExpr` annotations**

```rust
let mut params: Vec<FnParam> = Vec::new();

params.push(FnParam {
    name,
    type_annotation,
});
```

- [ ] **Step 3: Parse HTTP handler parameters with the same parameter grammar**

```rust
let mut params: Vec<FnParam> = Vec::new();
```

- [ ] **Step 4: Parse anonymous function parameters with the same parameter grammar and keep variadic name-only**

```rust
if parser.tokens[parser.pos].typ == Type::Spread {
    variadic_param = Some(parser.tokens[parser.pos + 1].literal.clone());
    ...
}
```

- [ ] **Step 5: Run the frontend parser suite**

Run: `cargo test -p zun-frontend -- --nocapture`
Expected: PASS

### Task 3: Add Shared Runtime Parameter Binding Validation

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/literals.rs`

- [ ] **Step 1: Add one helper that binds typed parameters into local state**

```rust
fn bind_typed_params(
    params: &[crate::ast::FnParam],
    args: &[ZunValue],
    state: &mut ProgramState,
    context: &RuntimeContext,
) -> Result<(), Error> {
    for (param, arg) in params.iter().zip(args.iter()) {
        if let Some(type_expr) = &param.type_annotation {
            validate_value_against_type_expr(type_expr, arg, context)
                .map_err(|error| parameter_type_error(&param.name, type_expr, error))?;
        }
        state.insert_env(param.name.clone(), arg.clone());
    }
    Ok(())
}
```

- [ ] **Step 2: Add one shared parameter error formatter**

```rust
fn parameter_type_error(
    param_name: &str,
    type_expr: &crate::ast::TypeExpr,
    error: TypeValidationError,
) -> Error {
    let message = match error {
        TypeValidationError::Mismatch { actual_type, .. } => format!(
            "parameter '{param_name}' expects type '{}', got '{actual_type}'",
            type_expr_name(type_expr)
        ),
        other => format!(
            "parameter '{param_name}' expects type '{}': {}",
            type_expr_name(type_expr),
            type_validation_detail(other)
        ),
    };

    Error::Interpreter(message)
}
```

- [ ] **Step 3: Replace direct param insertion in `call_fn` and `call_module_fn` with the shared helper**

```rust
bind_typed_params(&fn_decl.params, args, &mut local_state, context)?;
```

- [ ] **Step 4: Ensure anonymous function literals keep carrying the migrated parameter metadata**

```rust
params: lit.params.clone(),
```

- [ ] **Step 5: Run the focused runtime tests after the helper lands**

Run: `cargo test -p zun-runtime shared_return_type_validator type_validation -- --nocapture`
Expected: PASS

### Task 4: Validate HTTP Handler Parameters And Run Focused Regressions

**Files:**
- Modify: `crates/zun-runtime/src/runtime/http.rs`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Rework HTTP handler parameter binding so annotated params validate after path/query extraction**

```rust
for param in &route.params {
    if let Some(value) = extracted.get(&param.name) {
        if let Some(type_expr) = &param.type_annotation {
            validate_value_against_type_expr(type_expr, value, runtime_context)
                .map_err(|error| parameter_type_error(&param.name, type_expr, error).to_string())?;
        }
        state.insert_env(param.name.clone(), value.clone());
    }
}
```

- [ ] **Step 2: Run the targeted integration tests for parameter annotations**

Run: `cargo test --test integration_suite typed_function_parameter_ typed_http_handler_parameter_ typed_fn_literal_parameter_ -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run the broader typed regressions to confirm no drift**

Run: `cargo test --test integration_suite typed_list_ strict_type_construction_fixture_passes optional_null_and_list_construction_fixture_passes http_handler_user_return_type_bootstraps_in_test_mode http_handler_return_type_mismatch_fails_in_test_mode_bootstrap -- --nocapture`
Expected: PASS

- [ ] **Step 4: Keep variadic parameter behavior unchanged and untyped in this round**

```rust
if let Some(var_param) = &fn_decl.variadic_param {
    let extra_args: Vec<ZunValue> = args[min_params..].to_vec();
    local_state.insert_env(var_param.clone(), ZunValue::List(extra_args));
}
```
