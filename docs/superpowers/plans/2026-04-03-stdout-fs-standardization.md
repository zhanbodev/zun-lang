# Stdout And Fs Standardization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep `$>>` / `$>>ERR` as output syntax sugar, migrate file capabilities to `std.fs`, and validate the migration across Rust tests, `zun run`, and `zun serve`.

**Architecture:** Output stays as syntax-level primitives in the parser and runtime. File reads/writes become standard-library-first through `std.fs`, with a compatibility window for `$>>FILE/$<<FILE` while docs and tests migrate. Validation is split across runtime tests, script fixtures, and live HTTP integration tests so run/serve behavior stays aligned.

**Tech Stack:** Rust, Zun frontend parser/AST, Zun runtime/stdlib, CLI integration tests, `.zun` fixtures

---

## File Map

### Parser / AST / Syntax Removal Boundary

- Modify: `crates/zun-frontend/src/lexer/lexer.rs`
  - Current tokenization path for `$>>FILE` / `$<<FILE`
- Modify: `crates/zun-frontend/src/parser/stmt/io.rs`
  - Current statement parsing for `$>>FILE` / `$<<FILE`
- Modify: `crates/zun-frontend/src/parser/expr/primary.rs`
  - Current expression parsing for `FileWriteExpr` and `FileReadExpr`
- Modify: `crates/zun-frontend/src/ast/ast.rs`
  - Legacy `FileWrite*` / `FileRead*` AST nodes and comments
- Modify: `docs/spec/lexical.md`
- Modify: `docs/spec/syntax.md`

### Runtime / Stdlib

- Modify: `crates/zun-runtime/src/stdlib_native/fs.rs`
  - Add missing `write`, `append`, `delete` coverage if not already complete
- Modify: `crates/zun-runtime/src/stdlib_native/mod.rs`
  - Ensure `std.fs` exports the target API set
- Modify: `crates/zun-runtime/src/runtime/intrinsics.rs`
  - Reuse existing filesystem intrinsics or add missing ones for write/append/delete
- Modify: `crates/zun-runtime/src/interpreter/exec/io.rs`
  - Keep `$>>` / `$>>ERR` semantics stable
- Modify: `crates/zun-runtime/src/interpreter/eval/io.rs`
  - Remove legacy file-expression coupling when deprecation/removal phase arrives

### Compatibility / Diagnostics

- Modify: `crates/zun-frontend/src/diagnostics/codes.rs`
  - Add deprecation/removal diagnostic codes if needed
- Modify: `crates/zun-frontend/src/error/error.rs`
  - Support stable legacy-removal messages if parser/runtime emits them

### Tests

- Modify: `tests/spec/valid/stdlib/`
  - Add `std.fs` write/append/delete/read_text/read_lines fixtures
- Modify: `tests/spec/invalid/stdlib/`
  - Add removal/deprecation fixtures for legacy file syntax when phase reaches that point
- Modify: `tests/baseline_phase0.rs`
  - Add direct runtime smoke coverage for `std.fs`
- Modify: `tests/integration_suite.rs`
  - Add live `serve` handlers that call `std.fs`
- Modify: `tests/support/mod.rs`
  - Reuse temp project helpers if needed for run/serve fs tests

### Docs / Examples

- Modify: `docs/reference/functions.md`
- Modify: `docs/reference/stdlib-api.md`
- Modify: `docs/reference/syntax.md`
- Modify: `docs/guide/14-io-env-config.md`
- Modify: `docs/guide/examples.md`
- Modify: `docs/guide/appendix/syntax-cheatsheet.md`
- Modify: `docs/spec/security-model.md`
- Modify: `docs/CHANGELOG.md`

---

### Task 1: Lock The `std.fs` Surface With Failing Runtime Tests

**Files:**
- Modify: `tests/baseline_phase0.rs`
- Test: `tests/baseline_phase0.rs`

- [ ] **Step 1: Write failing tests for the target `std.fs` API surface**

Add focused tests near the existing runtime helper tests in `tests/baseline_phase0.rs`:

```rust
#[test]
fn std_fs_write_append_delete_round_trip() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("zun-stdout-fs-{unique}.txt"));
    let source = format!(
        "$mod std.fs;\n\
         fs.write(\"{}\", \"hello\");\n\
         fs.append(\"{}\", \"\\nworld\");\n\
         $ text = fs.read_text(\"{}\");\n\
         $ exists_before = fs.exists(\"{}\");\n\
         $ size_before = fs.size(\"{}\");\n\
         fs.delete(\"{}\");\n\
         $ exists_after = fs.exists(\"{}\");\n",
        path.display(),
        path.display(),
        path.display(),
        path.display(),
        path.display(),
        path.display(),
        path.display()
    );

    let (state, _context, _output) = run_program(&source);
    assert_eq!(
        state.env.get("text"),
        Some(&ZunValue::Str("hello\nworld".to_string()))
    );
    assert_eq!(state.env.get("exists_before"), Some(&ZunValue::Bool(true)));
    assert_eq!(state.env.get("exists_after"), Some(&ZunValue::Bool(false)));
    assert_eq!(state.env.get("size_before"), Some(&ZunValue::Int(11)));
}

#[test]
fn std_fs_read_lines_and_is_dir_work() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("zun-stdout-fs-dir-{unique}"));
    fs::create_dir_all(&dir).expect("dir should exist");
    let file = dir.join("lines.txt");
    fs::write(&file, "a\nb\n").expect("fixture file");

    let source = format!(
        "$mod std.fs;\n\
         $ lines = fs.read_lines(\"{}\");\n\
         $ dir_flag = fs.is_dir(\"{}\");\n",
        file.display(),
        dir.display()
    );

    let (state, _context, _output) = run_program(&source);
    assert_eq!(
        state.env.get("lines"),
        Some(&ZunValue::List(vec![
            ZunValue::Str("a".to_string()),
            ZunValue::Str("b".to_string())
        ]))
    );
    assert_eq!(state.env.get("dir_flag"), Some(&ZunValue::Bool(true)));
    fs::remove_dir_all(dir).expect("cleanup");
}
```

- [ ] **Step 2: Run the new runtime tests and confirm the current failure shape**

Run: `cargo test --test baseline_phase0 std_fs_ -- --nocapture`

Expected: at least one FAIL if `std.fs.write/append/delete` is not yet fully wired; if they already pass, keep the tests and proceed with the migration tasks as regression coverage.

- [ ] **Step 3: Implement the minimal missing `std.fs` functions**

If any tests fail, patch the `std.fs` export layer in these files:

```rust
// crates/zun-runtime/src/stdlib_native/mod.rs
pub fn register_stdlib_native_modules(context: &mut RuntimeContext) {
    fs::register(context);
}
```

```rust
// crates/zun-runtime/src/stdlib_native/fs.rs
// Ensure exported names include:
// write(path, content)
// append(path, content)
// delete(path)
// read_text(path)
// read_lines(path)
// exists(path)
// size(path)
// is_dir(path)
```

```rust
// crates/zun-runtime/src/runtime/intrinsics.rs
// Reuse or add host calls for:
// FS_WRITE_TEXT
// FS_APPEND_TEXT
// FS_DELETE
// FS_READ_TEXT
// FS_EXISTS
// FS_SIZE
// FS_IS_DIR
```

Keep behavior minimal:

- `write` overwrites
- `append` appends
- `delete` removes path
- errors bubble through existing runtime error formatting

- [ ] **Step 4: Re-run the runtime tests**

Run: `cargo test --test baseline_phase0 std_fs_ -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/baseline_phase0.rs crates/zun-runtime/src/stdlib_native/fs.rs crates/zun-runtime/src/stdlib_native/mod.rs crates/zun-runtime/src/runtime/intrinsics.rs
git commit -m "feat: complete std fs core runtime api"
```

---

### Task 2: Add `zun run` Fixture Coverage For The New Main Path

**Files:**
- Create: `tests/spec/valid/stdlib/fs_write_append_delete.zun`
- Create: `tests/spec/valid/stdlib/fs_write_append_delete.zun.stdout`
- Create: `tests/spec/valid/stdlib/fs_read_text_and_lines.zun`
- Create: `tests/spec/valid/stdlib/fs_read_text_and_lines.zun.stdout`
- Test: `tests/spec/valid/stdlib/`

- [ ] **Step 1: Write the failing `zun run` fixtures**

Create `tests/spec/valid/stdlib/fs_write_append_delete.zun`:

```zun
$mod std.fs;

$ path = "tests/fixtures/stdlib/fs_write_append_delete.txt";

fs.write(path, "hello");
fs.append(path, "\nworld");

$>> fs.read_text(path);
$>> fs.exists(path);
$>> fs.size(path);

fs.delete(path);
$>> fs.exists(path);
```

Create `tests/spec/valid/stdlib/fs_write_append_delete.zun.stdout`:

```text
hello
world
true
11
false
```

Create `tests/spec/valid/stdlib/fs_read_text_and_lines.zun`:

```zun
$mod std.fs;

$>> fs.read_text("tests/fixtures/stdlib/lines.txt");
$>> fs.read_lines("tests/fixtures/stdlib/lines.txt");
$>> fs.is_dir("tests/fixtures/stdlib");
```

Create `tests/spec/valid/stdlib/fs_read_text_and_lines.zun.stdout`:

```text
line1
line2

[line1, line2]
true
```

- [ ] **Step 2: Run the fixture tests and confirm failures if any**

Run: `cargo test -p zun-runtime fs_ -- --nocapture`

Expected: FAIL if stdout formatting or fs API wiring is incomplete; otherwise PASS and serve as regression coverage.

- [ ] **Step 3: Fix any stdout or value-format mismatch**

If fixture output mismatches, adjust only the minimal formatting/runtime behavior needed:

```rust
// crates/zun-runtime/src/interpreter/exec/io.rs
// keep `$>>` semantics unchanged

// crates/zun-runtime/src/interpreter/value.rs
// only if List/String formatting is inconsistent with existing fixture conventions
```

Do not redesign print formatting beyond what the fixture needs.

- [ ] **Step 4: Re-run the fixture tests**

Run: `cargo test -p zun-runtime fs_ -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/spec/valid/stdlib/fs_write_append_delete.zun tests/spec/valid/stdlib/fs_write_append_delete.zun.stdout tests/spec/valid/stdlib/fs_read_text_and_lines.zun tests/spec/valid/stdlib/fs_read_text_and_lines.zun.stdout
git commit -m "test: cover std fs in run mode fixtures"
```

---

### Task 3: Add Live `zun serve` Coverage For `std.fs`

**Files:**
- Modify: `tests/integration_suite.rs`
- Modify: `tests/support/mod.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write failing live serve integration tests**

Add tests to `tests/integration_suite.rs` using the existing live HTTP harness pattern:

```rust
#[test]
fn live_http_std_fs_write_read_delete_flow_works() {
    let project = r#"
[project]
name = "fs-serve"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8094
"#;

    let main = r#"
$mod std.fs;

$POST("/write") write_file() -> JSON {
    fs.write("tmp_serve_fs.txt", "hello");
    $# {"ok": true};
}

$POST("/append") append_file() -> JSON {
    fs.append("tmp_serve_fs.txt", "\nworld");
    $# {"ok": true};
}

$GET("/read") read_file() -> String {
    $# fs.read_text("tmp_serve_fs.txt");
}

$DELETE("/file") delete_file() -> JSON {
    fs.delete("tmp_serve_fs.txt");
    $# {"exists": fs.exists("tmp_serve_fs.txt")};
}
"#;

    // create temp project, boot serve, issue HTTP requests, assert responses
}
```

Also add metadata coverage:

```rust
#[test]
fn live_http_std_fs_meta_works() {
    // expose fs.exists/fs.size/fs.is_dir via GET /meta and assert JSON response
}
```

- [ ] **Step 2: Run the new live tests and verify the current failure**

Run: `cargo test --test integration_suite live_http_std_fs_ -- --nocapture`

Expected: FAIL until the serve-mode path handling and/or harness setup is correct.

- [ ] **Step 3: Implement the minimal serve-mode fixes**

If the tests fail, fix only the real issue:

```rust
// tests/support/mod.rs
// add temp-project helpers if existing helpers do not support this fixture style

// crates/zun-runtime/src/runtime/context.rs
// only if project-root/current-file path resolution differs between run and serve
```

Keep path semantics aligned with current project-root behavior.

- [ ] **Step 4: Re-run the live tests**

Run: `cargo test --test integration_suite live_http_std_fs_ -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add tests/integration_suite.rs tests/support/mod.rs crates/zun-runtime/src/runtime/context.rs
git commit -m "test: cover std fs in serve mode"
```

---

### Task 4: Migrate Documentation And Examples To `std.fs`

**Files:**
- Modify: `docs/reference/functions.md`
- Modify: `docs/reference/stdlib-api.md`
- Modify: `docs/reference/syntax.md`
- Modify: `docs/guide/14-io-env-config.md`
- Modify: `docs/guide/examples.md`
- Modify: `docs/guide/appendix/syntax-cheatsheet.md`
- Modify: `docs/spec/security-model.md`
- Modify: `docs/CHANGELOG.md`

- [ ] **Step 1: Replace primary file examples with `std.fs`**

Update docs so the primary examples use this shape:

```zun
$mod std.fs;

fs.write("output.txt", "Hello World");
fs.append("output.txt", "\nSecond line");

$ text = fs.read_text("output.txt");
$ lines = fs.read_lines("output.txt");

$>> text;
$>> lines;
```

Use a legacy note for old syntax:

```md
`$>>FILE(...)` / `$<<FILE(...)` are legacy syntax and will be removed after the `std.fs` migration window. Prefer `std.fs`.
```

- [ ] **Step 2: Update the stdlib API tables**

In `docs/reference/stdlib-api.md`, make sure the `fs` section includes:

```md
| `fs.write(path, content)` | `Null` | 覆盖写入文本 | Stable |
| `fs.append(path, content)` | `Null` | 追加写入文本 | Stable |
| `fs.delete(path)` | `Null` | 删除文件 | Stable |
| `fs.read_text(path)` | `String` | 读取全文 | Stable |
| `fs.read_lines(path)` | `List` | 按行读取 | Stable |
| `fs.exists(path)` | `Bool` | 文件/目录是否存在 | Stable |
| `fs.size(path)` | `Int` | 文件大小 | Stable |
| `fs.is_dir(path)` | `Bool` | 是否为目录 | Stable |
```

- [ ] **Step 3: Update security-model docs**

In `docs/spec/security-model.md`, move file capability emphasis from syntax to stdlib:

```md
- `std.fs.write(...)`
- `std.fs.append(...)`
- `std.fs.delete(...)`
- `std.fs.read_text(...)`
- `std.fs.read_lines(...)`
```

Keep legacy syntax listed only while compatibility remains.

- [ ] **Step 4: Review docs for consistency**

Run: `rg -n '\$>>FILE|\$<<FILE' docs`

Expected: only legacy/deprecation references remain, not primary recommendation sections.

- [ ] **Step 5: Commit**

```bash
git add docs/reference/functions.md docs/reference/stdlib-api.md docs/reference/syntax.md docs/guide/14-io-env-config.md docs/guide/examples.md docs/guide/appendix/syntax-cheatsheet.md docs/spec/security-model.md docs/CHANGELOG.md
git commit -m "docs: migrate file guidance to std fs"
```

---

### Task 5: Add Compatibility Messaging For Legacy File Syntax

**Files:**
- Modify: `crates/zun-frontend/src/diagnostics/codes.rs`
- Modify: `crates/zun-frontend/src/parser/stmt/io.rs`
- Modify: `crates/zun-frontend/src/parser/expr/primary.rs`
- Test: `tests/spec/invalid/stdlib/`

- [ ] **Step 1: Write the failing invalid/deprecation fixtures**

Create a removal-phase fixture such as `tests/spec/invalid/stdlib/legacy_file_syntax_removed.zun`:

```zun
$>>FILE("output.txt", "hello");
```

Expected diagnostic file `tests/spec/invalid/stdlib/legacy_file_syntax_removed.zun.error`:

```text
error[<CODE>]: legacy file syntax has been removed
note: replace `$>>FILE(path, content)` with `std.fs.write(path, content)`
```

If the product decision is deprecation before removal, use a non-fatal warning plan instead and keep this task for the actual removal phase.

- [ ] **Step 2: Run the invalid fixture and confirm failure**

Run: `cargo test -p zun-runtime legacy_file_syntax_removed -- --nocapture`

Expected: FAIL until diagnostics are implemented.

- [ ] **Step 3: Implement stable migration messaging**

Add a dedicated diagnostic code and parser/runtime message:

```rust
// crates/zun-frontend/src/diagnostics/codes.rs
pub const LEGACY_FILE_SYNTAX_REMOVED: &str = "ZUN-C004";
```

```rust
// crates/zun-frontend/src/parser/stmt/io.rs
// crates/zun-frontend/src/parser/expr/primary.rs
// reject `$>>FILE` / `$<<FILE` with migration note
```

Keep the message specific:

- what syntax is removed
- which `std.fs` function replaces it

- [ ] **Step 4: Re-run the invalid fixture**

Run: `cargo test -p zun-runtime legacy_file_syntax_removed -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/zun-frontend/src/diagnostics/codes.rs crates/zun-frontend/src/parser/stmt/io.rs crates/zun-frontend/src/parser/expr/primary.rs tests/spec/invalid/stdlib/legacy_file_syntax_removed.zun tests/spec/invalid/stdlib/legacy_file_syntax_removed.zun.error
git commit -m "refactor: replace legacy file syntax with std fs diagnostics"
```

---

### Task 6: Remove Legacy Parser/AST Paths When The Migration Window Closes

**Files:**
- Modify: `crates/zun-frontend/src/ast/ast.rs`
- Modify: `crates/zun-frontend/src/lexer/lexer.rs`
- Modify: `crates/zun-runtime/src/interpreter/eval/io.rs`
- Modify: `docs/spec/lexical.md`
- Modify: `docs/spec/syntax.md`

- [ ] **Step 1: Confirm all compatibility prerequisites are complete**

Checklist before code removal:

- `std.fs` API complete
- run-mode fixtures passing
- serve-mode integration passing
- docs migrated
- compatibility messaging shipped for at least one cycle if required

Do not remove parser support before this checklist is true.

- [ ] **Step 2: Remove legacy AST/parser/runtime branches**

Delete or stop generating:

```rust
// ast.rs
FileWriteExpr
FileReadExpr
FileWriteStmt
FileReadStmt
```

Delete or stop recognizing:

```rust
// lexer.rs
$>>FILE
$<<FILE
```

Delete runtime-only legacy eval branches:

```rust
// interpreter/eval/io.rs
// remove obsolete file expr evaluation once syntax is gone
```

- [ ] **Step 3: Update syntax/lexical docs**

Remove primary mention of:

```md
- `$>>FILE`
- `$<<FILE`
```

Retain only migration note if needed.

- [ ] **Step 4: Run the full regression set**

Run:

```bash
cargo test --test baseline_phase0 -- --nocapture
cargo test --test integration_suite live_http_std_fs_ -- --nocapture
cargo test -p zun-cli -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/zun-frontend/src/ast/ast.rs crates/zun-frontend/src/lexer/lexer.rs crates/zun-runtime/src/interpreter/eval/io.rs docs/spec/lexical.md docs/spec/syntax.md
git commit -m "refactor: remove legacy file syntax"
```

---

## Self-Review

### Spec Coverage

- `$>>` / `$>>ERR` retained: covered by Tasks 1, 2, 4, 6
- `std.fs` becomes primary file API: covered by Tasks 1, 2, 3, 4
- migration path and compatibility window: covered by Tasks 4, 5, 6
- Rust / run / serve testing: covered by Tasks 1, 2, 3, 6

### Placeholder Scan

- No `TODO` / `TBD` markers remain
- Each task contains exact files and concrete commands
- Code blocks are included for tests and implementation touchpoints

### Type Consistency

- `fs.write`, `fs.append`, `fs.delete`, `fs.read_text`, `fs.read_lines`, `fs.exists`, `fs.size`, `fs.is_dir` are used consistently across all tasks
- Legacy syntax references consistently use `$>>FILE` / `$<<FILE`

