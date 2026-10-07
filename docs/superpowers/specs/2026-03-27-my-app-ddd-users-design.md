# my-app DDD Users Design

## Summary

本设计将 `my-app` 重构为一个最小但完整的 DDD 风格示例，目标是验证 Zun 最新数据层能力已经可以支撑实际业务代码：

- 新增 `data/` 目录，使用最新 `$Type` 定义纯数据模型
- `services/` 持有假数据并提供查询全部用户、按 id 查询用户的业务函数
- `routers/` 只负责 HTTP 路由和参数转发
- `main.zun` 只做 `$HTTP(...).link("...")` 挂载
- 使用 `_password`、`_salt` 等隐藏字段验证 JSON 序列化自动过滤

本次实现只覆盖一个 `User` 上下文，不引入 repository 层，不扩展到多上下文协作。

## Goals

- 让 `my-app` 的代码结构符合当前数据层 Epic 推动的 DDD 方向
- 用真实可运行的业务示例验证 `$Type` 新语义：
  - 结构体字面量构造
  - 字段点访问
  - `_` 前缀隐藏字段的 JSON 过滤
- 提供两个可直接调用的 HTTP 接口：
  - `GET /v1/api/users`
  - `GET /v1/api/users/:id`

## Non-Goals

- 不引入真实数据库、文件存储或外部依赖
- 不新增 repository / dao 层
- 不处理用户创建、更新、删除
- 不修改语言行为；本次只消费已实现的数据层能力

## Current State

当前 `my-app` 只有三层雏形：

- `main.zun` 负责挂载 HTTP 子模块
- `routers/users_routers.zun` 暴露单个 `getUser` 路由
- `services/users_services.zun` 同时混合了 `$Type` 定义、假数据和查询逻辑

存在的问题：

- 数据模型和业务逻辑耦合在同一文件
- 服务层仍使用旧的 `$Type/$JSON` 写法，不符合当前语义
- 路由能力不足，缺少“查询全部用户”接口
- 没有实际演示 `_` 前缀隐藏字段的自动过滤能力

## Architecture

### data layer

新增 `my-app/data/users_data.zun`，只定义纯数据模型：

```zun
$Type User {
    id: Int
    name: String
    email: String
    role: String
    _password: String
    _salt: String
}
```

约束：

- 只放 `$Type`
- 不放查询函数、HTTP 路由或状态判断
- 保持“贫血模型”，只描述数据形状

### service layer

`my-app/services/users_services.zun` 负责：

- 导入 `data.users_data.User`
- 构造若干 `User { ... }` 作为假数据
- 提供业务函数：
  - `find_all_users()`
  - `find_user_by_id(id)`

行为：

- `find_all_users()` 返回用户列表，供路由直接作为 JSON 响应输出
- `find_user_by_id(id)` 在命中时返回 `User`
- 未命中时返回 `{ "Null": "No data" }`

服务层是唯一持有“模拟数据库数据”的地方，避免 router 直接接触数据细节。

### router layer

`my-app/routers/users_routers.zun` 只负责定义 HTTP 接口：

- `GET /users`
- `GET /users/:id`

router 只做这几件事：

- 导入 service 模块
- 从路由参数中获取 `id`
- 调用 service 并返回结果

不在 router 中定义 `$Type`，不在 router 中构造假数据。

### app entry

`my-app/main.zun` 保持最小入口：

```zun
$main() {
    $HTTP("/v1/api").link("routers.users_routers");
}
```

应用入口不承担业务逻辑。

## Data Flow

### GET /v1/api/users

1. 请求进入 `users_routers` 模块
2. route 调用 `users_services.find_all_users()`
3. service 构造并返回 `List<User>`
4. HTTP JSON 序列化执行时自动过滤 `_password`、`_salt`

### GET /v1/api/users/:id

1. 请求进入 `users_routers` 模块
2. route 将路径参数 `id` 传给 `users_services.find_user_by_id(id)`
3. service 遍历假数据并查找匹配项
4. 命中时返回 `User`
5. 未命中时返回 `{ "Null": "No data" }`

## API Contract

### List users

Route:

```text
GET /v1/api/users
```

Success body example:

```json
[
  {
    "id": 1,
    "name": "John",
    "email": "john@example.com",
    "role": "admin"
  }
]
```

### Get user by id

Route:

```text
GET /v1/api/users/:id
```

Success body example:

```json
{
  "id": 1,
  "name": "John",
  "email": "john@example.com",
  "role": "admin"
}
```

Not found body example:

```json
{
  "Null": "No data"
}
```

## Example Fake Data

服务层至少准备 2 到 3 条用户数据，覆盖不同角色，示例：

- `id = 1`, `role = "admin"`
- `id = 2`, `role = "editor"`
- `id = 3`, `role = "viewer"`

每条记录都包含：

- 公开字段：`id`, `name`, `email`, `role`
- 隐藏字段：`_password`, `_salt`

这样既能满足查询示例，也能验证隐藏字段不会进入 HTTP JSON 响应。

## Error Handling

- 查询全部用户不引入错误分支；直接返回列表
- 按 id 查询未命中时，不返回 HTTP 404，本示例保持 `200` 并返回：
  - `{ "Null": "No data" }`
- 本设计不引入 `$RES(...)` 状态码分支，避免让示例关注点偏离数据层验证

## Testing Strategy

先测试后实现，至少覆盖以下行为：

1. `GET /v1/api/users`
   - 返回 JSON 数组
   - 每个元素包含公开字段
   - 响应中不包含 `_password`
   - 响应中不包含 `_salt`

2. `GET /v1/api/users/1`
   - 返回单个用户对象
   - 字段值与假数据一致
   - 隐藏字段未出现在响应中

3. `GET /v1/api/users/999`
   - 返回 `{ "Null": "No data" }`

测试方式优先使用现有 Rust integration test 模式，直接启动 `my-app` 并发起真实 HTTP 请求。

## Implementation Notes

- `.zun` 代码必须使用当前 `$Type` 结构体字面量语法，不再使用旧的字符串 key 写法
- 若 service 需要返回列表，直接返回 Zun `List`，由现有 HTTP JSON 转换处理
- 若模块导入方式与当前 `my-app` 相对路径解析有差异，优先遵循当前模块系统实现，不额外发明命名规则

## Risks

### Route parameter shape

`GET /users/:id` 依赖当前 HTTP 路径参数行为。如果 `my-app` 现有写法和 runtime 实现存在差异，应以当前可执行集成测试为准调整 router 形态。

### JSON generic annotations

函数和 HTTP handler 的返回类型注解当前对 `JSON<...>` 的 enforcement 仍偏宽松，因此本示例应优先保证运行行为正确，而不是在类型注解上追求额外复杂度。

## Acceptance Criteria

- `my-app` 目录新增 `data/users_data.zun`
- `services/users_services.zun` 只保留业务逻辑和假数据
- `routers/users_routers.zun` 只保留 HTTP 路由
- `main.zun` 继续用 `$HTTP(...).link("...")` 挂载
- `GET /v1/api/users` 正常返回列表
- `GET /v1/api/users/:id` 命中返回用户对象
- `GET /v1/api/users/:id` 未命中返回 `{ "Null": "No data" }`
- 所有 HTTP JSON 响应中都不出现 `_password` 和 `_salt`
