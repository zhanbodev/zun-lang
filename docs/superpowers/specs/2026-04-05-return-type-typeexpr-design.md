# Return Type `TypeExpr` Migration Design

## Goal

Unify function and HTTP handler declared return types with the existing `TypeExpr` model so `$Type` fields, variable annotations, and return signatures all share the same structured type representation.

## Scope

Included:
- Ordinary function declared return types
- HTTP handler declared return types
- Parser and AST migration from `Option<String>` to `Option<TypeExpr>`
- Runtime return validation consuming `TypeExpr` directly on the main execution path

Explicitly excluded:
- New language-level return type capabilities
- Parameter type annotations
- Removal of every legacy string-based helper in this round

## Design

Return types move to the same structured model already used by `$Type` fields and variable annotations:

```rust
Option<TypeExpr>
```

This applies to:
- `$fn get_user() -> User`
- `$fn list_users() -> List<User>`
- `$GET("/users/:id") show_user() -> User`
- other HTTP superfunctions that declare return types

The parser must reuse the same type expression parser already used for `$Type` fields and variable annotations. Return signatures should no longer maintain a separate string-based parsing path.

## Runtime Behavior

Runtime return validation should consume the declared `TypeExpr` directly for:
- ordinary function returns
- HTTP handler returns

The existing runtime string bridge can remain temporarily for non-AST or compatibility paths, but function and HTTP execution should stop depending on string parsing once this migration lands.

This migration is representational, not behavioral:

- existing valid return signatures stay valid
- existing invalid return values stay invalid
- no automatic `Map -> $Type` coercion is added
- no new optional return type semantics are introduced by this spec

## Testing

Required coverage:

- existing return type validation tests compile and still pass with `TypeExpr`
- ordinary function `-> User` success/failure behavior remains unchanged
- ordinary function `-> List<User>` success/failure behavior remains unchanged
- HTTP handler return type validation remains unchanged on the current executable paths

## Non-Goals

- Adding `User?` / `List<User>?` as newly supported return syntax
- Expanding parameter annotations
- Deleting every string-based runtime type parsing helper in this same change
