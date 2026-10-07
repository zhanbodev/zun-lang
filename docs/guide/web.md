# Web 服务

> **Status: Migration Source**
>
> 这是一页迁移期旧 Web 文档。
> 当前主线请改读 [15-http-basics.md](15-http-basics.md) 和 [16-http-organization.md](16-http-organization.md)。
> 本页仍保留的独特价值是把 serve 启动、路由、CORS、静态资源放在同一页浏览，但不再作为主线入口。

Zun 内置 HTTP 服务支持，通过 `zun serve` 启动，使用声明式语法定义路由。

---

## 启动服务

```bash
# 运行单文件
zun serve main.zun

# 运行项目目录（自动读取 package.toml）
zun serve .

# 启动并打印路由表
zun serve . --routertab
```

默认监听 `127.0.0.1:8080`，可通过 `package.toml` 修改：

```toml
[server]
port = 3000
host = "127.0.0.1"
```

---

## 定义路由

### 基本语法

```zun
$METHOD("/path") handler_name() -> ReturnType {
    // 处理逻辑
    $# 返回值;
}
```

支持的 HTTP 方法：

| 关键字 | HTTP 方法 |
|--------|-----------|
| `$GET` | GET |
| `$POST` | POST |
| `$PUT` | PUT |
| `$DEL` | DELETE |
| `$PATCH` | PATCH |

### 示例

```zun
$GET("/hello") hello() -> String {
    $# "world";
}

$GET("/ping") ping() -> String {
    $# "pong";
}
```

---

## 返回类型

### 返回 JSON

```zun
$GET("/user") get_user() -> JSON {
    $# {"name": "Alice", "age": 30};
}
```

### 返回字符串

```zun
$GET("/health") health() -> String {
    $# "ok";
}
```

### 返回 HTML

```zun
$GET("/") index() -> HTML {
    $# "<h1>Welcome to Zun!</h1>";
}
```

---

## 路径参数

在路径中使用 `:param` 定义动态参数，参数值自动注入为同名变量：

```zun
$GET("/users/:id") get_user(id) -> JSON {
    $# {"id": id, "name": "User " + id};
}
```

访问 `/users/42` → `{"id": "42", "name": "User 42"}`

---

## 请求体（POST / PUT / PATCH）

`POST`、`PUT`、`PATCH` 请求的 JSON body 自动解析，通过 `body` 变量访问：

```zun
$POST("/echo") echo() -> JSON {
    $# body;
}

$POST("/users") create_user() -> JSON {
    $ name = body["name"];
    $ age = body["age"];
    $# {"created": true, "name": name, "age": age};
}
```

---

## 查询参数

URL 查询字符串（`?key=value`）中的参数自动注入为变量：

```zun
$GET("/search") search() -> JSON {
    // GET /search?q=hello&limit=10
    $# {"query": q, "limit": limit};
}
```

---

## 读取请求头

使用 `$HDR("Header-Name")` 读取请求头：

```zun
$GET("/whoami") whoami() -> JSON {
    $ token = $HDR("Authorization");
    $# {"token": token};
}
```

---

## 响应头注解

如果你要给响应统一附加静态 header，用 `@SET_HDR(...)`：

```zun
@SET_HDR({
    "X-Frame-Options": "DENY",
    "Cache-Control": "max-age=3600"
})
$GET("/health") health() -> String {
    $# "ok";
}
```

规则很简单：

- `@SET_HDR(...)` 只接受一个对象字面量
- key 必须是字符串 header 名
- value 必须是字符串字面量
- 一个注解里可以声明多个 header

如果没有写 `@SET_HDR(...)`，响应不会额外附加这些 header。

---

## CORS 注解

公开接口可以直接写：

```zun
@CORS("*")
$GET("/public") public_data() -> JSON {
    $# {"ok": true};
}
```

需要白名单时，用对象写法：

```zun
@CORS({
    origins: ["https://app.example.com"],
    methods: ["GET", "POST"],
    headers: ["Authorization"],
    max_age: 600,
    credentials: true
})
$GET("/profile") profile() -> JSON {
    $# {"ok": true};
}
```

`@CORS(...)` 可以放在：

- `$main()` 前，作为全局默认策略
- `$HTTP(...)` 前，作为块级策略
- 路由前，作为路由级策略

优先级是路由级覆盖块级，块级覆盖全局；没有写时，就不会额外返回 CORS 响应头。

---

## 路由分组（$HTTP 块）

使用 `$HTTP("/prefix") { ... }` 为一组路由添加统一前缀：

```zun
$HTTP("/api/v1") {
    $GET("/users") list_users() -> JSON {
        $# [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}];
    }

    $POST("/users") create_user() -> JSON {
        $# {"created": true, "user": body};
    }

    $GET("/users/:id") get_user(id) -> JSON {
        $# {"id": id};
    }
}
```

最终路由为 `/api/v1/users` 和 `/api/v1/users/:id`。

---

## 链接外部模块路由

将其他模块中定义的路由链接到当前服务，支持多级模块拆分：

```zun
// routers/api.zun
$GET("/status") status() -> String {
    $# "running";
}
```

```zun
// main.zun
$HTTP().link("routers.api");
```

多个模块链接：

```zun
$HTTP().link("routers.users");
$HTTP().link("routers.orders");
$HTTP().link("routers.admin");
```

---

## 静态文件

使用 `$STATIC` 提供静态文件服务：

```zun
// 将 /static 映射到项目的 static/ 目录
$STATIC("/static", "static");

// 简写（URL 前缀默认为 /static）
$STATIC("static");
```

项目结构：

```
my-app/
├── main.zun
└── static/
    ├── index.html
    ├── css/
    │   └── style.css
    └── js/
        └── app.js
```

---

## 读取项目配置

通过 `$<<CONFIG("KEY")` 读取 `package.toml` 的 `[env]` 配置：

```toml
# package.toml
[env]
DB_HOST = "localhost"
DB_PORT = "5432"
SECRET_KEY = "my-secret"
```

```zun
$GET("/config") show_config() -> JSON {
    $ db_host = $<<CONFIG("DB_HOST");
    $ db_port = $<<CONFIG("DB_PORT");
    $# {"db_host": db_host, "db_port": db_port};
}
```

---

## 状态码控制

默认情况下，handler 返回 HTTP 200。用 `$RES(status, body)` 显式指定状态码：

```zun
// 404 Not Found
$GET("/users/:id") get_user(id) -> JSON {
    $# $RES(404, {"msg": "用户不存在"});
}

// 201 Created
$POST("/users") create_user() -> JSON {
    $# $RES(201, {"msg": "创建成功", "data": body});
}
```

| 写法 | HTTP 状态码 | 适用场景 |
|------|------------|---------|
| `$# {...}` | 始终 200 | 普通 JSON 返回 |
| `$# $RES(201, {...})` | 指定状态码 | REST API 语义 |

---

## 错误处理

### $throw — 代码级错误传播

`$throw` 用于终止执行并传播错误，可配合 `$try/$catch` 捕获：

**未捕获 → HTTP 500 + 终端打印错误：**

```zun
$GET("/boom") boom() -> JSON {
    $throw "something went wrong";
    // 终端: [ERROR] uncaught throw: something went wrong
    // 客户端: HTTP 500 + {"error": "uncaught throw: something went wrong"}
}
```

**用 `$try/$catch` 捕获，主动返回响应：**

```zun
$GET("/safe") safe() -> JSON {
    $try {
        $throw "inner error";
    } $catch e {
        $# {"caught": e};
    }
}
```

### $>>ERR — 终端日志（不影响响应）

```zun
$GET("/debug") debug() -> JSON {
    $>>ERR("debug info");     // 仅输出到终端
    $# {"ok": true};          // HTTP 响应正常返回
}
```

---

## 完整示例

```zun
// main.zun
$mod std.json;

$GET("/") index() -> HTML {
    $# "<h1>Zun Web</h1>";
}

$HTTP("/api") {
    $GET("/hello") hello() -> JSON {
        $# {"message": "Hello, World!"};
    }

    $POST("/echo") echo() -> JSON {
        $# body;
    }

    $GET("/users/:id") get_user(id) -> JSON {
        $# {"id": id, "name": f"User {id}"};
    }
}

$STATIC("/static", "static");
```

启动：

```bash
zun serve main.zun --routertab
```

输出：

```
  GET    /              -> index
  GET    /api/hello     -> hello
  POST   /api/echo      -> echo
  GET    /api/users/:id -> get_user
  STATIC /static        -> static/

Local: http://127.0.0.1:8080
Network: http://192.168.1.25:8080

如果当前环境拿不到可用私网 IPv4，CLI 会回退成：

Network: listening on all interfaces (:8080), use your LAN IP to access
```
