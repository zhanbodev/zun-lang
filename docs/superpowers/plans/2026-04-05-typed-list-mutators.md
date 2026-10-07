# Typed List Mutators Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce `List<T>` element checks for list mutator methods that add new elements, specifically `push` and `insert`.

**Architecture:** Keep list builtins simple and add type enforcement in the mutable method execution path inside the interpreter. Reuse the existing `TypeExpr` validator by introducing one helper that checks a receiver variable's declared `List<T>` item type before dispatching `push` or `insert`.

**Tech Stack:** Rust, Zun runtime, Cargo unit/integration tests, Markdown fixtures

---

## Research Summary

- [`crates/zun-runtime/src/interpreter/exec/variables.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/exec/variables.rs) already routes mutable method calls through `handle_mut_method_call` and requires the receiver to be a direct variable reference, so the variable name is available before builtin dispatch.
- [`crates/zun-runtime/src/interpreter/builtins/list.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/builtins/list.rs) implements `push` and `insert` by directly writing values into the backing `Vec<ZunValue>` with no type checks.
- [`crates/zun-runtime/src/interpreter/type_validation.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/type_validation.rs) already exposes `validate_value_against_type_expr`, which can validate a single element against the `T` inside `List<T>`.
- [`tests/integration_suite.rs`](/Users/liangzhanbo/CodeStudio/Zun/tests/integration_suite.rs) is the right place to lock runtime behavior for successful and failing mutator calls.

## File Map

- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs`
- Modify: `tests/integration_suite.rs`

### Task 1: Lock Mutator Behavior With Red Tests

**Files:**
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Add one passing integration test for typed `push` and one for typed `insert`**

```rust
#[test]
fn typed_list_push_accepts_matching_item_type() {
    let code = r#"
$Type Post {
    title: String
}

$ posts: List<Post> = [];
posts.push(Post { title: "Hello" });
$>> posts[0].title;
"#;

    let output = run_zun_program(code).expect("program should succeed");
    assert_eq!(output, "Hello\n");
}

#[test]
fn typed_list_insert_accepts_matching_item_type() {
    let code = r#"
$Type Post {
    title: String
}

$ posts: List<Post> = [];
posts.insert(0, Post { title: "Hello" });
$>> posts[0].title;
"#;

    let output = run_zun_program(code).expect("program should succeed");
    assert_eq!(output, "Hello\n");
}
```

- [ ] **Step 2: Add one failing integration test for typed `push`, one for typed `insert`, and one dynamic-list regression**

```rust
#[test]
fn typed_list_push_rejects_wrong_item_type() {
    let code = r#"
$Type Post {
    title: String
}

$Type User {
    name: String
}

$ posts: List<Post> = [];
posts.push(User { name: "Alice" });
"#;

    let err = run_zun_program(code).expect_err("program should fail");
    assert!(err.to_string().contains("list method 'push' for 'posts' expects item type 'Post', got 'User'"));
}

#[test]
fn typed_list_insert_rejects_wrong_item_type() {
    let code = r#"
$Type Post {
    title: String
}

$Type User {
    name: String
}

$ posts: List<Post> = [];
posts.insert(0, User { name: "Alice" });
"#;

    let err = run_zun_program(code).expect_err("program should fail");
    assert!(err.to_string().contains("list method 'insert' for 'posts' expects item type 'Post', got 'User'"));
}

#[test]
fn dynamic_list_mutators_remain_untyped() {
    let code = r#"
$ list = [];
list.push(1);
list.insert(1, "two");
$>> list[0];
$>> list[1];
"#;

    let output = run_zun_program(code).expect("program should succeed");
    assert_eq!(output, "1\ntwo\n");
}
```

- [ ] **Step 3: Run the new targeted tests to confirm they fail before implementation**

Run: `cargo test --test integration_suite typed_list_push_ typed_list_insert_ dynamic_list_mutators_remain_untyped -- --nocapture`
Expected: FAIL because `push` and `insert` do not yet enforce typed `List<T>` writes

- [ ] **Step 4: Commit the red test lock**

```bash
git add tests/integration_suite.rs
git commit -m "test: lock typed list mutator behavior"
```

### Task 2: Enforce `push` And `insert` At The Mutable Call Boundary

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs`

- [ ] **Step 1: Add one helper that validates mutator items for receiver variables declared as `List<T>`**

```rust
fn validate_list_method_item_type(
    var_name: &str,
    method: &str,
    args: &[ZunValue],
    state: &ProgramState,
    span: crate::ast::Span,
    context: &RuntimeContext,
) -> Result<(), Error> {
    let Some(type_expr) = state.type_env.get(var_name) else {
        return Ok(());
    };

    let (item_type, value) = match (method, type_expr, args) {
        ("push", TypeExpr::List(item_type), [value]) => (item_type.as_ref(), value),
        ("insert", TypeExpr::List(item_type), [_, value]) => (item_type.as_ref(), value),
        _ => return Ok(()),
    };

    validate_value_against_type_expr(item_type, value, context)
        .map_err(|err| type_list_method_error(var_name, method, item_type, err, span))
}
```

- [ ] **Step 2: Add one dedicated error formatter for typed list mutators**

```rust
fn type_list_method_error(
    var_name: &str,
    method: &str,
    item_type: &TypeExpr,
    error: TypeValidationError,
    span: crate::ast::Span,
) -> Error {
    let message = match error {
        TypeValidationError::Mismatch { actual_type, .. } => format!(
            "list method '{method}' for '{var_name}' expects item type '{}', got '{actual_type}'",
            type_expr_name(item_type)
        ),
        other => format!(
            "list method '{method}' for '{var_name}' expects item type '{}': {}",
            type_expr_name(item_type),
            type_validation_detail(other)
        ),
    };

    Error::Diagnostic(
        crate::diagnostics::Diagnostic::error(
            codes::RUNTIME_TYPE_MISMATCH,
            message,
        )
        .with_span(span),
    )
}
```

- [ ] **Step 3: Call the helper from `handle_mut_method_call` before `dispatch_mut`**

```rust
validate_list_method_item_type(
    &var_name,
    &call.method,
    &arg_vals,
    state,
    crate::ast::Span::from_token(call.method_token.start),
    context,
)?;

match super::super::builtins::dispatch_mut(receiver, &call.method, &arg_vals) {
    Ok(_) => Flow::Normal,
    Err(err) => Flow::Err(err),
}
```

- [ ] **Step 4: Run the targeted integration tests again**

Run: `cargo test --test integration_suite typed_list_push_ typed_list_insert_ dynamic_list_mutators_remain_untyped -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the runtime enforcement**

```bash
git add crates/zun-runtime/src/interpreter/exec/variables.rs
git commit -m "feat: validate typed list mutator writes"
```

### Task 3: Regression Sweep

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs` if any test-driven cleanup is needed
- Modify: `tests/integration_suite.rs` if error assertions need tightening after real output is observed

- [ ] **Step 1: Run the broader typed-runtime regressions**

Run: `cargo test --test integration_suite typed_list_ strict_type_construction_fixture_passes optional_null_and_list_construction_fixture_passes -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the runtime unit tests that cover shared type validation**

Run: `cargo test -p zun-runtime type_validation -- --nocapture`
Expected: PASS

- [ ] **Step 3: If needed, tighten assertions to match exact observed error text and rerun the targeted suite**

```rust
assert!(err.to_string().contains("list method 'push' for 'posts' expects item type 'Post', got 'User'"));
```

Run: `cargo test --test integration_suite typed_list_push_ typed_list_insert_ dynamic_list_mutators_remain_untyped -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit the verified finish**

```bash
git add crates/zun-runtime/src/interpreter/exec/variables.rs tests/integration_suite.rs
git commit -m "test: verify typed list mutator regressions"
```
