# Auth News Sample Design

## Goal

在现有 `sample/auth-b2b-portal` 基础上新增一个“真实但克制”的新闻网站模块，用来继续验证 Zun 在以下组合场景下的项目能力：

- 公共前台页面与后台管理页面并存
- cookie session 与 bearer JWT 两条鉴权链路继续可用
- 角色与权限不仅用于“项目创建”示例，也能覆盖真实内容管理
- CSRF 在后台写操作中继续生效
- Postgres 业务数据与 auth store 并存

本次设计不是做完整 CMS，而是把现有 auth sample 从“认证能力样板”扩展成“带内容业务的真实网站样板”。

## Scope

本次范围包括：

- 新增前台新闻站页面
- 新增后台新闻管理页面
- 新增新闻业务 API
- 扩展用户角色矩阵到 `admin` / `editor` / `member`
- 新增新闻分类与文章表
- 用新闻业务重新验证 `401` / `403` / CSRF / session / bearer / 权限映射

本次不包括：

- 图片上传
- 富文本编辑器
- 评论系统
- 搜索、分页、标签、推荐算法
- 多作者协同审批流
- 媒体库、草稿版本历史

## Product Shape

样例扩展为两套界面：

### 1. Public News Site

公共前台不要求登录即可访问：

- `GET /news`
- `GET /news/:slug`

它只展示 `published` 状态文章，模拟真实新闻站首页与详情页。

### 2. News Admin Console

后台管理页要求登录：

- `GET /news-admin`

后台页面继续使用纯 HTML + 静态 CSS/JS，不引入前端框架。  
它通过浏览器调用 `/api/admin/news...` 路由，实现：

- 列出文章
- 创建草稿
- 编辑文章
- 发布文章
- 撤回文章

## Design Principles

### 1. News Module Must Strengthen Auth Validation

新闻模块的价值不在“多一个业务 demo”，而在于让 auth 权限验证从单点示例扩展成真实业务矩阵。

因此它必须同时覆盖：

- public read
- authenticated admin page access
- role/permission write checks
- cookie CSRF
- bearer write requests

### 2. Public And Admin Views Must Share The Same Source Data

前台与后台都以同一张 `news_articles` 表为准：

- 前台只读 `published`
- 后台可读 `draft` + `published`

这样才能验证“发布前不可见，发布后可见”。

### 3. Permissions Stay Flat And Explicit

本次不引入复杂 RBAC 表。

继续沿用当前 sample 的做法：

- 用户表存 `role`
- Zun 服务层把 `role` 映射为 `roles` + `permissions`

这样更适合样例工程，也能直接看清权限行为。

### 4. Frontend Remains Lightweight

前台和后台都使用：

- `$HTML().link("...")`
- `$STATIC("/assets", "app/public")`
- 原生浏览器 `fetch`

这样继续验证 Zun 当前页面组织方式，而不是引入外部前端工具链。

## Recommended Layout

在现有 `sample/auth-b2b-portal` 中新增如下内容：

```text
sample/auth-b2b-portal/
├── app/
│   ├── pages/
│   │   ├── console.html
│   │   ├── news.html
│   │   ├── news_detail.html
│   │   └── news_admin.html
│   ├── public/
│   │   ├── css/
│   │   │   ├── console.css
│   │   │   ├── news.css
│   │   │   └── news_admin.css
│   │   └── js/
│   │       ├── console.js
│   │       ├── news.js
│   │       ├── news_detail.js
│   │       └── news_admin.js
│   └── router/
│       ├── auth_router.zun
│       ├── account_router.zun
│       ├── admin_router.zun
│       ├── project_router.zun
│       ├── news_page_router.zun
│       └── news_api_router.zun
├── services/
│   ├── auth.zun
│   ├── permissions.zun
│   ├── users.zun
│   └── news.zun
└── scripts/
    ├── hash_password.zun
    └── smoke.sh
```

职责约束：

- `news_page_router.zun`
  - 前台页面与后台页面壳
- `news_api_router.zun`
  - 新闻公开 API 与后台 API
- `news.zun`
  - 新闻查询、创建、编辑、发布、撤回
- `news*.html/css/js`
  - 页面壳与前端交互

## Routing Design

### Page Routes

- `GET /`
  - 保留现有 auth console
- `GET /console`
  - 保留现有 auth console
- `GET /news`
  - 前台新闻列表页
- `GET /news/:slug`
  - 前台新闻详情页
- `GET /news-admin`
  - 新闻后台管理页

### Public News API

- `GET /api/news`
  - 返回已发布新闻列表
- `GET /api/news/:slug`
  - 返回单篇已发布新闻详情

### Admin News API

- `GET /api/admin/news`
  - 返回后台文章列表
  - 要求 `news:edit`
- `POST /api/admin/news`
  - 创建草稿
  - 要求 `news:create`
- `PUT /api/admin/news/:id`
  - 编辑草稿或已发布文章
  - 要求 `news:edit`
- `POST /api/admin/news/:id/publish`
  - 发布文章
  - 要求 `news:publish`
- `POST /api/admin/news/:id/unpublish`
  - 撤回为草稿
  - 要求 `news:publish`

## Permission Matrix

角色扩展为三类：

### `admin`

- roles: `["admin"]`
- permissions:
  - `project:create`
  - `project:read`
  - `audit:read`
  - `news:create`
  - `news:edit`
  - `news:publish`
  - `news:delete`

### `editor`

- roles: `["editor"]`
- permissions:
  - `news:create`
  - `news:edit`
  - `news:publish`

### `member`

- roles: `["member"]`
- permissions:
  - `project:read`

这次明确约束：

- `editor` 允许发布文章
- `member` 不能访问新闻后台写接口
- `admin` 继续保留项目创建和审计能力

## Database Design

现有表继续保留：

- `app_users`
- `auth_sessions`
- `auth_refresh_tokens`

本次新增两张业务表：

### `news_categories`

- `category_id`
- `slug`
- `name`
- `sort_order`

### `news_articles`

- `article_id`
- `slug`
- `title`
- `summary`
- `body`
- `category_id`
- `status`
- `author_user_id`
- `published_at`
- `created_at`
- `updated_at`

约束：

- `status` 只允许 `draft`、`published`
- `slug` 唯一
- `published_at` 在 `draft` 可为空

## Seed Data Design

样例数据建议至少包含：

### Users

- `admin`
- `editor`
- `member`

### Categories

- `company`
- `product`
- `security`

### Articles

- 2 篇 `published`
- 1 篇 `draft`

这样可以同时验证：

- 前台只能看到已发布内容
- 后台能看到全部内容
- 发布动作会改变前台可见性

## Frontend Design

### Public News Page

`/news` 页面展示：

- 顶部站点标题
- 分类信息
- 新闻卡片列表
- 每篇卡片跳到 `/news/:slug`

`/news/:slug` 页面展示：

- 标题
- 分类
- 发布时间
- 摘要
- 正文

风格应更像真实新闻站，而不是管理后台。

### News Admin Page

`/news-admin` 页面展示：

- 当前登录用户摘要
- 文章列表
- 状态筛选
- 编辑表单
- 发布/撤回按钮
- 最近一次 API 响应结果

后台按钮默认使用 cookie session 测试 CSRF；必要时保留 bearer 按钮，验证 JWT 写请求不受 cookie 干扰。

## Auth And Authorization Rules

现有 `package.toml` 需要补新闻规则：

- `GET /news-admin`
  - `require = "authenticated"`
  - `permissions_any = ["news:create", "news:edit", "news:publish"]`
- `GET /api/admin/news`
  - `permissions_all = ["news:edit"]`
- `POST /api/admin/news`
  - `permissions_all = ["news:create"]`
- `PUT /api/admin/news/*`
  - `permissions_all = ["news:edit"]`
- `POST /api/admin/news/*/publish`
  - `permissions_all = ["news:publish"]`
- `POST /api/admin/news/*/unpublish`
  - `permissions_all = ["news:publish"]`

前台新闻页和公开新闻 API 保持 `public`。

## Verification Standard

本次完成标准不是“页面能打开”，而是这些行为全部成立：

### Public Access

- 未登录可访问 `/news`
- 未登录可访问 `/news/:slug`
- 未登录只能看到 `published` 文章
- 未登录不能访问 `/news-admin`

### Role And Permission Checks

- `member` 登录后访问 `/news-admin` 应失败
- `member` 调 `POST /api/admin/news` 应 `403`
- `editor` 可进入 `/news-admin`
- `editor` 可创建并发布文章
- `admin` 同样可创建并发布文章

### Session / JWT / CSRF

- cookie 登录后创建新闻，不带 `X-CSRF-Token` 应 `403`
- cookie 登录后带正确 token 创建新闻，应成功
- bearer 调后台新闻写接口，应不要求 CSRF
- bearer 写请求不应因为浏览器残留 cookie 而误走 session 鉴权链

### Publish Visibility

- `draft` 文章在前台 `/news` 不可见
- 执行 publish 后，前台 `/news` 可见
- 执行 unpublish 后，前台再次不可见

### Database Persistence

- 新建新闻后，`news_articles` 有记录
- 发布后，`status = 'published'`
- `published_at` 被写入

## Non-Goals

本次明确不做：

- 新闻评论
- 后台审核流
- 封面图上传
- Markdown 或富文本编辑器
- 分页、搜索、推荐位
- 多租户新闻隔离

## Why This Is The Right Next Step

当前 `auth-b2b-portal` 已经验证了：

- 登录
- session
- JWT
- refresh
- admin/project 权限

但它还缺一个“真正能让权限矩阵展开”的内容业务。  
新闻模块正好满足这个缺口：

- 有 public 面
- 有 admin 面
- 有草稿与发布状态
- 有 editor 与 member 的明显权限差异

因此它能把 auth sample 从“认证能力演示”提升到“真实业务权限样板”。
