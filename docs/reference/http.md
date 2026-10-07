# HTTP 超函数参考

Zun 提供了完整的 HTTP 超函数支持，用于快速构建 Web API 服务。

## 目录

- [HTTP 方法](#http-方法)
- [返回类型](#返回类型)
- [HTTP 块](#http-块)
- [请求上下文](#请求上下文)
- [Serve 认证与授权](#serve-认证与授权)
- [响应返回](#响应返回)
- [模块挂载](#模块挂载)
- [测试模式](#测试模式)
- [HTTP 错误处理](#http-错误处理)

---

## HTTP 方法

### 基本语法

```zun
$METHOD("/path") handler_name(param1, param2) -> ReturnType {
    // handler body
    $# return_value;
}
```

### 支持的方法

| 方法 | 说明 | 适用场景 |
|------|------|----------|
| `$GET` | GET 请求 | 查询数据 |
| `$POST` | POST 请求 | 创建数据 |
| `$PUT` | PUT 请求 | 更新数据 |
| `$DELETE` 或 `$DEL` | DELETE 请求 | 删除数据 |
| `$PATCH` | PATCH 请求 | 部分更新 |

### 示例

```zun
// GET 请求 - 查询用户
$GET("/users/:id") get_user(id) -> JSON {
    $ result = $JSON {
        "id": id,
        "name": "John"
    };
    $# result;
}

// POST 请求 - 创建用户
$POST("/users") create_user() -> JSON {
    $ result = $JSON {
        "status": "created",
        "data": body
    };
    $# result;
}

// 带路径参数
$GET("/users/:id/posts/:post_id") get_user_post(id, post_id) -> JSON {
    $# $JSON { "user_id": id, "post_id": post_id };
}

// Query 参数自动提取
$GET("/search") search_handler(q, limit) -> JSON {
    $# $JSON { "query": q, "limit": limit };
}
```

---

## 返回类型

HTTP 路由支持以下返回类型：

| 返回类型 | 说明 | Content-Type |
|----------|------|--------------|
| `JSON` | JSON 响应（默认） | `application/json` |
| `HTML` | HTML 响应 | `text/html; charset=utf-8` |
| `String` | 纯文本响应 | `text/plain` |

### JSON 类型（默认）

```zun
$GET("/api/users") get_users() -> JSON {
    $# $JSON { "users": ["tom", "jerry"] };
}
```

### HTML 类型

可以使用 `$HTML()` 构造器或 `-> HTML` 返回类型：

```zun
// 使用 $HTML() 构造器（推荐）
$GET("/pages/home") home_page() -> HTML {
    $# $HTML("<h1>Welcome</h1><p>This is an HTML page</p>");
}

// 使用 -> HTML 返回类型
$GET("/pages/index") index_page() -> HTML {
    $# "<html><head><title>My App</title></head><body><h1>Hello</h1></body></html>";
}
```

#### $HTML().link() - 链接外部文件

可以使用 `$HTML().link()` 方法链接外部 HTML/CSS/JS/XML 文件：

```zun
// 链接外部 HTML 文件（使用模块路径格式）
$GET("/") index() -> HTML {
    $# $HTML().link("pages.index");
}

// 链接外部文件，内联内容优先
$GET("/page") page() -> HTML {
    $# $HTML("<h1>Override</h1>").link("pages.index");
}

// 支持的文件类型：html, htm, css, js, xml
$GET("/style") style() -> HTML {
    $# $HTML().link("assets.style");
}
```

**通配符支持**：

```zun
// 匹配目录下所有文件（包含子目录）
$GET("/all") all() -> HTML {
    $# $HTML().link("pages.*");
}

// 链式调用多个 link
$GET("/multi") multi() -> HTML {
    $# $HTML().link("pages.*").link("css.*");
}
```

**注意**：
- 模块路径格式：`"pages.index"` 对应 `pages/index.html`
- 不支持路径格式：`"pages/index"` 会报错
- 通配符 `"pages.*"` 会递归匹配 pages 目录下所有 html/htm/css/js/xml 文件
- 如果有内联内容，优先使用内联内容，忽略 link
- 多个 link 会按顺序合并内容

### String 类型

```zun
$GET("/text") text_handler() -> String {
    $# "Hello World";
}
```

---

## HTTP 块

### 基本块语法

可以使用 `$HTTP { }` 块在文件中定义多个路由：

```zun
$HTTP {
    $GET("/users") list_users() -> JSON { ... }
    $GET("/users/:id") get_user(id) -> JSON { ... }
    $POST("/users") create_user() -> JSON { ... }
}
```

### 带路径前缀的块

可以使用 `$HTTP(path) { }` 为块内的路由添加路径前缀：

```zun
$HTTP("/v1/api/") {
    $GET("/users") list_users() -> JSON { ... }
    $GET("/users/:id") get_user(id) -> JSON { ... }
}
// 实际路由: /v1/api/users, /v1/api/users/:id
```

路径前缀会自动处理斜杠去重：

```zun
$HTTP("/v1/api/") { ... }   // 前缀末尾有斜杠
$GET("/users") { ... }       // 路由开头有斜杠
// 结果: /v1/api/users (正确，去重)
```

---

## 请求上下文

### $HDR - 获取请求头

```zun
$GET("/api/data") api_handler() -> JSON {
    $ token = $HDR("Authorization");
    $# $JSON { "token": token };
}
```

### 请求体注入

当前实现会把请求体注入为隐式变量 `body`。

主线可依赖的口径是：

- `POST`、`PUT`、`PATCH` 可读取 `body`
- `GET`、`DELETE` 不作为主线 `body` 注入能力来承诺
- 文档不再把 `body` 写成 handler 形参

```zun
$POST("/users") create_user() -> JSON {
    $ name = body["name"];
    $# $JSON { "created": true, "name": name };
}
```

### `std.auth.*` 请求级上下文

`serve` 模式下启用 `[server.auth]` 后，运行时会先解析当前请求的认证态，再执行 handler。

可在 handler 中使用：

- `std.auth.session.current()`
- `std.auth.session.exists()`
- `std.auth.session.id()`
- `std.auth.session.get(key)`
- `std.auth.jwt.current()`
- `std.auth.jwt.bearer()`
- `std.auth.guard.principal()`
- `std.auth.guard.authenticated()`
- `std.auth.csrf.token()`
- `std.auth.csrf.rotate()`

这些 API 读取的是当前请求上下文中的 principal / session / JWT claims，不需要手动重复解析请求头。

如果你需要完整的 session / jwt / guard / csrf 方法表，请直接查 [stdlib-api.md](stdlib-api.md)。

---

## Serve 认证与授权

### 配置入口

认证和授权通过 `package.toml` 中的 `[server.auth]` 配置启用：

```toml
[server]
host = "127.0.0.1"
port = 8080

[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie", "bearer"]

[server.auth.session]
enabled = true
cookie_name = "zun_session"
ttl_seconds = 86400
idle_timeout_seconds = 7200
rotation = "on_login"

[server.auth.session.store]
driver = "postgres"

[server.auth.session.store.postgres]
url_env = "SESSION_DATABASE_URL"
table = "auth_sessions"
refresh_table = "auth_refresh_tokens"

[server.auth.jwt]
enabled = true
issuer = "zun"
audience = "zun"
algorithm = "HS256"
secret_env = "JWT_SECRET"
access_ttl_seconds = 3600
refresh_ttl_seconds = 2592000

[server.auth.authorization]
enabled = true
default = "public"

[[server.auth.authorization.rules]]
method = "GET"
path = "/admin"
require = "authenticated"
roles_any = ["admin"]
```

`algorithm` 当前支持：

- `HS256`
- `HS384`
- `HS512`

当前实现支持的 session store driver：

- `memory`
- `sqlite`
- `postgres`

当前实现里，refresh token store 会和 session store 共享同一 backend 选择，并支持单独配置 refresh token 表名：

- sqlite: `[server.auth.session.store.sqlite].refresh_table`
- postgres: `[server.auth.session.store.postgres].refresh_table`

默认值都是 `auth_refresh_tokens`。

session cookie 当前还有两个启动期安全约束：

- `cookie_same_site` 只接受 `lax` / `strict` / `none`
- 如果 `cookie_same_site = "none"`，必须同时配置 `cookie_secure = true`

`[server.auth.session].rotation` 当前支持：

- `off`
- `on_login`
- `always`

其中：

- `on_login` 会在 `session.create(...)` 建立新登录态时失效当前 session
- `always` 除了登录时失效旧 session，还会在每次已认证请求后轮换 session id

### CSRF 保护

当前 runtime 会对“基于 cookie session 的已认证写请求”自动启用 CSRF 校验：

- 适用方法：`POST` / `PUT` / `PATCH` / `DELETE`
- 仅对通过 session cookie 认证的请求生效
- bearer token 请求不会额外要求 CSRF header

校验方式：

- session 创建时会自动生成 `csrf_token`
- 请求需要携带 `X-CSRF-Token: <token>`
- token 可通过 `session.current()["csrf_token"]` 或 `std.auth.csrf.token()` 读取

缺失或不匹配时，响应为：

```json
{"status":403,"code":"auth_forbidden","message":"invalid csrf token"}
```

### 当前请求入口

请求进入 handler 前，runtime 会按 `identity_sources` 顺序解析认证态：

- `cookie`
  - 从请求 `Cookie` 中读取配置的 `cookie_name`
  - 查 session store，构造当前 principal
- `bearer`
  - 从 `Authorization: Bearer <token>` 读取 JWT
  - 校验签名、`issuer`、`audience`，构造当前 principal

如果没有命中任何身份来源，则当前请求视为匿名请求。

### 当前路由授权规则

当前已接通并有集成测试覆盖的规则：

- `require = "authenticated"`
- `roles_any = ["admin", ...]`
- `roles_all = ["editor", "publisher", ...]`
- `permissions_any = ["post:create", ...]`
- `permissions_all = ["post:create", "post:publish", ...]`
- `claims_all = { team_id = "t_001", ... }`

规则匹配当前支持：

- `method = "*"` 匹配任意 HTTP method
- `path = "/admin/*"` 这种前缀通配
- 多条规则同时命中时，优先使用更具体的路径规则；同等具体度下按声明顺序选择更早的规则

返回语义：

- 未登录或凭证无效：HTTP `401`
- 已登录但不满足角色要求：HTTP `403`

认证与授权失败响应体当前统一为：

```json
{
  "status": 401,
  "code": "auth_unauthorized",
  "message": "authentication required"
}
```

```json
{
  "status": 403,
  "code": "auth_forbidden",
  "message": "forbidden"
}
```

### Session 登录与受保护路由示例

```zun
$mod std.auth.session;

$GET("/login") login() -> JSON {
    session.create({
        "subject": "user_1",
        "roles": ["admin"],
        "permissions": []
    });
    $# {"ok": true};
}

$GET("/admin") admin() -> JSON {
    $# {"ok": true};
}
```

```toml
[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie"]

[server.auth.session]
enabled = true
cookie_name = "zun_session"

[server.auth.session.store]
driver = "memory"

[server.auth.authorization]
enabled = true

[[server.auth.authorization.rules]]
method = "GET"
path = "/admin"
require = "authenticated"
```

调用 `/login` 后，运行时会写入 `Set-Cookie`；后续请求带上该 cookie 才能访问受保护路由。

如果你想看完整可运行样例，可直接运行 [examples/http-auth](/Users/liangzhanbo/CodeStudio/zun/examples/http-auth)。

### Bearer Token 示例

```zun
$mod std.auth.jwt;

$GET("/token") token() -> JSON {
    $ token = jwt.sign({
        "sub": "user_1",
        "roles": ["admin"],
        "permissions": ["post:create"]
    });
    $# {"token": token};
}
```

配合：

```http
Authorization: Bearer <token>
```

即可访问启用了 bearer 认证入口的受保护路由。

### Refresh Token

refresh token 当前不自动进入 bearer 请求认证入口，而是通过标准库显式签发与校验：

```zun
$mod std.auth.jwt;

$ refresh = jwt.sign_refresh({
    "sub": "user_1",
    "roles": ["admin"],
    "permissions": ["post:create"]
});

$ claims = jwt.verify_refresh(refresh);
```

`jwt.refresh_pair(refresh_token)` 现在会执行 refresh token 轮换消费：

- 第一次使用旧 refresh token 刷新成功
- 第二次复用同一个旧 refresh token 会返回 HTTP `401`

当前 runtime 还会把 refresh token 元数据持久化到 auth store backend；使用 `sqlite` / `postgres` driver 时，refresh token 的有效性与撤销状态会跨服务重启保留。

项目层也可以显式撤销 refresh token：

```zun
$mod std.auth.jwt;

jwt.revoke_refresh(refresh_token);
```

---

## 响应返回

### $# - 返回响应

```zun
// 返回 JSON
$GET("/api/users") get_users() -> JSON {
    $# $JSON { "users": ["tom", "jerry"] };
}

// 返回字符串
$GET("/hello") hello() -> String {
    $# "Hello World";
}

// 返回空响应
$DELETE("/users/:id") delete_user(id) -> JSON {
    $# $JSON { "deleted": true };
}
```

### $RES - 指定 HTTP 状态码

默认情况下 handler 返回 HTTP 200。需要返回其他状态码时，用 `$RES(status, body)`：

**语法**：`$RES(状态码整数, JSON体)`

```zun
// 404 Not Found
$GET("/users/:id") get_user(id) -> JSON {
    $if !user_exists(id) {
        $# $RES(404, {"msg": "用户不存在"});
    }
    $# $RES(200, {"id": id});
}

// 201 Created
$POST("/users") create_user() -> JSON {
    $# $RES(201, {"msg": "创建成功", "data": body});
}
```

两种响应方式的区别：

| 写法 | HTTP 状态码 | 适用场景 |
|------|------------|---------|
| `$# {...}` | 始终 200 | 业务自定义 code，前端按 JSON 判断 |
| `$# $RES(404, {...})` | 指定状态码 | 需要 HTTP 语义（REST API、网关鉴权） |

### $>> - 输出日志

```zun
$GET("/api/test") test_handler() -> JSON {
    $>> "Processing request...";
    $# $JSON { "ok": true };
}
```

---

## 模块挂载

### $HTTP(path).link(module) - 挂载外部模块

可以将其他 `.zun` 文件中的路由挂载到指定路径前缀下：

**main.zun:**
```zun
$main() {
    $HTTP("/v1/api/").link("routers.api");
}
```

**routers/api.zun:**
```zun
$GET("/users") list_users() -> JSON {
    $# $JSON { "users": ["tom", "jerry"] };
}

$GET("/users/:id") get_user(id) -> JSON {
    $# $JSON { "id": id, "name": "John" };
}
```

挂载后路由：
- `GET /v1/api/users` → list_users
- `GET /v1/api/users/:id` → get_user

说明：

- 这里不需要额外写 `$mod routers.api;`
- `$HTTP(...).link("routers.api")` 只扫描并挂载目标模块中的 HTTP 路由
- 目标文件里的普通 `$fn` 不会被导入当前语言作用域

### 路径斜杠处理

系统会自动处理路径斜杠，去除重复的斜杠：

```zun
$HTTP("/v1/api/").link("routers.api")  // 前缀末尾有斜杠
$GET("/users")                          // 路由开头有斜杠
// 结果: /v1/api/users (正确)
```

---

## 模块声明

### $mod - 导入模块

```zun
$mod dao.user;
$mod services.auth;
$mod services.*;
```

这会加载对应的 `.zun` 文件：
- `dao.user` → `dao/user.zun`
- `services.auth` → `services/auth.zun`
- `services.*` → 导入 `services/` 目录下的直接子模块

---

## $main() 入口

`$main()` 函数是服务器入口点，只能在 `main.zun` 中定义：

```zun
$main() {
    // 初始化逻辑
    $>> "Server starting...";

    // 可以直接定义路由
    $GET("/hello") hello() -> String {
        $# "Hello";
    }

    // 也可以挂载模块
    $HTTP("/api/").link("routers.api");
}
```

---

## 测试模式

### zun test

```bash
# 测试所有路由
zun test main.zun

# 测试特定路由
zun test main.zun --route GET /hello

# 带 body 测试
zun test main.zun --route POST /api/users --body '{"name":"tom"}'
```

---

## 静态文件服务

### $STATIC - 静态文件服务

可以使用 `$STATIC()` 函数来服务静态文件（CSS、JS、图片等）：

```zun
$main() {
    // 方式1: 指定 URL 前缀和目录
    $STATIC("/css", "css");      // /css/style.css -> css/style.css
    $STATIC("/js", "js");        // /js/main.js -> js/main.js
    $STATIC("/images", "img");   // /images/logo.png -> img/logo.png

    // 方式2: 只指定目录（默认 URL 前缀为 /static）
    $STATIC("public");           // /static/index.html -> public/index.html

    // 方式3: 带点号格式的目录
    $STATIC("/zuncss", "css.zun"); // /zuncss/style.css -> css/zun.css
}
```

**语法：**
- `$STATIC("dir")` - 单参数，URL 前缀默认为 `/static`
- `$STATIC("/url-prefix", "dir")` - 双参数，指定 URL 前缀和目录
- `dir` 支持点号格式：`css.zun` 会转换为 `css/` 目录

**示例：**

```zun
// 项目结构
// my-app/
// ├── main.zun
// ├── css/
// │   └── style.css
// └── js/
//     └── main.js

$main() {
    $STATIC("/css", "css");
    $STATIC("/js", "js");

    $GET("/") index() -> HTML {
        $# $HTML().link("pages.index");
    }
}
```

访问：
- `GET /css/style.css` → 返回 `css/style.css`
- `GET /js/main.js` → 返回 `js/main.js`

---

## CLI 命令

| 命令 | 说明 |
|------|------|
| `zun` | 启动 REPL 模式 |
| `zun run <file>` | 运行 .zun 文件 |
| `zun serve [path]` | 启动 HTTP 服务器 |
| `zun serve --routertab` | 启动并显示路由表 |
| `zun test [file]` | 运行测试 |

---

## HTTP 错误处理

### $throw 在 handler 中的行为

`$throw` 用于代码级错误传播（通常在工具函数 / 模块中）。在 HTTP handler 里：

**未捕获 → HTTP 500 + 终端打印：**

```zun
$GET("/boom") boom() -> JSON {
    $throw "something went wrong";
    // 终端: [ERROR] uncaught throw: something went wrong
    // 客户端: HTTP 500 + {"error": "uncaught throw: something went wrong"}
}
```

**用 `$try/$catch` 捕获，主动控制响应：**

```zun
$fn check_token(token) {
    $if token == "" { $throw "token 为空"; }
}

$GET("/api/data") api_data() -> JSON {
    $try {
        check_token($HDR("Authorization"));
        $# $RES(200, {"data": "ok"});
    } $catch err {
        $# $RES(401, {"msg": err});
    }
}
```

### $>>ERR — 终端日志（不影响响应）

```zun
$GET("/debug") debug() -> JSON {
    $>>ERR("debug: handler called");   // 仅打印到开发终端
    $# {"ok": true};                   // HTTP 响应不受影响
}
```

### 行为速查

| 写法 | 终端输出 | HTTP 响应 |
|------|---------|-----------|
| `$# $RES(404, {...})` | 无 | HTTP 404 + body |
| `$# {...}` | 无 | HTTP 200 + body |
| `$throw "err"` 未捕获 | `[ERROR] uncaught throw: err` | HTTP 500 |
| `$throw "err"` + `$catch` | 无 | 取决于 catch 块的 `$#` |
| `$>>ERR("log")` | 打印到终端 | 无影响 |

---

## 完整示例

**项目结构：**
```
my-app/
├── main.zun
└── routers/
    ├── api.zun
    └── users.zun
```

**main.zun:**
```zun
$mod routers.api;
$mod routers.users;

$main() {
    // 挂载 API 路由到 /v1/api
    $HTTP("/v1/api/").link("routers.api");

    // 挂载用户路由到 /v1/users
    $HTTP("/v1/users/").link("routers.users");

    // 本地定义一个路由
    $GET("/health") health() -> JSON {
        $# $JSON { "status": "ok" };
    }
}
```

**routers/api.zun:**
```zun
$GET("/info") get_info() -> JSON {
    $# $JSON { "version": "1.0.0" };
}
```

**routers/users.zun:**
```zun
$GET("/list") list_users() -> JSON {
    $# $JSON { "users": ["tom", "jerry"] };
}

$POST("/create") create_user() -> JSON {
    $# $JSON { "created": true, "data": body };
}
```

**启动服务：**
```bash
zun serve .
# 或显示路由表
zun serve . --routertab
```

**注册路由：**
```
GET /health         → health
GET /v1/api/info    → get_info
GET /v1/users/list   → list_users
POST /v1/users/create → create_user
```
