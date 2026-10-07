# Typed List Mutators Design

## Goal

Close the remaining `List<T>` write-path gap by enforcing typed element validation for list mutator methods that add new elements: `push` and `insert`.

## Scope

This spec only covers mutable list methods that introduce new elements into an existing list.

Included:
- `list.push(value)`
- `list.insert(index, value)`

Explicitly excluded:
- `pop`
- `remove_at`
- `clear`
- `reverse`
- `sort`
- `sort_desc`
- list methods that do not currently exist, such as `append` or `extend`

## Runtime Semantics

Typed list mutator validation is driven by variable declarations, not by implicit tags on list values.

- If a variable is declared as `List<T>`, then `push` and `insert` must validate the incoming element against `T`.
- If a variable has no declared type, mutator behavior stays dynamic.
- If a mutable method receiver is not a direct variable reference, current behavior remains unchanged.

Examples:

```zun
$Type Post {
    title: String
}

$Type User {
    name: String
}

$ posts: List<Post> = []
posts.push(Post { title: "Hello" })     // ok
posts.insert(0, Post { title: "World" }) // ok
posts.push(User { name: "Alice" })      // error
```

## Integration Point

Validation happens before dispatching to the underlying mutable list builtin.

The existing mutable method execution path already requires direct variable receivers, so the runtime can:

1. Resolve the receiver variable name.
2. Look up its declared type in `type_env`.
3. If the declared type is `List<T>`, validate the incoming element against `T`.
4. Only call the list builtin when validation succeeds.

This keeps type enforcement centralized in interpreter execution instead of scattering ad hoc checks inside list builtins.

## Error Model

Validation failures use a dedicated list mutator message shape:

- `list method 'push' for 'posts' expects item type 'Post', got 'User'`
- `list method 'insert' for 'posts' expects item type 'Post', got 'Map'`

Parsing and nested `$Type` validation continue to reuse the existing `TypeExpr` validator. This spec only adds method-specific wrapping around those failures.

## Testing

Required coverage:

- `List<T>` variable accepts `push(T)`
- `List<T>` variable rejects `push(U)`
- `List<T>` variable accepts `insert(index, T)`
- `List<T>` variable rejects `insert(index, U)`
- Dynamic list without annotation still accepts heterogeneous `push` and `insert`

## Non-Goals

- Tracking element types on anonymous temporary lists
- Adding implicit type tags to all list values
- Expanding support to list methods not currently implemented
- Changing return type parsing or parameter type annotations
