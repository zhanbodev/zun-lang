# Stdout And Fs Standardization Design

## Summary

为 Zun 当前混合在语法层里的输出与文件能力做一次职责收口：

- 保留 `$>> expr;` 作为 stdout 输出语法糖
- 保留 `$>>ERR("msg");` 作为 stderr 输出语法糖
- 停止把文件系统能力继续挂在 `$>>` 家族下面
- 将 `$>>FILE(...)`、`$<<FILE(...)` 以及相关文件能力迁移到正式标准库 `std.fs`
- 保证 `zun run` 与 `zun serve` 两种模式下文件能力行为一致

本次设计的核心不是“把 `$>>` 标准库化”，而是把 `$>>` 恢复为“输出”这一件事，同时让文件系统能力进入更清晰、更可扩展的标准库边界。

## Goals

- 保留 `$>>` / `$>>ERR` 的脚本友好性与现有输出语义
- 将文件读写、存在性、目录判断、大小等能力收口到 `std.fs`
- 为后续文件系统能力扩展提供稳定 API 边界
- 为旧语法迁移提供清晰、可验证的路径
- 让 Rust、`zun run`、`zun serve` 三条测试链路都能覆盖迁移

## Non-Goals

- 不移除 `$>>` 或 `$>>ERR`
- 不在这一轮设计里扩展新的终端颜色/logging DSL
- 不在这一轮设计里引入完整文件流/缓冲 IO 模型
- 不在这一轮设计里设计目录遍历、glob、watcher 等高级 fs API
- 不把 stderr 输出迁移到 `std.io`

## Current State

当前仓库里相关能力分散在两层：

### 1. 语法级输出能力

- `$>> expr;`
- `$>>ERR("msg");`

这两者由 lexer/parser 直接识别，并在 runtime 中执行打印。

### 2. 语法级文件能力

- `$>>FILE(path, content[, mode])`
- `$ f = $>>FILE(path[, mode]);`
- `$<<FILE(path[, mode])`

以及由 `File` 对象暴露出的能力：

- `content(...)`
- `read()`
- `read_lines()`
- `exists()`
- `size()`
- `is_dir()`

### 3. 已经存在的标准库文件能力

当前仓库中已经存在 `std.fs` 的一部分函数能力，并且文档里已经开始主推：

- `fs.read_text(path)`
- `fs.read_lines(path)`
- `fs.exists(path)`
- `fs.size(path)`
- `fs.is_dir(path)`

这意味着仓库实际上已经处于“双轨状态”：

- 一部分文件能力走语法糖
- 一部分文件能力走 `std.fs`

这正是需要收口的原因。

## Design Principles

### 1. `$>>` Means Output, Not Filesystem

`$>>` 的语义应该稳定地表示“把一个表达式结果输出到 stdout”。

它可以消费任何表达式值，包括：

```zun
$>> "hello";
$>> user.name;
$>> fs.read_text("notes.txt");
$>> fs.exists("data.txt");
```

但它不应再承担“创建文件对象”或“执行文件写入”这类宿主能力入口。

### 2. `stderr` Is Still A Syntax-Level Output Primitive

`$>>ERR("msg");` 与 `$>> expr;` 属于同一类能力，都是终端输出语法糖。

这类能力保留在语法层是合理的，不需要为了形式统一而强行迁移进标准库。

### 3. Filesystem Must Be Standard Library Capability

文件读写、追加、删除、存在性判断、目录判断、大小查询等能力，本质上属于 `fs`，不属于“输出语法扩展”。

这类能力进入 `std.fs` 后：

- 语义更清晰
- API 更容易继续扩展
- 文档更容易组织
- run/serve 的测试边界也更明确

### 4. Migration Must Be Incremental

不能一边删旧语法、一边还没有完整替代路径。必须先让 `std.fs` 覆盖到足够完整的能力面，再切换主推荐写法，最后才讨论彻底移除旧语法。

## Target Boundary

最终边界定义如下：

### 保留的语法糖

- `$>> expr;`
- `$>>ERR("msg");`

### 移除方向的语法糖

- `$>>FILE(...)`
- `$<<FILE(...)`

### 正式文件标准库

统一放在：

```zun
$mod std.fs;
```

主推荐 API 以函数式为主。

## Proposed `std.fs` API

第一阶段建议以“函数式 API 覆盖现有主路径”为主，不把复杂对象模型作为第一优先级。

### 写入类

- `fs.write(path, content)`
  - 覆盖写入
- `fs.append(path, content)`
  - 追加写入
- `fs.delete(path)`
  - 删除文件

### 读取类

- `fs.read_text(path)`
  - 读取全文，返回 `String`
- `fs.read_lines(path)`
  - 按行读取，返回 `List`

### 元信息类

- `fs.exists(path)`
  - 返回 `Bool`
- `fs.size(path)`
  - 返回 `Int`
- `fs.is_dir(path)`
  - 返回 `Bool`

### 可选的后续扩展

这轮不必首发，但 API 边界应允许后续添加：

- `fs.mkdir(path)`
- `fs.copy(from, to)`
- `fs.move(from, to)`
- `fs.open(path, mode)`
- `file.write(...)`
- `file.read(...)`

## Mapping From Existing Syntax

旧写法与新写法的推荐映射如下：

| 旧写法 | 新写法 |
| --- | --- |
| `$>>FILE(path, content)` | `fs.write(path, content)` |
| `$>>FILE(path, content, "W")` | `fs.write(path, content)` |
| `$>>FILE(path, content, "A")` | `fs.append(path, content)` |
| `$>>FILE(path, "", "DEL")` | `fs.delete(path)` |
| `$<<FILE(path)` | `fs.read_text(path)` 或 `fs.exists(path)` / `fs.size(path)` / `fs.is_dir(path)` |
| `$<<FILE(path, "LINES")` | `fs.read_lines(path)` |

注意：

- `$>> fs.read_text(path);` 是合法且推荐的组合方式
- `$>>` 只负责打印表达式结果，不承担 fs 行为本身

## File Object Strategy

当前 `File` 对象存在以下方法：

- `content(...)`
- `read()`
- `read_lines()`
- `exists()`
- `size()`
- `is_dir()`

本设计建议：

### Phase 1

不再继续推广 `File` 对象模型作为主入口，优先以 `std.fs` 的函数式 API 覆盖 80% 使用场景。

### Phase 2

评估是否还需要保留 `File` 对象。如果保留，应通过 `std.fs.open(...)` 进入，而不是 `$>>FILE/$<<FILE`。

### Naming Note

如果未来保留文件对象写入方法，`content(...)` 建议改名为 `write(...)`，因为它表达动作比表达“内容字段”更准确。

## Migration Plan

### Phase A: Fill `std.fs`

先确保 `std.fs` 覆盖当前 `$>>FILE/$<<FILE` 的主路径：

- `write`
- `append`
- `delete`
- `read_text`
- `read_lines`
- `exists`
- `size`
- `is_dir`

此阶段不删旧语法。

### Phase B: Switch Documentation

更新以下文档和示例，使 `std.fs` 成为主推荐路径：

- `docs/reference/functions.md`
- `docs/reference/stdlib-api.md`
- `docs/guide/14-io-env-config.md`
- `docs/guide/examples.md`
- `docs/guide/appendix/syntax-cheatsheet.md`

此阶段文档中对 `$>>FILE/$<<FILE` 标为 legacy 或 compatibility syntax。

### Phase C: Add Compatibility Notice

为旧语法增加迁移提示，方式二选一：

- parser/runtime 发出 deprecated 警告
- 或文档中明确标注“未来将移除”

如果当前诊断体系暂时不适合 non-fatal warning，先从文档声明开始。

### Phase D: Remove Legacy Syntax

当 `std.fs` 与文档、示例、测试都稳定后，再移除：

- `$>>FILE(...)`
- `$<<FILE(...)`

以及依赖这两条语法入口的 parser/runtime 路径。

## Acceptance Criteria

### Language/API Acceptance

- `$>> expr;` 行为完全不回归
- `$>>ERR("msg");` 行为完全不回归
- `std.fs` 足以覆盖当前主文件能力面
- `$>> fs.xxx(...)` 可以直接打印标准库返回值

### Migration Acceptance

- 文档主推荐路径全部切到 `std.fs`
- 新示例不再使用 `$>>FILE/$<<FILE`
- 旧语法在兼容期内仍然可运行，或能给出稳定迁移提示

### Runtime Acceptance

- `zun run` 下 `std.fs` 行为正确
- `zun serve` 下 handler 内使用 `std.fs` 行为正确
- 相对路径解析在项目模式下稳定
- run/serve 两种模式下对同一路径操作语义一致

## Test Strategy

测试必须覆盖 3 层：

### 1. Rust Tests

目的：验证 parser/runtime/stdlib 主行为。

覆盖点：

- `std.fs.write/append/delete`
- `std.fs.read_text/read_lines`
- `std.fs.exists/size/is_dir`
- 兼容期内 `$>>FILE/$<<FILE` 的旧行为
- 未来移除时的 parser/runtime 失败诊断

建议落点：

- `tests/baseline_phase0.rs`
- `tests/spec/valid/stdlib/*`
- `tests/spec/invalid/stdlib/*`

### 2. `zun run` Script Mode

目的：验证真实脚本运行语义，而不是只看 Rust helper。

最小覆盖矩阵：

- 写文件后读取全文
- 追加后读取全文
- 读取行列表
- 删除后 `exists=false`
- `size` 返回正确值
- `is_dir` 对目录路径返回 `true`
- `$>> fs.xxx(...)` 的终端输出符合预期

建议通过 `.zun` fixture 和 stdout 断言完成。

### 3. `zun serve` Service Mode

目的：验证文件能力在 HTTP handler 中同样可靠。

最小覆盖矩阵：

- `POST /write` -> `fs.write(...)`
- `POST /append` -> `fs.append(...)`
- `GET /read` -> `fs.read_text(...)`
- `GET /lines` -> `fs.read_lines(...)`
- `DELETE /file` -> `fs.delete(...)`
- `GET /meta` -> `fs.exists/size/is_dir`

验证方式：

- 真实启动 `serve`
- 通过 HTTP 请求触发 handler
- 校验响应体与磁盘副作用

建议落点：

- `tests/integration_suite.rs`

## Test Matrix

| Capability | Rust | `zun run` | `zun serve` |
| --- | --- | --- | --- |
| `fs.write` | Required | Required | Required |
| `fs.append` | Required | Required | Required |
| `fs.delete` | Required | Required | Required |
| `fs.read_text` | Required | Required | Required |
| `fs.read_lines` | Required | Required | Required |
| `fs.exists` | Required | Required | Required |
| `fs.size` | Required | Required | Required |
| `fs.is_dir` | Required | Required | Required |
| `$>> fs.xxx(...)` output | Optional | Required | Required |
| legacy `$>>FILE/$<<FILE` | Required during migration | Optional during migration | Optional during migration |

## Risks

### 1. Docs And Implementation Already Diverge

仓库当前已经部分主推 `std.fs`，但旧语法还广泛存在于文档与示例中。迁移时如果不系统更新，用户会继续混用两套风格。

### 2. Path Semantics May Drift Between Run And Serve

文件路径在项目根、当前文件、HTTP handler 执行上下文中的语义如果不统一，会导致 `run` 通过而 `serve` 失败。

### 3. Removing Syntax Too Early Will Break Existing Material

如果在 `std.fs`、文档、测试还没全部覆盖前就删掉 `$>>FILE/$<<FILE`，会造成示例、旧项目和教程断裂。

## Recommendation

采用以下执行顺序：

1. 先补齐并验证 `std.fs`
2. 再切文档和示例
3. 再决定旧语法是先 deprecated 还是直接删除

不要反过来做。

## Open Questions

- `File` 对象是否要保留为长期正式模型，还是统一收敛到函数式 `std.fs`
- 如果保留对象模型，是否引入 `fs.open(path, mode)` 作为唯一正式入口
- 旧语法的兼容窗口要跨几个版本
- deprecated 提示是 parser warning、runtime warning，还是先只做文档级声明
