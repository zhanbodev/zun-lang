# Test HTTP Database Design

## Goal

把 `test-http-database` 从“可运行的能力验证工程”升级为 Zun 官方推荐的 HTTP + Database 项目样板，并借此沉淀一套面向真实业务项目的推荐组织规范。

这套规范的核心目标不是增加框架复杂度，而是明确 Zun 项目在业务增长后的稳定组织方式：

- 推荐使用 DDD 开发思想
- 推荐先按业务域组织代码，再在域内做稳定分层
- 明确 `config` 是独立层，不把数据库连接等运行时配置散落在业务模块中
- 明确 `main.zun` 只做入口和装配，不承载业务路由实现
- 为后续文档和新项目提供一个统一、可复用、可演示的官方样板

本次工作仍然保持在“项目规范与样板重构”层面，不引入解释器新能力，也不改变 Zun 现有模块解析规则。

## Scope

本次范围包括：

- 将 `test-http-database` 重构为符合推荐规范的样板项目
- 把现有 HTTP + PostgreSQL 示例提升为官方推荐组织方式
- 明确 Zun 推荐的 DDD + layering 目录结构、依赖方向、命名约定
- 为后续文档提供可引用的真实工程样板

本次不包括：

- 修改解释器源码以支持新的项目系统能力
- 引入 `src/` 目录作为源码根
- 把推荐规范上升为编译器或 CLI 的强制规则
- 设计完整企业框架或通用 ORM
- 扩展超出当前示例范围的大量业务功能

## Design Principles

### 1. DDD Is The Recommended Mindset

Zun 推荐以 DDD 作为业务项目的默认开发思想。

这里的 DDD 不是要求一开始就引入复杂战术模式，而是强调：

- 业务代码优先围绕 domain 聚合
- 目录结构优先反映业务边界，而不是只反映技术职责
- 每个 domain 内部再保持稳定分层
- 跨域依赖应经过清晰边界，而不是自由穿透

因此，Zun 官方推荐结构不是整个项目顶层直接平铺 `routers / services / data`，而是先进入 `domains/<domain>/`，再在域内分层。

### 2. Layering Still Matters Inside A Domain

虽然推荐 DDD，但 Zun 项目仍然需要清晰分层来控制职责边界。

每个业务域内固定三层：

- `data`
- `services`
- `routers`

这三层分别承担：

- `data`: 数据访问、SQL、持久化结果转换
- `services`: 业务用例编排、聚合、错误转换
- `routers`: HTTP 协议适配、参数读取、调用 service、返回响应

### 3. Config Is A First-Class Layer

数据库连接、项目配置、环境读取属于运行时配置问题，不属于业务域，也不应被伪装成普通工具模块。

因此 Zun 推荐把这些能力集中到 `config/`：

- `config/app_config.zun`
- `config/database.zun`

如果存在跨域公用但不属于配置的工具能力，再进入 `shared/`。

### 4. Entry Files Must Stay Thin

`main.zun` 只能承担：

- 启动入口
- 全局注解或启动动作
- 路由装配

`main.zun` 不应承担：

- 某个 domain 的业务路由实现
- SQL
- 业务聚合
- 数据库连接细节

## Project System Decision

本次规范明确不引入 `src/` 目录。

原因如下：

- Zun 当前项目系统以项目根目录为模块解析基准
- 现有 `package.toml` 的 `entry` 与示例工程均围绕项目根组织
- 当前阶段优先解决业务边界与分层问题，而不是引入额外路径层级

因此，官方推荐目录仍然以项目根作为源码根：

```text
<project>/
├── package.toml
├── main.zun
├── app/
├── config/
├── domains/
├── shared/
└── README.md
```

未来如果 Zun 项目系统显式支持“源码根目录”语义，可以单独设计第二代 `src/` 布局规范，但不混入本次样板设计。

## Recommended Architecture

推荐的顶层结构如下：

```text
<project>/
├── package.toml
├── main.zun
├── app/
├── config/
├── domains/
├── shared/
└── README.md
```

各层职责如下：

- `main.zun`
  - 入口
  - 启动动作
  - 装配 `app` 层暴露的路由
- `app/`
  - 应用级装配逻辑
  - 把多个 domain router 统一组合为最终 HTTP surface
- `config/`
  - 运行时配置
  - 环境变量读取
  - 数据库连接创建
- `domains/`
  - 业务核心
  - 每个 domain 保持 `data / services / routers`
- `shared/`
  - 真实跨域公共能力
  - 不承载业务域逻辑，不承载应用配置

## Dependency Direction

推荐依赖方向如下：

```text
router -> service -> data
config -> router/service/data
shared -> router/service/data
main -> app -> domain routers
```

补充约束：

- domain A 不应直接依赖 domain B 的 `data`
- 如果一个 domain 需要另一个 domain 的业务能力，应优先依赖对方暴露的 `service`
- `main.zun` 不直接调用 domain service 获取业务数据
- `main.zun` 不直接定义某个 domain 的具体业务路由实现

## Layer Responsibilities

### `routers`

职责：

- 读取路径参数、查询参数、请求体
- 调用 service
- 决定 HTTP 返回类型

禁止：

- 写 SQL
- 创建数据库连接
- 编排复杂业务聚合

### `services`

职责：

- 封装用例
- 组合多个 `data` 查询
- 做错误转换与聚合结果组装

禁止：

- 直接声明 HTTP 路由
- 直接处理底层路由细节
- 任意穿透其他 domain 的 `data`

### `data`

职责：

- SQL
- 数据访问
- 行结果转换为领域类型或中间结果

禁止：

- 处理 HTTP
- 决定接口层语义
- 承担完整业务编排

### `config`

职责：

- 读取项目配置
- 暴露数据库连接创建与配置访问入口

禁止：

- 依赖业务 service
- 承担业务规则

### `shared`

职责：

- 真正跨域复用的公共模块

禁止：

- 因“可能未来复用”就提前抽公共
- 放置本应属于 `config` 或某个 domain 的代码

## Naming Convention

推荐目录命名统一使用复数集合名：

- `domains`
- `services`
- `routers`

`data` 保持单词不变。

推荐文件命名采用“领域名 + 职责”形式：

- `user_queries.zun`
- `user_service.zun`
- `user_router.zun`
- `order_queries.zun`
- `order_service.zun`
- `order_router.zun`

配置模块按配置主题命名：

- `app_config.zun`
- `database.zun`

这套命名与仓库现有样例更连续，也更容易被初学者一眼理解。

## Recommended Layout For `test-http-database`

目标目录结构：

```text
test-http-database/
├── package.toml
├── main.zun
├── app/
│   └── http/
│       └── routes.zun
├── config/
│   ├── app_config.zun
│   └── database.zun
├── domains/
│   ├── users/
│   │   ├── data/
│   │   │   ├── user_types.zun
│   │   │   └── user_queries.zun
│   │   ├── services/
│   │   │   └── user_service.zun
│   │   └── routers/
│   │       └── user_router.zun
│   ├── products/
│   │   ├── data/
│   │   │   ├── category_types.zun
│   │   │   ├── product_types.zun
│   │   │   └── product_queries.zun
│   │   ├── services/
│   │   │   └── product_service.zun
│   │   └── routers/
│   │       └── product_router.zun
│   └── orders/
│       ├── data/
│       │   ├── order_types.zun
│       │   └── order_queries.zun
│       ├── services/
│       │   └── order_service.zun
│       └── routers/
│           └── order_router.zun
└── shared/
    └── db/
        └── queries.zun
```

## Migration Of Current Files

对现有样板的推荐迁移如下：

- `shared/db/connection.zun` -> `config/database.zun`
- `shared/db/queries.zun` 保留在 `shared/db/queries.zun`，前提是它只承载通用查询辅助
- `main.zun` 中的订单相关 HTTP 路由移动到 `domains/orders/routers/order_router.zun`
- 新增 `app/http/routes.zun` 作为统一路由装配模块

迁移后各文件角色应变为：

- `main.zun`
  - 启动项目
  - 链接或装配 `app.http.routes`
- `app/http/routes.zun`
  - 汇总 users/products/orders 的 router
- `domains/*/routers/*.zun`
  - 各自只暴露本域 HTTP 路由
- `config/database.zun`
  - 提供数据库连接创建入口

## HTTP Surface

当前样板仍保留这些接口：

- `GET /api/users`
- `GET /api/categories`
- `GET /api/products`
- `GET /api/users/:id/orders`
- `GET /api/orders/:id`
- `GET /api/orders/:id/items`
- `GET /api/orders/:id/detail`

变化不在接口本身，而在于这些接口的代码归属：

- 所有 users 路由进入 `domains/users/routers`
- 所有 products 路由进入 `domains/products/routers`
- 所有 orders 路由进入 `domains/orders/routers`
- 装配统一通过 `app/http/routes.zun`

## Error Handling Guidance

样板继续采用“data 抛底层访问错误，service 转换为稳定业务错误”的方式。

推荐规则：

- 列表查无数据返回空列表
- 单资源不存在由 service 抛稳定错误
- 数据库连接异常与 SQL 执行异常由 service 统一转换为对上层更稳定的错误语义

`router` 层只承接这些 service 结果，不直接解释底层数据库错误细节。

## Documentation Intent

`test-http-database` 不再只是“数据库能通”的演示目录，而是承担三重角色：

- 真实可运行示例
- 官方推荐项目结构样板
- 文档中讲解 DDD + layering 时的配套案例

因此这次设计不是单纯重构目录，而是为 Zun 形成一致的项目叙事：

- 业务按 domain 拆分
- domain 内分 `data / services / routers`
- `config` 独立成层
- `main.zun` 只做装配
- 当前不引入 `src/`

## Testing Intent

样板重构后仍需保留以下验证价值：

- PostgreSQL 连接建立
- 多 domain 路由装配
- service 层聚合查询
- domain 内稳定分层调用
- `.link()` 或等效装配方式在分层项目中的可用性

本次样板的成功标准不是“目录更漂亮”，而是让一个真实可运行的 Zun HTTP + Database 项目，同时体现推荐思想、清晰分层和可复制结构。
