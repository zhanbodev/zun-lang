# Parameter Type Annotations Design

## Goal

Extend the `$Type` / `TypeExpr` system to function parameters so ordinary functions, HTTP handlers, and anonymous functions can validate incoming arguments at bind time.

## Scope

Included:
- Ordinary function parameters
- HTTP handler parameters
- Anonymous function parameters
- Shared runtime parameter binding validation

Explicitly excluded:
- Variadic parameter type annotations
- Automatic `Map/JSON -> $Type` coercion
- New advanced type expression features beyond the existing `TypeExpr` model

## AST Model

Function-like declarations should stop storing parameters as `Vec<String>` and move to one shared parameter model:

```rust
pub struct FnParam {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
}
```

This applies to:
- `FnDeclStmt`
- `HttpFnStmt`
- `FnLiteral`

Variadic parameters remain name-only in this round:

```zun
$fn f(id: Int, ...rest) { ... }
```

Supported:
- `name`
- `name: Int`
- `name: User`
- `name: List<Post>`
- `name: String?`

Not supported in this round:
- `...rest: List<Int>`

## Runtime Semantics

Parameter validation happens when arguments are bound, before function body execution continues.

- If a parameter has no annotation, binding stays dynamic.
- If a parameter has a `TypeExpr`, the incoming argument must validate against it immediately.
- Validation uses the existing shared `validate_value_against_type_expr` helper.
- A parameter annotated as `User` only accepts a real typed instance of `User`, not a shape-compatible `Map`.

This behavior should be shared by:
- ordinary function calls
- module function calls
- anonymous function calls (through the same underlying call path)
- HTTP handler parameter binding

## Error Model

Validation failures use parameter-specific wording:

- `parameter 'id' expects type 'Int', got 'String'`
- `parameter 'user' expects type 'User', got 'Map'`
- `parameter 'items' expects type 'List<Post>', got 'List<User>'`

The message format should stay consistent across ordinary functions, module functions, anonymous functions, and HTTP handlers.

## Testing

Required coverage:

- Ordinary function parameter success / failure
- Module function parameter success / failure
- HTTP handler parameter success / failure
- Anonymous function parameter success / failure
- `List<T>` parameter success / failure
- `$Type` parameter rejects bare `Map`
- Variadic parameters continue to work without type annotations

## Non-Goals

- Typed variadic parameters
- Implicit value conversion into `$Type`
- New syntax beyond the current `TypeExpr` system
