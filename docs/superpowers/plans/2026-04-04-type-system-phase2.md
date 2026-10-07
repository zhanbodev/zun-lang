# `$Type` Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend Zun’s `$Type` system with structured `TypeExpr` support for `$Type` fields and variable annotations, while enforcing `List<T>` and optional-null semantics through runtime validation and list index assignment checks.

**Architecture:** Introduce one shared `TypeExpr` model in the frontend AST and runtime environment, then migrate `$Type` field declarations and variable / constant annotations to that model before upgrading the validator to operate on structured expressions. Keep function and HTTP declared return types string-backed for now, but bridge them into `TypeExpr` at runtime so phase 2 stays scoped to fields, annotations, and list index assignment.

**Tech Stack:** Rust, Zun frontend/runtime, Cargo unit/integration/spec tests, Markdown docs

---

## Research Summary

- [`crates/zun-frontend/src/ast/ast.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/ast/ast.rs) still stores `$Type` field types as `type_name: String` plus `optional: bool`, and variable annotations as `Option<String>`.
- [`crates/zun-frontend/src/parser/stmt/declarations.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-frontend/src/parser/stmt/declarations.rs) only accepts a single identifier after `:` for both `$Type` fields and variable / constant declarations.
- [`crates/zun-runtime/src/interpreter/env.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/env.rs) stores `TypeEnv = HashMap<String, ValueType>`, which cannot express `User` or `List<Post>`.
- [`crates/zun-runtime/src/interpreter/type_validation.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/type_validation.rs) currently validates only string type names and `List<T>` string parsing; it has no concept of `Optional(T)` or structured type expressions.
- [`crates/zun-runtime/src/interpreter/exec/variables.rs`](/Users/liangzhanbo/CodeStudio/Zun/crates/zun-runtime/src/interpreter/exec/variables.rs) already owns variable declaration and index assignment behavior, so it is the right place to enforce annotation-backed `List<T>` writes.

## File Map

- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/declarations.rs`
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Modify: `crates/zun-runtime/src/interpreter/env.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/type_validation.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/constructors.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`
- Modify: `tests/spec/valid/data/type_construction.zun`
- Modify: `tests/spec/valid/data/type_construction.zun.stdout`
- Create: `tests/spec/valid/data/type_optional_null_and_list_construction.zun`
- Create: `tests/spec/valid/data/type_optional_null_and_list_construction.zun.stdout`
- Create: `tests/spec/invalid/syntax/type_expr_rejects_optional_list_item.zun`
- Create: `tests/spec/invalid/syntax/type_expr_rejects_optional_list_item.zun.error`
- Create: `tests/spec/invalid/data/type_list_field_mismatch.zun`
- Create: `tests/spec/invalid/data/type_list_field_mismatch.zun.error`
- Create: `tests/spec/invalid/data/variable_typed_list_mismatch.zun`
- Create: `tests/spec/invalid/data/variable_typed_list_mismatch.zun.error`
- Modify: `tests/integration_suite.rs`
- Modify: `docs/guide/09-gradual-typing.md`
- Modify: `docs/reference/types.md`

### Task 1: Lock Phase 2 Behavior With Red Fixtures

**Files:**
- Modify: `tests/spec/valid/data/type_construction.zun`
- Modify: `tests/spec/valid/data/type_construction.zun.stdout`
- Create: `tests/spec/valid/data/type_optional_null_and_list_construction.zun`
- Create: `tests/spec/valid/data/type_optional_null_and_list_construction.zun.stdout`
- Create: `tests/spec/invalid/syntax/type_expr_rejects_optional_list_item.zun`
- Create: `tests/spec/invalid/syntax/type_expr_rejects_optional_list_item.zun.error`
- Create: `tests/spec/invalid/data/type_list_field_mismatch.zun`
- Create: `tests/spec/invalid/data/type_list_field_mismatch.zun.error`
- Create: `tests/spec/invalid/data/variable_typed_list_mismatch.zun`
- Create: `tests/spec/invalid/data/variable_typed_list_mismatch.zun.error`
- Test: `tests/spec_suite.rs`

- [ ] **Step 1: Expand the existing valid construction fixture to exercise `List<T>` field construction**

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

$Type Feed {
    items: List<Post>
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

$ feed = Feed {
    items: [post],
};

$>> feed.items[0].title;
```

```text
Hello
```

- [ ] **Step 2: Add one valid fixture that proves optional fields can be omitted or set to `null`, and that `List<T>?` accepts both `null` and a typed list**

```zun
$Type Post {
    title: String
}

$Type Profile {
    nickname: String?
    posts: List<Post>?
}

$ empty_profile = Profile {};
$ full_profile = Profile {
    nickname: null,
    posts: [
        Post {
            title: "First",
        }
    ],
};

$>> empty_profile.nickname.type();
$>> full_profile.nickname.type();
$>> full_profile.posts[0].title;
```

```text
Null
Null
First
```

- [ ] **Step 3: Add the syntax fixture that must reject optional list elements**

```zun
$Type Feed {
    items: List<User?>
}
```

```text
ZUN-P001
```

- [ ] **Step 4: Add the invalid list-field mismatch fixture**

```zun
$Type User {
    id: Int
}

$Type Post {
    title: String
}

$Type Feed {
    items: List<Post>
}

$ feed = Feed {
    items: [
        User {
            id: 1,
        }
    ],
};
```

```text
ZUN-R011
```

- [ ] **Step 5: Add the invalid typed variable list mismatch fixture**

```zun
$Type User {
    id: Int
}

$Type Post {
    title: String
}

$ posts: List<Post> = [
    User {
        id: 1,
    }
];
```

```text
ZUN-R006
```

- [ ] **Step 6: Run the new fixtures before implementation to confirm they fail for the right reason**

Run: `cargo test --test spec_suite invalid_spec_samples_fail_with_expected_error -- --nocapture`
Expected: FAIL because the new syntax and runtime fixtures are not enforced yet

- [ ] **Step 7: Commit the phase-2 fixture lock**

```bash
git add tests/spec/valid/data/type_construction.zun tests/spec/valid/data/type_construction.zun.stdout tests/spec/valid/data/type_optional_null_and_list_construction.zun tests/spec/valid/data/type_optional_null_and_list_construction.zun.stdout tests/spec/invalid/syntax/type_expr_rejects_optional_list_item.zun tests/spec/invalid/syntax/type_expr_rejects_optional_list_item.zun.error tests/spec/invalid/data/type_list_field_mismatch.zun tests/spec/invalid/data/type_list_field_mismatch.zun.error tests/spec/invalid/data/variable_typed_list_mismatch.zun tests/spec/invalid/data/variable_typed_list_mismatch.zun.error
git commit -m "test: lock $Type phase-2 fixtures"
```

### Task 2: Introduce `TypeExpr` In The Frontend AST And Parser

**Files:**
- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/declarations.rs`

- [ ] **Step 1: Add the shared `TypeExpr` AST node and migrate `$Type` fields plus variable annotations to use it**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Named(String),
    List(Box<TypeExpr>),
    Optional(Box<TypeExpr>),
}

#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub hidden: bool,
}

pub struct VarDeclStmt {
    pub span: Span,
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Box<Expr>,
}
```

- [ ] **Step 2: Add one parser helper for `Ident`, `Ident?`, `List<T>`, and `List<T>?`**

```rust
fn parse_type_expr(&mut self) -> Result<TypeExpr, Error> {
    let mut base = if self.peek().typ == Type::Ident && self.peek().literal == "List" {
        self.advance();
        self.expect(Type::Less)?;
        let inner = self.parse_type_expr()?;
        self.expect(Type::Greater)?;
        if matches!(inner, TypeExpr::Optional(_)) {
            return Err(Error::Parse(crate::error::ParseError::new(
                "optional list item types are not supported; use List<T> or List<T>?"
            )));
        }
        TypeExpr::List(Box::new(inner))
    } else {
        TypeExpr::Named(self.expect_ident()?.literal.clone())
    };

    if !self.at_end() && self.peek().typ == Type::Question {
        self.advance();
        base = TypeExpr::Optional(Box::new(base));
    }

    Ok(base)
}
```

- [ ] **Step 3: Wire `parse_var_decl`, `parse_const_decl`, and `parse_type_decl` to call the helper**

```rust
let type_annotation = if self.peek().typ == Type::Colon {
    self.advance();
    Some(self.parse_type_expr()?)
} else {
    None
};
```

- [ ] **Step 4: Add parser-focused unit tests for `List<User>?` and rejection of `List<User?>`**

```rust
#[test]
fn parse_type_decl_accepts_optional_list_field_type() {
    let input = "$Type Feed { items: List<User>? }";
    let statements = crate::parser::parse(input).expect("parse should succeed");
    // assert TypeExpr::Optional(TypeExpr::List(TypeExpr::Named("User")))
}
```

- [ ] **Step 5: Run the frontend parser coverage**

Run: `cargo test -p zun-frontend parse_type_decl_accepts_optional_list_field_type -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit the frontend type-expression parser work**

```bash
git add crates/zun-frontend/src/ast/ast.rs crates/zun-frontend/src/parser/stmt/declarations.rs
git commit -m "feat: parse structured type expressions"
```

### Task 3: Upgrade Runtime Shapes, TypeEnv, And Validator To `TypeExpr`

**Files:**
- Modify: `crates/zun-runtime/src/runtime/context.rs`
- Modify: `crates/zun-runtime/src/interpreter/env.rs`
- Modify: `crates/zun-runtime/src/interpreter/mod.rs`
- Modify: `crates/zun-runtime/src/interpreter/type_validation.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/functions.rs`

- [ ] **Step 1: Replace string-backed field and annotation storage with `TypeExpr`**

```rust
pub struct TypeField {
    pub name: String,
    pub type_expr: crate::ast::TypeExpr,
    pub hidden: bool,
}

pub type TypeEnv = HashMap<String, crate::ast::TypeExpr>;
```

- [ ] **Step 2: Add runtime validation on structured expressions, including optional-null behavior**

```rust
pub fn validate_value_against_type_expr(
    expected: &TypeExpr,
    value: &ZunValue,
    context: &RuntimeContext,
) -> Result<(), TypeValidationError> {
    match expected {
        TypeExpr::Named(name) => validate_named_type(name, value, context),
        TypeExpr::List(inner) => validate_list(inner, value, context),
        TypeExpr::Optional(inner) => {
            if matches!(value, ZunValue::Null) {
                Ok(())
            } else {
                validate_value_against_type_expr(inner, value, context)
            }
        }
    }
}
```

- [ ] **Step 3: Add a bridge that converts old return-type strings into `TypeExpr` at runtime**

```rust
pub fn parse_runtime_type_expr(type_name: &str) -> Result<TypeExpr, TypeValidationError> {
    // Parse Ident / List<T> / T? / List<T>?.
    // Reject bare List and List<User?>.
}
```

- [ ] **Step 4: Keep return-type validation calling the bridge so function and HTTP signatures do not have to migrate in this phase**

```rust
let expected = parse_runtime_type_expr(expected_type)?;
validate_value_against_type_expr(&expected, value, context)
```

- [ ] **Step 5: Add unit tests for `Optional(Named(String))`, `Optional(List(User))`, and `List(User)` mismatch paths**

```rust
#[test]
fn validate_optional_string_accepts_null() {
    let context = test_context();
    let ty = TypeExpr::Optional(Box::new(TypeExpr::Named("String".into())));
    assert!(validate_value_against_type_expr(&ty, &ZunValue::Null, &context).is_ok());
}
```

- [ ] **Step 6: Run the runtime validator coverage**

Run: `cargo test -p zun-runtime type_validation -- --nocapture`
Expected: PASS

- [ ] **Step 7: Commit the runtime `TypeExpr` validator migration**

```bash
git add crates/zun-runtime/src/runtime/context.rs crates/zun-runtime/src/interpreter/env.rs crates/zun-runtime/src/interpreter/mod.rs crates/zun-runtime/src/interpreter/type_validation.rs crates/zun-runtime/src/interpreter/exec/functions.rs
git commit -m "refactor: validate runtime values with type expressions"
```

### Task 4: Enforce Phase-2 Construction, Annotation, And Index Assignment Rules

**Files:**
- Modify: `crates/zun-runtime/src/interpreter/eval/constructors.rs`
- Modify: `crates/zun-runtime/src/interpreter/exec/variables.rs`
- Modify: `tests/integration_suite.rs`

- [ ] **Step 1: Make typed construction validate `List<T>` fields and optional-null fields via `TypeExpr`**

```rust
validate_typed_instance_fields(&ctor.type_name, &fields, context)
    .map_err(|error| type_validation_error(error, ctor.span))?;
```

```rust
let feed = Feed {
    items: [post],
};
```

- [ ] **Step 2: Upgrade variable and constant declarations so `: User` and `: List<Post>` store `TypeExpr` and validate the initial value**

```rust
if let Some(ref type_expr) = stmt.type_annotation {
    validate_value_against_type_expr(type_expr, &val, context)
        .map_err(as_type_annotation_error)?;
    state.type_env.insert(stmt.name.clone(), type_expr.clone());
}
```

- [ ] **Step 3: Enforce typed list index assignment when the receiver variable is declared as `List<T>`**

```rust
if let Some(type_expr) = state.type_env.get(&var_name) {
    if let TypeExpr::List(item_type) = type_expr {
        validate_value_against_type_expr(item_type, &val, context)
            .map_err(as_index_assignment_error)?;
    }
}
```

- [ ] **Step 4: Add integration tests for typed list variables and typed list index assignment**

```rust
#[test]
fn typed_list_variable_rejects_wrong_initial_item() {
    let outcome = run_fixture(
        "spec/invalid/data/variable_typed_list_mismatch.zun",
        RuntimeMode::Test,
    );
    let error = outcome.error.expect("typed list should fail");
    assert!(error.contains("variable 'posts' expects type 'List<Post>'"));
}
```

```rust
#[test]
fn typed_list_index_assignment_rejects_wrong_item_type() {
    let project_dir = write_temp_project(
        "zun-typed-list-index",
        "name = \"typed-list-index\"\nversion = \"0.1.0\"\nentry = \"main.zun\"\n",
        &[(
            "main.zun",
            r#"$Type User { id: Int }
$Type Post { title: String }

$ posts: List<Post> = [
    Post { title: "ok" }
];

posts[0] = User { id: 1 };
"#,
        )],
    );
    let outcome = run_program_at_path(&project_dir.join("main.zun"), RuntimeMode::Test);
    let error = outcome.error.expect("wrong list item should fail");
    assert!(error.contains("list element for 'posts' expects type 'Post'"));
}
```

- [ ] **Step 5: Run focused integration coverage**

Run: `cargo test --test integration_suite typed_list_ -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit the phase-2 runtime enforcement**

```bash
git add crates/zun-runtime/src/interpreter/eval/constructors.rs crates/zun-runtime/src/interpreter/exec/variables.rs tests/integration_suite.rs
git commit -m "feat: enforce type expressions in runtime state"
```

### Task 5: Update Docs To Explain `List<T>` Fields And Optional-Null Semantics

**Files:**
- Modify: `docs/guide/09-gradual-typing.md`
- Modify: `docs/reference/types.md`

- [ ] **Step 1: Add one guide example showing a `$Type` field with `List<Post>` and an optional-null field**

```md
$Type Feed {
    items: List<Post>
    next_cursor: String?
}

`next_cursor` 可以缺失，也可以显式为 `null`。
`items` 必须是 `List<Post>`，不能混入 `Map` 或其他 typed instance。
```

- [ ] **Step 2: Add one reference note that `List<User?>` is not part of the current mainline**

```md
当前主线支持：

- `User`
- `User?`
- `List<User>`
- `List<User>?`

当前主线不支持：

- `List<User?>`
```

- [ ] **Step 3: Run the nearest targeted regression after docs land**

Run: `cargo test --test integration_suite strict_type_construction_fixture_passes typed_list_ -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit the docs**

```bash
git add docs/guide/09-gradual-typing.md docs/reference/types.md
git commit -m "docs: explain phase-2 type expressions"
```

## Self-Review

- Spec coverage: `TypeExpr`, `$Type` list fields, optional-null semantics, variable / constant annotations, and typed list index assignment all have dedicated tasks. List methods and parameter annotations remain explicitly out of scope.
- Placeholder scan: every task contains exact file paths, concrete snippets, verification commands, and expected outcomes.
- Type consistency: the plan consistently uses `TypeExpr`, `validate_value_against_type_expr`, `parse_runtime_type_expr`, and `TypeEnv = HashMap<String, TypeExpr>` across frontend and runtime tasks.
