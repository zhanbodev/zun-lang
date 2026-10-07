# Test HTTP Database Admin UI Design

## Goal

在 `test-http-database` 项目上新增一个纯 Zun 驱动的后台管理前端样板，验证 Zun 在以下场景中的完整项目能力：

- 使用 `$HTML().link("...")` 返回页面
- 使用 `$STATIC(...)` 暴露静态资源
- 使用 `app/pages` 与 `app/public` 组织页面与前端资源
- 前后端分离
- 前端通过浏览器请求后端 API
- 后端继续遵守已确定的 DDD + layering 结构
- 在真实业务场景下提供查询和 CRUD 页面能力

本次设计的重点不是做一个完整产品，而是把 `test-http-database` 从“后端 API 样板”扩展为“带管理后台页面的全链路样板”。

## Scope

本次范围包括：

- 在 `test-http-database` 中新增后台单页入口
- 约定 `app/pages/` 作为静态页面目录
- 约定 `app/public/` 作为静态资源目录
- 使用 Zun HTTP 路由返回页面入口
- 使用前端 JS 主动请求 `/api/...`
- 为 `users` 与 `products` 提供完整 CRUD 页面能力
- 为 `orders` 提供只读运营视图页面能力

本次不包括：

- 登录、鉴权、会话管理
- 文件上传
- 复杂分页、排序 DSL、模糊搜索 DSL
- 订单写操作
- 通用前端框架、打包器或第三方 JS 工具链
- 改动 Zun 解释器或标准库能力

## Design Principles

### 1. Frontend And Backend Stay Separated

本次后台样板采用前后端分离模式：

- Zun 后端负责页面入口、静态资源暴露、业务 API
- 前端 JS 负责状态管理、请求发送、DOM 更新、CRUD 交互

边界明确为：

- 页面入口：`GET /admin`
- 静态资源：`/assets/...`
- 业务 API：`/api/...`

### 2. Page Layer Must Not Pollute Domain Layer

后台页面属于应用层能力，不属于某个业务 domain。

因此页面相关内容只能放在：

- `app/pages/`
- `app/public/`
- `app/http/`

而不能混入：

- `domains/*/routers`
- `domains/*/services`
- `domains/*/data`

`domains/*/routers` 继续只负责业务 API。

### 3. Single-Page Admin Is The Preferred V1 Shape

第一版后台样板使用单页后台：

- 一个入口页面 `admin.html`
- 一个前端脚本 `admin.js`
- 一个样式文件 `admin.css`

页面内部通过 JS 切换：

- `users`
- `products`
- `orders`

这样更适合作为官方样板：

- 页面结构更统一
- 更符合“前后端分离 + 前端请求 API”
- 后续扩展新模块时不需要重复页面壳

### 4. CRUD Scope Should Be Practical

虽然目标包含增删改查，但所有业务域不应一步做成同样复杂度。

本次明确分两档：

- 完整 CRUD：`users`、`products`
- 只读视图：`orders`

原因：

- `users` / `products` 适合做后台管理样板
- `orders` 写操作会迅速引入订单项、状态流转等复杂业务，不适合第一版样板

## Recommended Layout

在现有 `test-http-database` 结构上，新增页面层目录如下：

```text
test-http-database/
├── app/
│   ├── http/
│   │   └── routes.zun
│   ├── pages/
│   │   └── admin.html
│   └── public/
│       ├── css/
│       │   └── admin.css
│       └── js/
│           └── admin.js
├── config/
├── domains/
├── shared/
├── main.zun
└── package.toml
```

职责如下：

- `app/http/routes.zun`
  - 页面入口
  - 静态资源暴露
  - 业务 API router 装配
- `app/pages/admin.html`
  - 后台页面壳
- `app/public/css/admin.css`
  - 后台样式
- `app/public/js/admin.js`
  - 前端请求、状态与渲染逻辑

## Routing Strategy

推荐把路由拆成三类：

### 1. Page Route

页面入口只提供单页后台壳：

- `GET /admin`

返回方式：

```zun
$# $HTML().link("app.pages.admin");
```

### 2. Static Asset Route

静态资源统一通过：

```zun
$STATIC("/assets", "app/public");
```

这样浏览器可访问：

- `/assets/css/admin.css`
- `/assets/js/admin.js`

### 3. Business API Routes

业务 API 继续沿用当前 DDD 分层组织：

- `$HTTP("/api").link("domains.users.routers.user_router")`
- `$HTTP("/api").link("domains.products.routers.product_router")`
- `$HTTP("/api").link("domains.orders.routers.order_router")`

因此页面与 API 的分工为：

- `/admin` 负责加载单页后台
- `/assets/...` 负责加载前端资源
- `/api/...` 负责前端 CRUD 与查询请求

## Single-Page Admin Structure

后台页面为单页应用，但不引入外部前端框架。

推荐页面内部分为这些区域：

- 顶部标题区
- 导航区
- 列表区
- 详情/编辑表单区
- 操作消息区

推荐根节点结构：

- `#app`
- `#toolbar`
- `#content`
- `#modal`
- `#message`

这些节点由 `admin.html` 提供，`admin.js` 接管渲染与交互。

## Frontend Asset Responsibilities

### `admin.html`

只负责：

- 页面骨架
- 标题与挂载点
- 资源引用

不负责：

- 大段内联业务逻辑
- 直接写 CRUD 交互实现

### `admin.js`

只负责：

- 当前模块状态
- API 请求封装
- 列表渲染
- 表单渲染与提交
- 编辑、删除、切换 tab
- 消息提示

推荐的最小前端状态模型：

- `currentTab`
- `listData`
- `selectedItem`
- `formMode`
- `loading`
- `errorMessage`

### `admin.css`

只负责：

- 布局
- 表格
- 表单
- 弹窗或抽屉
- 消息提示样式

## Data Flow

后台页面的数据流如下：

1. 浏览器访问 `/admin`
2. 后端返回 `admin.html`
3. 浏览器加载 `/assets/css/admin.css`
4. 浏览器加载 `/assets/js/admin.js`
5. 前端初始化默认 tab
6. 前端请求对应 `/api/...`
7. 后端返回 JSON
8. 前端渲染表格/详情/表单
9. 新增、编辑、删除成功后，前端重新拉取当前列表

关键约束：

- 前端只依赖 HTTP API
- 不直接依赖 Zun domain 内部模块
- 页面与业务逻辑唯一边界是 `/api/...`

## API Scope

### Users

第一版完整 CRUD：

- `GET /api/users`
- `GET /api/users/:id`
- `POST /api/users`
- `PUT /api/users/:id`
- `DELETE /api/users/:id`

### Products

第一版完整 CRUD：

- `GET /api/products`
- `GET /api/products/:id`
- `POST /api/products`
- `PUT /api/products/:id`
- `DELETE /api/products/:id`

为页面中的分类选择保留：

- `GET /api/categories`

### Orders

第一版只读：

- `GET /api/orders`
- `GET /api/users/:id/orders`
- `GET /api/orders/:id`
- `GET /api/orders/:id/items`
- `GET /api/orders/:id/detail`

本次明确不做：

- `POST /api/orders`
- `PUT /api/orders/:id`
- `DELETE /api/orders/:id`

## Layering Rules

为了维持已批准的项目规范，本次新增页面层后，依赖方向继续保持：

```text
page route -> HTML/static asset
frontend JS -> /api/*
router -> service -> data
config -> router/service/data
shared -> router/service/data
```

新增约束：

- 页面路由只放在 `app/http/*`
- domain router 只放业务 API
- domain service 不感知 HTML、CSS、JS
- 前端资源不直接导入 domain 模块

## Error Handling

错误处理分三层：

### API Layer

后端继续返回稳定错误语义，不把底层数据库错误直接暴露给页面脚本。

### Frontend Request Layer

`admin.js` 应统一封装请求函数：

- 成功返回数据
- 失败返回统一错误对象或错误消息

### Page Interaction Layer

页面交互规则：

- 操作成功后显示成功消息
- 操作失败后显示统一错误消息
- 新增/编辑/删除成功后刷新当前列表

第一版不采用浏览器原生 `alert` 作为主要错误提示方式。

## Validation Strategy

验证分三层：

### 1. API Verification

使用 `curl` 验证 CRUD 与查询接口。

### 2. Page Load Verification

验证：

- `/admin` 可以正常打开
- `/assets/js/admin.js` 与 `/assets/css/admin.css` 可加载

### 3. Frontend Interaction Verification

在页面中验证：

- 查询
- 新建
- 编辑
- 删除
- tab 切换

## Implementation Phasing

推荐按以下顺序实现：

### Phase 1

先打通页面壳：

- `/admin`
- `admin.html`
- `admin.css`
- `admin.js`
- `$STATIC(...)`

### Phase 2

实现 `products` 完整 CRUD 页面与 API。

### Phase 3

实现 `users` 完整 CRUD 页面与 API。

### Phase 4

实现 `orders` 只读视图页面。

这样可以尽快看到前后端分离链路跑通，又不会一开始就陷入订单域复杂度。

## Success Criteria

本次后台样板完成后，应满足：

- `GET /admin` 能返回后台单页入口
- 静态资源通过 `/assets/...` 可访问
- 前端页面可切换 `users / products / orders`
- `users` 与 `products` 支持完整 CRUD 页面交互
- `orders` 支持列表/详情只读页面交互
- 前端只通过 HTTP API 与后端通信
- 后端仍保持 DDD + layering 结构

## Non-Goals

为了防止样板过重，本次明确不是：

- 企业级后台框架
- 完整权限系统
- 通用组件库
- 通用 ORM
- 通用前端构建平台

本次目标始终是：

一个纯 Zun、结构清晰、可演示、可继续扩展的后台管理样板。
