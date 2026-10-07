# Return Type `TypeExpr` Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate function and HTTP handler declared return types from string-backed annotations to `TypeExpr`, while preserving current runtime behavior.

**Architecture:** Reuse the existing frontend `TypeExpr` parser and thread `Option<TypeExpr>` through AST, runtime function metadata, and HTTP route metadata. Update the shared return validator to consume `TypeExpr` directly on the main execution path, but keep the legacy runtime string parser available for any remaining non-AST compatibility uses.

**Tech Stack:** Rust, Zun frontend/runtime, Cargo unit tests, Cargo integration tests

---

## Research Summary

- [`crates/zun-frontend/src/ast/ast.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/ast/ast.rs) still stores function, closure, and HTTP handler return types as `Option<String>`.
- [`crates/zun-frontend/src/parser/stmt/functions.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/stmt/functions.rs) and [`crates/zun-frontend/src/parser/stmt/http.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/stmt/http.rs) still parse return types with identifier/string logic instead of the shared `parse_type_expr`.
- [`crates/zun-runtime/src/interpreter/exec/functions.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/exec/functions.rs) validates declared returns through `Option<&str>` and bridges to `TypeExpr` at runtime.
- [`crates/zun-runtime/src/interpreter/exec/http.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/runtime/http.rs) and related metadata carriers still expect string-backed `return_type`.

## File Map

- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/functions.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/http.rs`
- Modify: `crates/zun-frontend/src/parser/literal.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/value.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/literals.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/http.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Modify: `crates/zun-runtime/src/runtime/http.rs`

### Task 1: Lock Return-Type Behavior With Compiler-Driven Test Updates

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`

- [ ] **Step 1: Update the existing return-validator unit tests to express expectations as `TypeExpr`**

```rust
use crate::ast::TypeExpr;

validate_declared_return_type(
    "function",
    "f",
    Some(&TypeExpr::Named("Int".to_string())),
    Some(&value),
    &context,
)
```

- [ ] **Step 2: Update any runtime metadata fixtures or helper constructors that still build string-backed return types**

```rust
return_type: Some(TypeExpr::Named("String".to_string())),
```

- [ ] **Step 3: Run the return-validator unit tests before implementation**

Run: `cargo test -p zun-runtime shared_return_type_validator -- --nocapture`
Expected: FAIL to compile or fail tests because the implementation still expects `Option<&str>`

### Task 2: Migrate AST And Parser Return Types To `TypeExpr`

**Files:**
- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/functions.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/http.rs`
- Modify: `crates/zun-frontend/src/parser/literal.rs`

- [ ] **Step 1: Change function-like AST nodes to store `Option<TypeExpr>`**

```rust
pub return_type: Option<TypeExpr>,
```

- [ ] **Step 2: Reuse the existing type-expression parser for ordinary function return signatures**

```rust
let return_type = if !self.at_end() && self.peek().typ == Type::Arrow {
    self.advance();
    Some(self.parse_type_expr()?)
} else {
    None
};
```

- [ ] **Step 3: Reuse the same parser for HTTP handler return signatures**

```rust
let return_type = if !self.at_end() && self.peek().typ == Type::Arrow {
    self.advance();
    Some(self.parse_type_expr()?)
} else {
    None
};
```

- [ ] **Step 4: Run the frontend parser tests to catch AST and parser fallout**

Run: `cargo test -p zun-frontend -- --nocapture`
Expected: PASS

### Task 3: Thread `TypeExpr` Through Runtime Function And Route Metadata

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/value.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/literals.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/http.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`

- [ ] **Step 1: Update runtime structs that store declared returns to use `Option<TypeExpr>`**

```rust
pub return_type: Option<TypeExpr>,
```

- [ ] **Step 2: Keep cloning and carrying declared return types unchanged, but now as structured expressions**

```rust
return_type: lit.return_type.clone(),
```

- [ ] **Step 3: Adjust any helper logic that inspects return types so it reads `TypeExpr` instead of strings**

```rust
match return_type {
    Some(TypeExpr::Named(name)) if name == "HTML" => ...,
    _ => ...
}
```

- [ ] **Step 4: Run the runtime crate tests that exercise metadata propagation**

Run: `cargo test -p zun-runtime -- --nocapture`
Expected: PASS except for any pre-existing unrelated failures

### Task 4: Switch Shared Return Validation To `TypeExpr`

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `crates/zun-runtime/src/runtime/http.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/http.rs`

- [ ] **Step 1: Change the shared return validator signature to accept `Option<&TypeExpr>`**

```rust
pub fn validate_declared_return_type(
    kind: &str,
    name: &str,
    declared: Option<&TypeExpr>,
    actual: Option<&ZunValue>,
    context: &RuntimeContext,
) -> Result<(), Error>
```

- [ ] **Step 2: Remove the function/HTTP main-path dependency on string parsing and validate directly against `TypeExpr`**

```rust
if let Err(error) = validate_value_against_type_expr(expected_type, actual_value, context) {
    ...
}
```

- [ ] **Step 3: Update HTTP probe defaults to inspect structured return types without changing current semantics**

```rust
fn default_probe_body(return_type: Option<&TypeExpr>) -> ZunValue
```

- [ ] **Step 4: Run the focused runtime tests for return validation**

Run: `cargo test -p zun-runtime shared_return_type_validator -- --nocapture`
Expected: PASS

### Task 5: Run Focused Regressions And Confirm No Behavior Drift

**Files:**
- Modify: files above only if test-driven fixes are needed

- [ ] **Step 1: Run the typed runtime regressions most likely to touch return validation**

Run: `cargo test --test integration_suite strict_type_construction_fixture_passes optional_null_and_list_construction_fixture_passes typed_list_ http_handler_user_return_type_bootstraps_in_test_mode http_handler_return_type_mismatch_fails_in_test_mode_bootstrap -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the frontend parser suite one more time**

Run: `cargo test -p zun-frontend -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run the runtime validator tests one more time**

Run: `cargo test -p zun-runtime shared_return_type_validator type_validation -- --nocapture`
Expected: PASS
