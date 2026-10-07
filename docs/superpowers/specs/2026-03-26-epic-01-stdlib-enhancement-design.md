# EPIC-01 Stdlib Enhancement Design

## Goal

在 Zun 当前原生标准库体系上新增 `std.time`、`std.uuid`、`std.http` 三个模块，使脚本具备后端开发常见的时间处理、唯一 ID 生成、同步 HTTP 调用能力，同时保持现有 `$mod std.*;` 导入机制、`$try/$catch` 错误模型和 `tests/spec` 规范测试方式不变。

## Scope

本设计只覆盖 [ISSUES/EPIC-01-stdlib-enhancement.md](/Users/liangzhanbo/CodeStudio/zun/.worktrees/epic-01-stdlib-enhancement/ISSUES/EPIC-01-stdlib-enhancement.md) 及其三个子 issue：

- `STD-001` `std.time`
- `STD-002` `std.uuid`
- `STD-003` `std.http`

本次不实现：

- `std.crypto`
- `std.base64`
- `stdlib/http/` 纯 Zun 辅助层
- HTTP 客户端的 async 模型、流式响应、cookie/session 管理

## Current Context

当前仓库已经存在原生模块注册入口 [crates/zun-runtime/src/stdlib_native/mod.rs](/Users/liangzhanbo/CodeStudio/zun/.worktrees/epic-01-stdlib-enhancement/crates/zun-runtime/src/stdlib_native/mod.rs)，并通过 `RuntimeContext::register_native_module()` 把 Rust 闭包导出为 `$mod std.*;` 可见模块。现有模块 `std.fs`、`std.env`、`std.str`、`std.math`、`std.json` 都遵循同一模式：

- 每个模块一个 `*.rs`
- 运行时类型错误通过 `Error::Interpreter(...)` 抛出
- spec 测试通过 `tests/spec/valid|invalid/**/*.zun` 驱动

`std.http` 会新增网络副作用能力，因此除了代码和测试，还必须同步更新安全模型文档。

## Approach

采用“两阶段交付”策略：

1. 先落地纯本地、确定性强的 `std.time` 与 `std.uuid`
2. 再落地 `std.http`，单独处理网络副作用、错误映射和测试隔离

这样可以把外部网络不确定性从前两部分隔离出去，并保持回归范围可控。

## Module Design

### `std.time`

代码文件：`crates/zun-runtime/src/stdlib_native/time.rs`

导出函数：

- `time.now() -> Int`
- `time.now_ms() -> Int`
- `time.format(ts: Int, fmt: String) -> String`
- `time.parse(s: String, fmt: String) -> Int`
- `time.year(ts: Int) -> Int`
- `time.month(ts: Int) -> Int`
- `time.day(ts: Int) -> Int`
- `time.hour(ts: Int) -> Int`
- `time.minute(ts: Int) -> Int`
- `time.second(ts: Int) -> Int`
- `time.weekday(ts: Int) -> String`
- `time.add_days(ts: Int, n: Int) -> Int`
- `time.diff_days(ts1: Int, ts2: Int) -> Int`

语义约束：

- 所有时间戳统一按 Unix 秒级时间戳处理，基准时区固定为 UTC
- `now_ms()` 返回毫秒级整数，但其他时间分解与格式化函数仍以秒级时间戳为输入
- `format` 与 `parse` 使用 `chrono` 的 `strftime`/`strptime` 兼容格式
- `parse("1970-01-01", "%Y-%m-%d")` 必须返回 `0`
- `add_days` 使用 UTC 日历日偏移，最小实现等价于秒级加减 `86400 * n`
- `diff_days(ts1, ts2)` 返回 `(ts1 - ts2) / 86400` 的整数结果，可为负

错误处理：

- 非 `Int` 时间戳参数或非 `String` 格式参数直接抛出 `Error::Interpreter`
- 无法解析的时间字符串抛出可被 `$try/$catch` 捕获的运行时错误

### `std.uuid`

代码文件：`crates/zun-runtime/src/stdlib_native/uuid.rs`

导出函数：

- `uuid.v4() -> String`
- `uuid.is_valid(s: String) -> Bool`

语义约束：

- `v4()` 使用 `uuid` crate 生成 RFC 4122 风格随机 UUID v4
- 返回值保持小写连字符格式，长度 36
- `is_valid()` 只做格式合法性判断，不额外区分版本之外的业务语义

错误处理：

- 非字符串参数抛出 `Error::Interpreter`

### `std.http`

代码文件：`crates/zun-runtime/src/stdlib_native/http_client.rs`

导出函数：

- `http.get(url: String) -> Map`
- `http.get(url: String, headers: Map) -> Map`
- `http.post(url: String, body: Map) -> Map`
- `http.post(url: String, body: Map, headers: Map) -> Map`
- `http.put(url: String, body: Map) -> Map`
- `http.delete(url: String) -> Map`
- `http.request(method: String, url: String, body: Map, headers: Map) -> Map`

返回结构统一为：

```text
{
  "status": Int,
  "body": String,
  "headers": Map
}
```

语义约束：

- 使用 `reqwest::blocking`，不引入 async runtime
- 默认超时 30 秒
- 请求头参数必须是 `Map<String, String>` 语义；运行时接受 `Map`，内部逐项校验 value 是否为 `String`
- `body` 优先接受 `Map`/`Json`，通过 `value_to_json()` 编码为 JSON 并自动附加 `content-type: application/json`
- 响应头折叠为 `Map<String, String>`；重复 header 先取逗号拼接后的字符串结果，避免丢失信息

错误处理：

- 非法 URL、连接失败、超时、请求构建失败统一映射为 `Error::Interpreter("http.*: ...")`
- 类型不匹配继续沿用当前原生模块的参数校验风格

## Code Changes

需要修改或新增的主要文件：

- 修改 [crates/zun-runtime/Cargo.toml](/Users/liangzhanbo/CodeStudio/zun/.worktrees/epic-01-stdlib-enhancement/crates/zun-runtime/Cargo.toml)
- 修改 [crates/zun-runtime/src/stdlib_native/mod.rs](/Users/liangzhanbo/CodeStudio/zun/.worktrees/epic-01-stdlib-enhancement/crates/zun-runtime/src/stdlib_native/mod.rs)
- 新增 `crates/zun-runtime/src/stdlib_native/time.rs`
- 新增 `crates/zun-runtime/src/stdlib_native/uuid.rs`
- 新增 `crates/zun-runtime/src/stdlib_native/http_client.rs`
- 新增 `tests/spec/valid/stdlib/time_functions.zun` 与 `.stdout`
- 新增 `tests/spec/valid/stdlib/uuid_functions.zun` 与 `.stdout`
- 新增 `tests/spec/valid/stdlib/http_client_*.zun` 与 `.stdout`
- 新增 `tests/spec/invalid/stdlib/http_*` 或 `time_*` 回归样例（按需要最小化补充）
- 修改 [docs/spec/security-model.md](/Users/liangzhanbo/CodeStudio/zun/.worktrees/epic-01-stdlib-enhancement/docs/spec/security-model.md)
- 修改 [docs/CHANGELOG.md](/Users/liangzhanbo/CodeStudio/zun/.worktrees/epic-01-stdlib-enhancement/docs/CHANGELOG.md)

## Testing Strategy

### `std.time`

使用确定性 spec 样例验证：

- `time.format(0, "%Y-%m-%d") == "1970-01-01"`
- `time.parse("1970-01-01", "%Y-%m-%d") == 0`
- `time.year(0) == 1970`
- `time.add_days(0, 1) == 86400`
- `time.diff_days(86400, 0) == 1`
- 对错误参数使用 `$try/$catch` 验证可捕获性

`time.now()` / `time.now_ms()` 只验证“为正整数”或“相对关系成立”，不依赖精确当前时间。

### `std.uuid`

使用 spec 样例验证：

- `uuid.v4()` 输出长度 36
- 连续调用两次结果不相同
- `uuid.is_valid(uuid.v4()) == true`
- `uuid.is_valid("not-a-uuid") == false`
- 参数类型错误可被 `$try/$catch` 捕获

### `std.http`

避免依赖外网。优先采用两层验证：

1. 在 Rust 单元测试中启动本地临时 HTTP 服务，直接验证 `http_client.rs` 的响应结构与 header 映射
2. 在 Zun spec 中仅覆盖最小稳定场景：
   - 非法 URL 进入 `$try/$catch`
   - 如果已有本地测试服务入口可复用，则增加成功路径 spec；否则先保证 Rust 层覆盖成功路径，Zun spec 保持失败路径和结构路径

这样可以满足 EPIC 目标，同时避免 `httpbin.org` 带来的 flaky 测试。

## Documentation Changes

- 在 changelog 的 `Unreleased -> Added` 中记录三个新模块
- 在安全模型中把“网络出站请求”加入副作用入口说明，并明确 `std.http` 当前默认开放

## Risks

- `time.parse` 对不含时区信息的格式必须明确按 UTC 解释，否则结果会受本机时区影响
- `std.http` 需要把 Zun `Map` 严格约束成字符串键，否则请求头和值序列化会出现模糊行为
- 外网依赖测试会导致 CI 不稳定，因此成功路径不应直接依赖公网服务

## Verification Baseline

在隔离 worktree 中，当前基线状态已验证通过：

- `cargo build`
- `cargo test`

## Approval

如果此设计保持不变，下一步进入 implementation plan，并按照任务拆分顺序先实现 `std.time` + `std.uuid`，再实现 `std.http`。
