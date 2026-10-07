# `$Type` Tightening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `$Type` behave like a strict nominal runtime type in Zun phase 1 by enforcing construction, field assignment, and declared return validation consistently in `run`, `serve`, and `test`.

**Architecture:** Introduce one shared runtime validator for `$Type` and `List<T>` matching, then route struct construction, typed field assignment, and function / HTTP declared return checks through that validator. Keep parser syntax unchanged for phase 1, but allow `$Type` fields to reference other user-defined types by name and document phase 2 separately.

**Tech Stack:** Rust, Zun frontend/runtime, Cargo integration tests, spec fixtures, Markdown docs

---

## Research Summary

- [`crates/zun-runtime/src/interpreter/eval/constructors.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/eval/constructors.rs) currently only checks that `ctor.type_name` exists and then blindly builds `TypedInstance`.
- [`crates/zun-runtime/src/interpreter/exec/variables.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/exec/variables.rs) checks some built-in field types, but it still allows undeclared-field drift and treats user-defined field types as pass-through.
- [`crates/zun-runtime/src/interpreter/exec/functions.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/exec/functions.rs) already validates declared returns for `$fn` / HTTP, but that logic only reasons about top-level type names and does not serve constructor / assignment paths.
- [`crates/zun-frontend/src/parser/stmt/declarations.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/stmt/declarations.rs) only parses `$Type` field types as one identifier plus optional `?`, so phase 1 should not attempt `List<User>` field syntax.
- Existing fixtures such as [`tests/spec/valid/data/type_construction.zun`](/Users/liangzhanbo/CodeStudio/Zun/tests/spec/valid/data/type_construction.zun) currently rely on permissive construction and will need to be updated once missing required fields start failing.

## File Map

- Create: `crates/zun-runtime/src/interpreter/type_validation.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/constructors.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `tests/spec/valid/data/type_construction.zun`
- Modify: `tests/spec/invalid/data/field_type_mismatch.zun`
- Modify: `tests/spec/invalid/data/field_type_mismatch.zun.error`
- Create: `tests/spec/invalid/data/type_missing_required_field.zun`
- Create: `tests/spec/invalid/data/type_missing_required_field.zun.error`
- Create: `tests/spec/invalid/data/type_unknown_field.zun`
- Create: `tests/spec/invalid/data/type_unknown_field.zun.error`
- Create: `tests/spec/invalid/data/type_nested_user_field_mismatch.zun`
- Create: `tests/spec/invalid/data/type_nested_user_field_mismatch.zun.error`
- Modify: `tests/integration_suite.rs`
- Modify: `docs/guide/09-gradual-typing.md`
- Modify: `docs/reference/types.md`

### Task 1: Lock The Phase-1 Behavior With Failing Fixtures

**Files:**
- Modify: `tests/spec/valid/data/type_construction.zun`
- Modify: `tests/spec/invalid/data/field_type_mismatch.zun`
- Modify: `tests/spec/invalid/data/field_type_mismatch.zun.error`
- Create: `tests/spec/invalid/data/type_missing_required_field.zun`
- Create: `tests/spec/invalid/data/type_missing_required_field.zun.error`
- Create: `tests/spec/invalid/data/type_unknown_field.zun`
- Create: `tests/spec/invalid/data/type_unknown_field.zun.error`
- Create: `tests/spec/invalid/data/type_nested_user_field_mismatch.zun`
- Create: `tests/spec/invalid/data/type_nested_user_field_mismatch.zun.error`
- Test: `tests/spec_suite.rs`

- [ ] **Step 1: Update the valid construction fixture so it matches the stricter shape rules**

```zun
$Type User {
    id: Int
    name: String
    email: String
    @HIDE password: String
}

$Type Post {
    id: Int
    title: String
    content: String
    author: User
}

$ user = User {
    id: 1,
    name: "John",
    email: "john@example.com",
    password: "secret",
};

$ post = Post {
    id: 7,
    title: "Hello",
    content: "World",
    author: user,
};

$>> user.name;
$>> user.password;
$>> post.content;
post.author = user;
$>> post.author.email;
```

- [ ] **Step 2: Keep the field-assignment mismatch fixture minimal and explicit**

```zun
$Type Post {
    author_id: Int
}

$ post = Post {
    author_id: 1,
};

post.author_id = "wrong";
```

```text
ZUN-R011
```

- [ ] **Step 3: Add the missing-required-field fixture**

```zun
$Type User {
    id: Int
    name: String
}

$ user = User {
    id: 1,
};
```

```text
ZUN-R011
```

- [ ] **Step 4: Add the unknown-field fixture**

```zun
$Type User {
    id: Int
}

$ user = User {
    id: 1,
    nickname: "neo",
};
```

```text
ZUN-R011
```

- [ ] **Step 5: Add the nested user-type mismatch fixture**

```zun
$Type User {
    id: Int
}

$Type Post {
    author: User
}

$ post = Post {
    author: {
        id: 1,
    },
};
```

```text
ZUN-R011
```

- [ ] **Step 6: Run the spec suite to verify the new fixtures fail before implementation**

Run: `cargo test --test spec_suite valid_spec_samples_produce_expected_output invalid_spec_samples_fail_with_expected_error -- --nocapture`
Expected: FAIL because the new invalid fixtures still construct successfully under the permissive runtime

- [ ] **Step 7: Commit the fixture lock once the failures are confirmed**

```bash
git add tests/spec/valid/data/type_construction.zun tests/spec/invalid/data/field_type_mismatch.zun tests/spec/invalid/data/field_type_mismatch.zun.error tests/spec/invalid/data/type_missing_required_field.zun tests/spec/invalid/data/type_missing_required_field.zun.error tests/spec/invalid/data/type_unknown_field.zun tests/spec/invalid/data/type_unknown_field.zun.error tests/spec/invalid/data/type_nested_user_field_mismatch.zun tests/spec/invalid/data/type_nested_user_field_mismatch.zun.error
git commit -m "test: lock strict $Type phase-1 fixtures"
```

### Task 2: Extract One Shared Runtime `$Type` Validator

**Files:**
- Create: `crates/zun-runtime/src/interpreter/type_validation.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`

- [ ] **Step 1: Add a dedicated validator module instead of spreading shape checks across exec paths**

```rust
use crate::error::Error;
use crate::runtime::RuntimeContext;

use super::value::ZunValue;

pub fn validate_value_against_type(
    expected_type: &str,
    value: &ZunValue,
    context: &RuntimeContext,
    path: &str,
) -> Result<(), Error> {
    if let Some(item_type) = parse_list_item_type(expected_type) {
        let ZunValue::List(items) = value else {
            return Err(type_mismatch(path, expected_type, value));
        };

        for (index, item) in items.iter().enumerate() {
            validate_value_against_type(
                item_type,
                item,
                context,
                &format!("{path}[{index}]"),
            )?;
        }
        return Ok(());
    }

    match expected_type {
        "Int" | "Integer" => expect_int(path, value),
        "Float" => expect_float(path, value),
        "String" | "Str" => expect_string(path, value),
        "Bool" | "Boolean" => expect_bool(path, value),
        "Map" => expect_map(path, value),
        "Json" | "JSON" => expect_json_like(path, value),
        other => expect_user_type(path, other, value, context),
    }
}
```

- [ ] **Step 2: Add a shape-level helper that constructors and assignments can call directly**

```rust
pub fn validate_typed_instance_fields(
    type_name: &str,
    fields: &indexmap::IndexMap<String, ZunValue>,
    context: &RuntimeContext,
) -> Result<(), Error> {
    let shape = context.get_type(type_name).ok_or_else(|| {
        Error::Interpreter(format!("type '{type_name}' is not defined"))
    })?;

    for (stored_key, value) in fields {
        let field_name = stored_key.trim_start_matches('_');
        let field = shape
            .fields
            .iter()
            .find(|field| field.name == field_name)
            .ok_or_else(|| Error::Interpreter(format!(
                "type '{type_name}' has no field '{field_name}'"
            )))?;

        validate_value_against_type(
            &field.type_name,
            value,
            context,
            &format!("field '{field_name}' of type '{type_name}'"),
        )?;
    }

    for field in &shape.fields {
        let stored_name = if field.hidden {
            format!("_{}", field.name)
        } else {
            field.name.clone()
        };
        if !field.optional && !fields.contains_key(&stored_name) {
            return Err(Error::Interpreter(format!(
                "type '{type_name}' requires field '{}'",
                field.name
            )));
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Refactor declared return validation to delegate to the shared validator**

```rust
pub fn validate_declared_return_type(
    kind: &str,
    name: &str,
    expected_type: Option<&str>,
    value: Option<&ZunValue>,
    context: &RuntimeContext,
) -> Result<(), Error> {
    let Some(expected_type) = expected_type else {
        return Ok(());
    };
    let Some(value) = value else {
        return Err(Error::Interpreter(format!(
            "{kind} '{name}' expects return type '{expected_type}' but returned nothing"
        )));
    };

    validate_value_against_type(expected_type, value, context, &format!("{kind} '{name}'"))
}
```

- [ ] **Step 4: Add focused unit tests in the new module for user type, nested user type, and list item path failures**

```rust
#[test]
fn validate_value_against_type_rejects_map_for_user_type() {
    let mut context = RuntimeContext::new(RuntimeMode::Test, std::path::PathBuf::from("."));
    context.register_type("User", TypeShape { name: "User".into(), fields: vec![] });

    let value = ZunValue::Map(indexmap::IndexMap::new());
    let error = validate_value_against_type("User", &value, &context, "function 'f'")
        .expect_err("map should not satisfy User");

    assert!(error.to_string().contains("expects 'User'"));
}
```

- [ ] **Step 5: Run the nearest unit coverage for the new validator module**

Run: `cargo test -p zun-runtime type_validation -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit the validator extraction**

```bash
git add crates/zun-runtime/src/interpreter/type_validation.rs crates/zun-runtime/src/interpreter/mod.rs crates/zun-runtime/src/interpreter/exec/functions.rs
git commit -m "refactor: centralize $Type runtime validation"
```

### Task 3: Enforce Strict `$Type` Construction

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/eval/constructors.rs`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Make struct constructors build the stored field map and validate it before creating `TypedInstance`**

```rust
pub fn eval_struct_constructor(
    ctor: &StructConstructor,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn std::io::Write,
) -> Result<ZunValue, Error> {
    let shape = context.get_type(&ctor.type_name).cloned().ok_or_else(|| {
        Error::Diagnostic(
            crate::diagnostics::Diagnostic::error(
                codes::RUNTIME_FIELD_NOT_FOUND,
                format!("type '{}' is not defined", ctor.type_name),
            )
            .with_span(ctor.span),
        )
    })?;

    let mut fields = IndexMap::new();
    for (key, value_expr) in &ctor.fields {
        let value = eval_expr(value_expr, state, context, w, false)?;
        let field = shape.fields.iter().find(|field| field.name == *key).ok_or_else(|| {
            Error::Diagnostic(
                crate::diagnostics::Diagnostic::error(
                    codes::RUNTIME_FIELD_TYPE_MISMATCH,
                    format!("type '{}' has no field '{}'", ctor.type_name, key),
                )
                .with_span(ctor.span),
            )
        })?;
        let stored_key = if field.hidden {
            format!("_{}", key)
        } else {
            key.clone()
        };
        fields.insert(stored_key, value);
    }

    validate_typed_instance_fields(&ctor.type_name, &fields, context)?;

    Ok(ZunValue::TypedInstance {
        type_name: ctor.type_name.clone(),
        fields,
    })
}
```

- [ ] **Step 2: Add an integration test that proves a nested user field must be a typed instance, not a map**

```rust
#[test]
fn typed_constructor_rejects_nested_map_for_user_field() {
    let outcome = support::run_fixture(
        "spec/invalid/data/type_nested_user_field_mismatch.zun",
        RuntimeMode::Test,
    );

    let error = outcome.error.expect("nested map should fail");
    assert!(error.contains("field 'author' of type 'Post' expects 'User'"));
}
```

- [ ] **Step 3: Run the targeted spec and integration coverage**

Run: `cargo test --test spec_suite valid_spec_samples_produce_expected_output invalid_spec_samples_fail_with_expected_error -- --nocapture`
Expected: PASS for the new construction fixtures

Run: `cargo test --test integration_suite typed_constructor_rejects_nested_map_for_user_field -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit the constructor tightening**

```bash
git add crates/zun-runtime/src/interpreter/eval/constructors.rs tests/integration_suite.rs tests/spec/valid/data/type_construction.zun tests/spec/invalid/data/type_missing_required_field.zun tests/spec/invalid/data/type_missing_required_field.zun.error tests/spec/invalid/data/type_unknown_field.zun tests/spec/invalid/data/type_unknown_field.zun.error tests/spec/invalid/data/type_nested_user_field_mismatch.zun tests/spec/invalid/data/type_nested_user_field_mismatch.zun.error
git commit -m "feat: enforce strict $Type construction"
```

### Task 4: Enforce Strict Typed Field Assignment And Return Paths

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `tests/integration_suite.rs`
- Modify: `tests/spec/invalid/data/field_type_mismatch.zun`
- Modify: `tests/spec/invalid/data/field_type_mismatch.zun.error`

- [ ] **Step 1: Replace the ad-hoc field assignment type checks with the shared validator**

```rust
if let Some(ref expected_type) = expected_field_type {
    validate_value_against_type(
        expected_type,
        &val,
        context,
        &format!("field '{}' of type '{}'", target.method, type_name),
    )
    .map_err(as_field_assignment_diagnostic)?;
} else {
    return Flow::Err(Error::Diagnostic(
        crate::diagnostics::Diagnostic::error(
            codes::RUNTIME_FIELD_TYPE_MISMATCH,
            format!("type '{}' has no field '{}'", type_name, target.method),
        )
        .with_span(crate::ast::Span::from_token(stmt.span.start)),
    ));
}
```

- [ ] **Step 2: Add an integration test that proves undeclared-field assignment now fails**

```rust
#[test]
fn typed_instance_assignment_rejects_unknown_field() {
    let script = r#"
$Type User {
    id: Int
}

$ user = User {
    id: 1,
};

user.nickname = "neo";
"#;

    let outcome = support::run_program(script, RuntimeMode::Test);
    let error = outcome.error.expect("unknown field assignment should fail");
    assert!(error.contains("type 'User' has no field 'nickname'"));
}
```

- [ ] **Step 3: Confirm function and HTTP returns still flow through the shared validator**

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

```rust
super::functions::validate_declared_return_type(
    "http handler",
    &route.name,
    route.return_type.as_deref(),
    result.as_ref(),
    &probe_context,
)
```

- [ ] **Step 4: Run focused regression coverage for assignment and returns**

Run: `cargo test --test integration_suite typed_instance_assignment_rejects_unknown_field linked_http_router_preserves_imported_user_types -- --nocapture`
Expected: PASS

Run: `cargo test -p zun-runtime shared_return_type_validator -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the assignment and return-path tightening**

```bash
git add crates/zun-runtime/src/interpreter/exec/variables.rs crates/zun-runtime/src/interpreter/exec/functions.rs tests/integration_suite.rs tests/spec/invalid/data/field_type_mismatch.zun tests/spec/invalid/data/field_type_mismatch.zun.error
git commit -m "feat: tighten typed assignment and returns"
```

### Task 5: Update The User-Facing Docs For The New `$Type` Contract

**Files:**
- Modify: `docs/guide/09-gradual-typing.md`
- Modify: `docs/reference/types.md`

- [ ] **Step 1: Rewrite the guide copy so `$Type` is described as a nominal runtime type, not just a structure hint**

```md
`$Type` 定义的是 Zun 的名义自定义类型，不是形状相同即可互换的 Map。

- `User { ... }` 是 `User`
- `{ ... }` 只是 `Map`
- 如果函数声明 `-> User`，实际值必须是 `User { ... }`
```

- [ ] **Step 2: Add one construction example and one failure rule to the reference page**

```md
`$Type` 构造现在会在 runtime 校验：

- 未声明字段
- 缺失必填字段
- 字段类型不匹配

这意味着 `User { id: 1 }` 在 `name` 为必填字段时会直接报错。
```

- [ ] **Step 3: Run doc generation or at least the nearest doc sanity checks if the repo has none**

Run: `cargo test --test spec_suite valid_spec_samples_produce_expected_output -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit the doc updates**

```bash
git add docs/guide/09-gradual-typing.md docs/reference/types.md
git commit -m "docs: explain strict $Type nominal behavior"
```

## Phase 2 Follow-Up

Do not implement these in this plan. Track them as the next plan after phase 1 lands:

- Extend `$Type` field syntax to support `List<T>` and `List<T>?`
- Upgrade AST field type representation beyond `type_name: String`
- Support variable / constant annotations for `User` and `List<User>`
- Enforce element validation on list index writes and future append/push helpers

## Self-Review

- Spec coverage: phase-1 constructor strictness, field-assignment strictness, shared return validation, nested user-type fields, and docs updates all map to Tasks 1-5. Phase 2 items are intentionally deferred into the follow-up section.
- Placeholder scan: every task includes exact file paths, concrete snippets, exact commands, and explicit expected outcomes.
- Type consistency: the plan uses one shared helper naming scheme, `validate_value_against_type(...)` and `validate_typed_instance_fields(...)`, across constructor, assignment, and return validation steps.
