# Serve Auth Platform Design

## Summary

为 Zun `serve` 模式设计一套内置的认证与授权平台能力，首发聚焦以下目标：

- 内置标准库提供 `session`、`jwt`、`password`、`guard` 能力
- `serve` 模式下通过 `package.toml` 统一声明 auth 配置
- 支持 `cookie session` 与 `JWT bearer token`
- 支持 `memory`、`sqlite`、`postgres` 三种 session store
- 支持路由级认证与授权规则，包括 authenticated、roles、permissions

本次设计遵循一个关键原则：auth 是 Zun runtime 与 stdlib 的原生能力扩展，不是对外部 Web 框架配置的语法映射。`package.toml` 只负责默认策略与接线，项目仍然可以在 Zun handler 中通过 `std.auth.*` 显式控制认证和授权流程。

## Goals

- 为 `serve` 模式提供统一、可配置、可测试的 auth/authz 能力
- 让 session 与 JWT 都能同时作为 Zun 官方支持的身份载体
- 让 auth 行为可以通过 `package.toml` 配置，而不是散落在业务代码中
- 为 Zun 项目提供一致的 `principal` 身份对象模型
- 保持 runtime、stdlib、HTTP 集成三者职责清晰
- 在不引入新语法的前提下完成第一阶段 auth 平台设计

## Non-Goals

- OAuth、OIDC、SAML 或第三方社交登录
- 分布式缓存型 session store，如 Redis
- 完整策略语言或 ABAC 引擎
- 新增 HTTP auth 专用语法或注解
- 多租户 IAM 平台级能力
- 浏览器前端 SDK 或客户端刷新令牌自动协商机制

## Current State

当前仓库已经具备以下相关基础：

- HTTP 路由注册、请求头注入、请求体注入、响应状态码返回
- `@SET_HDR(...)` 与 `@CORS(...)` 等静态 HTTP 元数据能力
- runtime intrinsic 与 stdlib native 模块机制
- `std.sqlite` 与 `std.postgres` 这类宿主能力接入路径
- `package.toml` 配置与 `serve` 模式执行入口

当前缺失的部分包括：

- 没有内置 `session`、`jwt`、`password`、`guard` 标准库
- 没有请求级 `principal` 认证态上下文
- 没有 cookie 解析、session store、bearer token 校验的统一 runtime 管线
- 没有路由级 authz 规则配置
- 没有把 HTTP 请求区分为 `401` 与 `403` 的统一 auth 语义

## Design Principles

### 1. Auth Must Be Runtime-Native

auth 不能只是把外部框架的配置项翻译到 Zun 语法中。首发设计必须把认证对象模型、请求级上下文、标准库接口和授权判定都定义在 Zun runtime 里。

### 2. Config Declares Defaults, Stdlib Controls Behavior

`package.toml` 负责默认策略、密钥来源、session store 选择和路由保护规则；项目代码通过 `std.auth.*` 执行登录、登出、签发 token、显式 guard 判断。

### 3. One Unified Principal Model

不管身份来自 session 还是 JWT，handler 侧都应看到统一的 `principal` 结构，而不是被迫理解底层凭证来源差异。

### 4. Session Store Is A Pluggable Runtime Capability

session store 不是业务项目自己拼 SQL 的数据表，而是 runtime 管理的宿主能力。首发支持 `memory`、`sqlite`、`postgres`，三者共享统一字段模型和接口。

### 5. Route Protection Should Be Explicit But Thin

授权规则以配置驱动为主，只覆盖静态入口条件，例如 authenticated、roles、permissions。更复杂的业务授权仍然交给 Zun handler 和 `std.auth.guard`。

## Architecture

整体架构拆为四层：

1. `serve config layer`
   - 从 `package.toml` 读取 auth 配置
   - 定义默认认证方案、session store、JWT 参数、授权规则

2. `runtime auth core`
   - 负责请求级身份提取、session 加载、JWT 校验、principal 构建、授权判定

3. `stdlib auth modules`
   - 通过 `std.auth.session`、`std.auth.jwt`、`std.auth.password`、`std.auth.guard` 暴露项目可编程接口

4. `HTTP integration layer`
   - 将 auth core 接入 `serve` 模式的 HTTP 请求生命周期
   - 负责 cookie 回写、401/403 响应、请求级上下文注入

这四层的职责划分如下：

- 配置层不承担认证执行
- runtime core 不依赖 Zun 语法扩展
- stdlib 不直接实现底层持久化细节
- HTTP 层只负责接线，不负责定义业务授权语义

## Configuration Model

auth 配置挂在 `serve` 下，推荐结构如下：

```toml
[serve]
host = "127.0.0.1"
port = 8080

[serve.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie", "bearer"]

[serve.auth.session]
enabled = true
cookie_name = "zun_session"
cookie_secure = true
cookie_http_only = true
cookie_same_site = "lax"
cookie_path = "/"
ttl_seconds = 86400
idle_timeout_seconds = 7200
rotation = "on_login"

[serve.auth.session.store]
driver = "postgres"

[serve.auth.session.store.postgres]
url_env = "SESSION_DATABASE_URL"
table = "auth_sessions"

[serve.auth.jwt]
enabled = true
issuer = "zun-app"
audience = "zun-api"
algorithm = "HS256"
secret_env = "JWT_SECRET"
access_ttl_seconds = 3600
refresh_ttl_seconds = 2592000

[serve.auth.authorization]
enabled = true
default = "public"

[[serve.auth.authorization.rules]]
method = "GET"
path = "/admin/*"
require = "authenticated"
roles_any = ["admin"]

[[serve.auth.authorization.rules]]
method = "POST"
path = "/api/posts"
require = "authenticated"
permissions_all = ["post:create"]
```

### Config Rules

- 所有密钥优先通过 `*_env` 提供
- `session` 与 `jwt` 可以同时启用
- `identity_sources` 决定请求解析顺序
- `authorization.default` 建议默认为 `public`
- 路由规则按更具体路径优先，再按声明顺序匹配

### Startup Validation

以下情况应在 `serve` 启动时直接失败：

- 启用了 JWT 但缺少 secret 或 secret_env
- 启用了持久化 session store 但缺少连接配置
- TTL、idle timeout 或 refresh TTL 为非法值
- store driver 名称不受支持
- 授权规则字段互相冲突或不合法

## Principal Model

runtime 对外统一暴露 `principal`：

```json
{
  "subject": "user_123",
  "scheme": "session",
  "roles": ["admin"],
  "permissions": ["post:create"],
  "claims": { "team_id": "t_001" },
  "session_id": "sess_xxx"
}
```

设计约束如下：

- `subject` 是统一身份标识
- `scheme` 取值至少包括 `session`、`bearer`
- `roles` 与 `permissions` 统一进入 principal
- `claims` 容纳项目自定义扩展字段
- 只有 session 认证态会带 `session_id`

JWT 与 session 必须都能映射到这套统一结构，否则 handler 将被迫处理双重语义。

## Session Model

session 不是任意 Map，而是 runtime 管理的结构化对象。建议统一字段模型：

- `session_id`
- `subject`
- `roles`
- `permissions`
- `claims`
- `csrf_token`
- `issued_at`
- `expires_at`
- `idle_timeout_at`
- `created_at`
- `updated_at`

### Session Store Drivers

首发支持：

- `memory`
- `sqlite`
- `postgres`

三种 driver 共用一套语义：

- 根据 session id 加载会话
- 校验绝对过期时间
- 校验 idle timeout
- 支持 create / update / rotate / destroy
- 支持显式失效和 cookie 清除

### Store Interface

session store 是 runtime 内部抽象，不直接暴露底层连接细节给 Zun 项目。接口语义至少包含：

- `get(session_id)`
- `create(session)`
- `update(session)`
- `rotate(old_session_id, new_session)`
- `delete(session_id)`

## JWT Model

JWT 首发承担双重角色：

- 作为 HTTP bearer token 认证路径
- 作为标准库可直接调用的通用签发/校验能力

### JWT Validation Rules

- 校验签名算法
- 校验 `issuer`
- 校验 `audience`
- 校验 `exp`
- 可选校验 `nbf`
- 校验失败时不构建 principal，并返回 `401`

### JWT Claims Mapping

建议以下 claim 为保留语义：

- `sub` -> `principal.subject`
- `roles` -> `principal.roles`
- `permissions` -> `principal.permissions`

其余字段进入 `principal.claims`。

## Standard Library API

首发设计四个模块。

### `std.auth.session`

负责当前请求的 session 读写与生命周期控制：

```zun
$ session = std.auth.session.current();
$ exists = std.auth.session.exists();
$ sid = std.auth.session.id();

std.auth.session.create({
    subject: user["id"],
    roles: ["admin"],
    permissions: ["post:create"],
    claims: { "team_id": "t_001" }
});

std.auth.session.set("cart_id", "c_001");
$ value = std.auth.session.get("cart_id");
std.auth.session.delete("cart_id");

std.auth.session.rotate();
std.auth.session.destroy();
```

### `std.auth.jwt`

负责底层 JWT 签发和校验，也提供当前请求中的 bearer 读取能力：

```zun
$ token = std.auth.jwt.sign({
    sub: "user_123",
    roles: ["admin"],
    permissions: ["post:create"]
});

$ payload = std.auth.jwt.verify(token);
$ bearer = std.auth.jwt.bearer();
$ claims = std.auth.jwt.current();
```

### `std.auth.password`

负责最小闭环的密码哈希与校验：

```zun
$ hash = std.auth.password.hash("plain-password");
$ ok = std.auth.password.verify("plain-password", hash);
```

### `std.auth.guard`

负责显式授权判断与 require 风格的拒绝控制：

```zun
$ principal = std.auth.guard.principal();
$ ok = std.auth.guard.authenticated();
$ ok = std.auth.guard.has_role("admin");
$ ok = std.auth.guard.has_any_role(["editor", "admin"]);
$ ok = std.auth.guard.has_permission("post:create");
$ ok = std.auth.guard.has_all_permissions(["post:create", "post:publish"]);

std.auth.guard.require_role("admin");
std.auth.guard.require_permission("billing:read");
```

`require_*` 失败时应转为 HTTP 403，而不是普通业务错误字符串。

## HTTP Request Lifecycle

一次 HTTP 请求的 auth 流程固定为以下步骤：

1. 路由命中后读取 `serve.auth` 配置
2. 按 `identity_sources` 顺序提取身份凭证
3. 从 session store 或 JWT 构建统一 principal
4. 将 principal、session、jwt claims 注入请求级上下文
5. 执行静态授权规则
6. 进入 Zun handler 执行
7. 响应阶段提交 session/cookie 变更
8. 合并响应头并返回最终 HTTP 响应

### Identity Resolution

推荐解析顺序：

- 如果 `identity_sources = ["cookie", "bearer"]`，先取 cookie session，再回退 bearer token
- 如果都不存在，则请求为匿名请求
- 如果两个来源都存在，以先命中的来源作为当前 principal

### Request-Scoped Context

当前 runtime 已经会为 handler 注入请求头和请求体。auth 扩展后，应增加专用请求级上下文槽位，而不是继续依赖匿名魔法变量。

这些上下文至少应包含：

- 当前 principal
- 当前 auth scheme
- 当前 session 对象
- 当前 JWT claims

`std.auth.*` 一律从请求级上下文读取这些信息，而不是重复解析 Cookie 或请求头。

### Session Mutation Semantics

`std.auth.session.create()`、`rotate()`、`destroy()` 不应立即回写响应，而应记录为本次请求的待提交 auth side effect，在 handler 成功返回后统一提交。

这样做的原因是：

- 避免 handler 中途报错造成半成功状态
- 保持 session store 与 `Set-Cookie` 的一致性
- 让 runtime 能统一管理 cookie 清除、旋转和写回

## Authorization Model

首发授权范围定义为轻量但完整的 route authz：

- `public`
- `authenticated`
- `roles_any`
- `roles_all`
- `permissions_any`
- `permissions_all`

### Authorization Sources

授权判断分两类：

1. 静态入口规则
   - 来源于 `package.toml`
   - 在 handler 执行前做快速判定

2. 显式业务授权
   - 来源于 `std.auth.guard.*`
   - 适用于更细粒度的业务分支

这两层能力是互补关系，不互相替代。

## Error Semantics

auth 相关错误分为三类：

### Startup Configuration Errors

服务启动失败，例如：

- JWT secret 缺失
- session store 连接配置缺失
- 授权规则格式不合法

### Authentication Errors

返回 `401`，例如：

- session cookie 缺失或无效
- session 已过期
- bearer token 无效
- JWT 签名或 claim 校验失败

### Authorization Errors

返回 `403`，例如：

- principal 有效，但角色不足
- principal 有效，但权限不足
- `std.auth.guard.require_*` 判定失败

## Implementation Routing

首发实现建议落在以下模块：

- 配置结构扩展：`crates/zun-runtime/src/config.rs`
- HTTP runtime 接入：`crates/zun-runtime/src/runtime/http.rs`
- 请求级上下文扩展：`crates/zun-runtime/src/runtime/context.rs`
- session store 抽象：`crates/zun-runtime/src/runtime/` 下新增 auth/session 相关模块
- stdlib native 暴露：`crates/zun-runtime/src/stdlib_native/`
- serve backend 集成：`crates/zun-cli/src/backends/axum_backend.rs`

### Structural Constraints

- 不新增 auth 专用语法或注解
- 不新建独立 Web framework crate
- 不把 session store 暴露成用户可自由操纵的底层数据库接口
- 不让 handler 自己通过 `$HDR(...)` 重新实现框架级 cookie 解析

## Testing Strategy

测试至少覆盖三层。

### 1. Config And Startup Validation

- `package.toml` 合法配置可启动
- 缺 secret、缺 store 配置、非法 TTL 等配置会在启动时报错

### 2. Runtime And Stdlib Semantics

- session create / get / rotate / destroy
- JWT sign / verify / invalid token
- password hash / verify
- principal 注入与 `std.auth.guard` 行为

### 3. Real HTTP Integration

- session cookie 登录后可访问 protected route
- session 过期返回 `401`
- bearer token 有效时可访问 protected route
- route rules 不满足时返回 `403`
- memory / sqlite / postgres 三种 store 语义一致
- rotate / destroy 能正确回写 cookie

## Documentation Impact

实现该设计时需要同步更新：

- `docs/spec/security-model.md`
- `docs/reference/http.md`
- `docs/reference/stdlib-api.md`
- `docs/CHANGELOG.md`

其中 `docs/spec/security-model.md` 需要明确：

- HTTP 请求级身份上下文已成为新的 runtime 行为边界
- session store 与 JWT 密钥属于高敏感配置
- auth 不提供额外 sandbox，但会把身份判断和授权失败变成统一 HTTP 语义

## Rollout Strategy

建议按以下顺序交付：

1. 配置模型与启动校验
2. principal 模型与请求级上下文
3. session store 抽象与 memory driver
4. sqlite / postgres session drivers
5. `std.auth.session` 与 `std.auth.guard`
6. `std.auth.jwt` 与 bearer token 认证
7. `std.auth.password`
8. route authz 规则与 real HTTP integration tests

这个顺序的原因是先把 runtime 主干建立稳定，再接入可选后端与授权规则，避免一开始把所有能力并行耦合到一起。

## Open Decisions Resolved

本次 brainstorming 已达成以下明确结论：

- auth 进入内置标准库
- 首发覆盖 cookie session 与 JWT
- auth 行为由 `serve` 模式下的 `package.toml` 驱动
- session store 首发支持 `memory`、`sqlite`、`postgres`
- JWT 既承担 HTTP 认证路径，也暴露通用底层签发/校验能力
- 首发范围包含授权，不只做认证
- 不通过新增语法实现 auth，先坚持 config + runtime + stdlib 路线
