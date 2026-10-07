# HTTP Handler Return Type Check Design

## Goal

修复 `RuntimeMode::Test` 下 HTTP handler 返回类型静默绕过的问题，并把 HTTP handler 与普通 `$fn` 的返回类型校验收敛到同一套 runtime 规则。

本次设计覆盖两个目标：

- `tests/spec/invalid/http/http_handler_return_type_mismatch.zun` 在 test mode 下按预期失败
- 后续用户类型与 `List<User>` 等返回类型扩展不再需要分别修改 `$fn` 和 HTTP handler 两条链路

## Problem Summary

当前普通函数在 [`crates/zun-runtime/src/interpreter/exec/functions.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/exec/functions.rs) 中会在返回时执行 `validate_return_type(...)`。

但 HTTP handler 在 [`crates/zun-runtime/src/interpreter/exec/http.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/exec/http.rs) 注册时只构造 `HttpRoute` 并调用 `context.register_http_route(...)`，不会执行 handler body，因此：

- invalid spec 在 `RuntimeMode::Test` 下不会触发实际返回值检查
- 返回类型注解对 HTTP handler 只起文档作用
- `$fn` 和 HTTP handler 的类型语义已经发生分叉

## Design Decision

采用共享返回类型验证器方案：

1. 抽出一个共享的 runtime 返回类型验证器
2. 普通 `$fn` 继续在调用完成后走该验证器
3. HTTP handler 在 `RuntimeMode::Test` 注册后立即用 synthetic input 试执行一次 body
4. 试执行结果交给同一个验证器处理

不采用“把 HTTP handler 包装成临时普通函数”的方案，因为 HTTP handler 有独立的参数注入与请求上下文，强行复用函数调用接口会增加适配层复杂度并模糊语义边界。

## Architecture

### Shared Return Type Validator

新增一个共享入口，职责只包含：

- 接收主体类型名，例如 `function` 或 `http handler`
- 接收主体名称，例如 `get_user`
- 接收声明返回类型字符串
- 接收实际返回值
- 使用 `RuntimeContext` 做用户类型可见性与类型形状查询
- 产出统一的错误消息

这个验证器会成为普通 `$fn` 与 HTTP handler 的唯一返回类型比较入口。

### HTTP Test-Mode Probe

在 `RuntimeMode::Test` 下，HTTP route 注册流程增加一次 isolated probe：

- 以 route 自身保存的 `module_env` / `module_fns` 为初始状态
- 为 path params 注入 dummy `String`
- 注入空 `__headers__`
- 注入 `body = Null`
- 不修改真实 route registry 之外的运行状态
- 拿到 probe 返回值后调用共享验证器

如果 probe 失败，route 注册直接报错并中止当前脚本执行。

### Serve Mode Behavior

`serve` 模式不改行为：

- 不在注册时试执行 handler
- 不引入额外的启动时副作用
- 本次 bugfix 的行为变化只作用于 `RuntimeMode::Test`

## Type Validation Rules

### Built-in Types

共享验证器需要覆盖并统一这些内建返回类型：

- `Int`
- `Float`
- `String`
- `Bool`
- `Json`
- `List`
- `Map`
- `Response`

如果已有普通函数分支当前只实现了其中一部分，本次收敛时要以“共享规则一致”为准补齐，而不是继续保留两套不对齐的支持面。

### Missing Return

如果声明了返回类型但没有得到 `$#` 返回值，则统一报：

```text
{kind} '{name}' expects return type '{declared}' but returned nothing
```

HTTP handler 与普通函数使用同一文案结构。

### User-Defined Types

如果声明返回类型是用户类型，例如 `User`：

- 需要先确认该类型在当前上下文可见
- 实际值必须是同名 `TypedInstance`

如果类型名不可见，报：

```text
http handler 'get_user' references unknown return type 'User'
```

普通函数沿用同一文案结构，只替换主体种类。

### `List<T>`

如果声明返回类型是 `List<User>`：

- 先要求实际值为 `List`
- 再逐项验证元素是否满足 `User`

如果元素类型不匹配，报错仍然收敛到统一结构，实际类型名按运行时值类型输出。

本次不扩展新的泛型系统，只在 runtime 验证阶段支持已经存在的 `List<T>` 语义。

### Unsupported Generic Forms

`JSON<T>` 等已被 syntax/spec 明确禁止的写法不在本设计中新增运行时兼容逻辑，继续由 parser/spec 约束处理。

## Data Flow

普通函数路径：

1. `call_fn` / `call_module_fn` 执行 body
2. 收到 `Flow::Return` 或无返回结果
3. 调用共享验证器
4. 成功则返回原值，失败则抛 runtime error

HTTP handler test-mode 路径：

1. 构造 `HttpRoute`
2. 注册前检测当前 mode 是否为 `RuntimeMode::Test`
3. 若是，则执行 isolated probe
4. probe 返回结果走共享验证器
5. 成功才注册 route，失败则直接中止

## Error Handling

probe 执行中的错误处理遵循当前 handler 执行模型：

- handler body 自身报错则直接返回该错误
- 返回类型不匹配则由共享验证器生成标准错误
- 不吞掉 probe 过程中的 runtime error

这样 invalid spec 能在 route 注册时直接得到稳定错误，而不是静默成功。

## Files Affected

- [`crates/zun-runtime/src/interpreter/exec/functions.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/exec/functions.rs)
  - 抽出共享返回类型验证器，普通 `$fn` 改为委托调用
- [`crates/zun-runtime/src/interpreter/exec/http.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/exec/http.rs)
  - 在 test mode 路由注册流程中增加 HTTP probe
- [`crates/zun-runtime/src/interpreter/mod.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/interpreter/mod.rs)
  - 暴露共享验证器或 probe 所需接口
- [`crates/zun-runtime/src/runtime/context.rs`](/Users/liangzhanbo/CodeStudio/zun/crates/zun-runtime/src/runtime/context.rs)
  - 仅在需要时补充 mode 读取辅助接口
- [`tests/spec/invalid/http/http_handler_return_type_mismatch.zun`](/Users/liangzhanbo/CodeStudio/zun/tests/spec/invalid/http/http_handler_return_type_mismatch.zun)
  - 保持为主回归 fixture
- [`tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun`](/Users/liangzhanbo/CodeStudio/zun/tests/spec/invalid/http/http_handler_unknown_user_type_without_import.zun)
  - 作为用户类型未知错误的回归 fixture
- `tests/spec/valid/http/*` 或相邻 HTTP fixture
  - 增加一个用户类型或 `List<User>` 的正向覆盖

## Testing

必须覆盖：

- invalid spec:
  - `http_handler_return_type_mismatch.zun`
  - `http_handler_unknown_user_type_without_import.zun`
- valid spec:
  - 至少一个 HTTP handler 返回用户类型或 `List<User>` 的成功样例
- targeted runtime/unit tests:
  - 共享验证器的 built-in / user-type / missing-return 行为

验证命令至少包括：

```bash
cargo test invalid_spec_samples_fail_with_expected_error
```

以及针对新增 runtime/unit tests 的更小范围命令。

## Scope

本次只修复返回类型校验缺失并统一验证入口，不改变 `serve` 期运行语义，也不引入新的语法能力。
