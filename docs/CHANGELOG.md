# Changelog

本文件记录 Zun 的用户可见变化。

记录规则：

- 只记录用户可见变化
- 每次发布都按固定栏目归档
- breaking change、弃用、移除必须写清兼容影响

## Unreleased

### Added

- 建立 `docs/spec/` 正式语言规范目录
- 增加 Phase 9 的版本、兼容性、弃用规则文档
- 增加 `std.str`、`std.math`、`std.json` 原生模块，可通过 `$mod std.*;` 使用常见字符串、数学与 JSON 能力
- 新增 `$Type` 自定义类型声明，支持描述 JSON 数据形状（字段名、类型、可选标记 `?`）
- 函数与 HTTP handler 返回类型注解支持 `JSON<TypeName>` 形式（如 `-> JSON<User>`）
- 增加 `std.time` 原生模块，提供 Unix 时间戳、UTC 格式化/解析、日期偏移与日期字段读取能力
- 增加 `std.uuid` 原生模块，提供 UUID v4 生成与格式校验能力
- 增加 `std.http` 原生模块，提供同步 HTTP 客户端能力并统一返回 `{ status, body, headers }` 结构
- 增加 `std.sqlite` 与 `std.postgres` 原生模块，并引入 `Connection.query(...)` / `Connection.execute(...)` / `Connection.close()` 数据库句柄能力
- `serve` 模式新增 `@SET_HDR(...)` 与 `@CORS(...)` 配置能力，可按全局 / HTTP 块 / 路由控制响应头与 CORS 策略
- 增加 `std.auth.session`、`std.auth.jwt`、`std.auth.password`、`std.auth.guard` 原生模块
- `package.toml` 新增 `[server.auth]` 配置树，支持 session / JWT / 授权规则与 session store driver 配置
- `serve` 模式新增请求级认证入口，支持 Cookie Session 与 `Authorization: Bearer` JWT
- Session store 新增 `memory`、`sqlite`、`postgres` 三种后端
- `std.auth.jwt` 新增 refresh token 签发与校验 API：`sign_refresh(...)` / `verify_refresh(...)`
- `std.auth.jwt.refresh_pair(...)` 现在会轮换消费 refresh token，旧 token 重放会被拒绝
- refresh token 现在有独立 store，可随 auth backend 持久化，并支持 `std.auth.jwt.revoke_refresh(...)` 主动失效
- JWT 现在支持按配置选择 `HS256` / `HS384` / `HS512`，非法算法会在启动时被拒绝
- 路由授权规则现在支持前缀通配、具体规则优先级，以及 `claims_all` claims 精确匹配约束
- `std.auth.guard` 新增批量角色/权限辅助：`has_any_*` / `has_all_*` / `require_any_*` / `require_all_*`
- 新增可直接运行的 [examples/http-auth](/Users/liangzhanbo/CodeStudio/zun/examples/http-auth) 示例工程，覆盖登录、刷新、登出、受保护路由与 session/JWT 双入口
- 新增 `std.auth.csrf` 原生模块，并为 cookie session 写请求接通默认 CSRF 校验
- `std.fs` 现在补齐 `write(...)` / `append(...)` 主 API，并在 `run` 与 `serve` 模式下统一按项目根解析相对路径

### Changed

- 规范化语言行为文档与 `tests/spec` 的映射关系
- 将 stdlib native module 注册从 `intrinsics.rs` 拆分到 `src/runtime/stdlib/`，为后续标准库扩展提供结构化入口
- `serve` 请求链路现在会在进入 handler 前解析当前 principal，并把认证态注入请求级 auth context
- 受保护路由当前支持 `require = "authenticated"`、`roles_any`、`roles_all`、`permissions_any`、`permissions_all` 规则
- session cookie 配置现在会在启动期拒绝不安全组合：非法 `cookie_same_site`，以及 `SameSite=None` 但未启用 `Secure`
- session rotation 配置现在会在启动期拒绝非法模式；`rotation = "always"` 下重新登录会立即失效旧 cookie，避免旧 session 继续可用
- 文件读写文档主路径现在切换到 `std.fs`；`$>>FILE(...)` / `$<<FILE(...)` 已移除，继续使用会报 `ZUN-P008`

### Deprecated

- None

### Removed

- None

### Fixed

- 对齐 spec 样例与当前 parser/runtime 真实行为
- `zun run <file.zun>` 现在按真正的脚本模式执行，不再因为文件位于项目目录内而继承上层 `package.toml` 的 auth / session store 初始化；项目内工具脚本可脱离 `serve` 配置单独运行
- 修复 `std.postgres` 在 `serve` 模式 HTTP handler 中调用 `Connection.close()` 时触发的 Tokio runtime 嵌套 panic，真实 HTTP + PostgreSQL 查询链路现在可正常工作
- HTTP handler 在真实 `serve` 请求链路中的路径参数、query 参数与 `$HDR(...)` 请求头读取行为已补齐并覆盖集成测试
- `RuntimeMode::Test` 下 HTTP handler 注册阶段现在会执行返回类型校验；同时普通 `$fn` 与 HTTP handler 共用同一套返回类型验证逻辑
- `$RES(status, body)` HTTP 状态码修复：之前 status 参数被忽略，响应始终为 HTTP 200；
  现已正确返回指定状态码（如 404、201、500 等）
- HTTP handler 中未捕获的 `$throw` 在客户端侧稳定返回 HTTP 500 错误响应
- `serve` 认证失败与授权失败现在会分别返回 HTTP 401 / 403
- `serve` 认证失败与授权失败现在统一返回稳定 JSON 结构：`status` / `code` / `message`

### Changed（P0）

- `$throw` 未捕获时终端行为：HTTP handler 中未被 `$try/$catch` 捕获的 `$throw`
  现在会在终端打印 `[ERROR] uncaught throw: <value>`，便于开发调试；客户端仍收到 HTTP 500
- HTTP 服务器内部架构：引入 `HttpBackend` trait，Axum 实现封装至 `AxumBackend`，
  `server.rs` 降为纯协调层（对用户无感知，不影响任何 .zun 语法）

## Release Template

后续版本请按如下模板追加：

```md
## x.y.z - YYYY-MM-DD

### Added

- ...

### Changed

- ...

### Deprecated

- 当前行为：
- 替代行为：
- 起始版本：
- 计划移除版本：

### Removed

- ...

### Fixed

- ...
```
