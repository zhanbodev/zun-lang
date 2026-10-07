# 12A. 配置文件与 Serve 模式

这一章只回答一个问题：

**`zun serve` 当前会读取 `package.toml` 里的哪些配置块，以及每个块接受哪些参数。**

如果你只想快速知道“哪些字段能写、默认值是什么、哪些是 serve 模式才会看”，这一章就是入口。

完整 schema 和底层实现细节请补充阅读：

- [../reference/project-system.md](../reference/project-system.md)
- [../reference/http.md](../reference/http.md)

## 最小 serve 项目

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080
```

当前 `serve` 模式首先会读取项目根目录下的 `package.toml`。

就**文件配置块**而言，当前实现接受的完整树就是这一套：

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"

[server]

[server.auth]
[server.auth.session]
[server.auth.session.store]
[server.auth.session.store.sqlite]
[server.auth.session.store.postgres]
[server.auth.jwt]
[server.auth.authorization]
[[server.auth.authorization.rules]]

[env]
[dependencies]
```

也就是说，**当前 `package.toml` 能被解析和消费的块，到这里就列完了**。

serve 模式真正会用到的主要是这些字段：

- `[project]`
- `[server]`
- `[server.auth]` 及其子块
- `[env]`

`[dependencies]` 会被项目系统接受，但它不属于 serve 专属配置。

## 先说一个边界

下面这些能力虽然会影响 serve 行为，但**不是 `package.toml` 文件块**：

- `@CORS(...)`
- `@SET_HDR(...)`
- `$HTTP(...).link(...)`
- `$main()` 里的启动逻辑

它们属于 `.zun` 源码中的服务声明，不属于本章的“配置文件章节”范围。

## `[project]`

基础项目元信息现在推荐统一放在 `[project]` 下：

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"
```

兼容性说明：

- 当前实现仍兼容旧写法：顶层 `name` / `version` / `entry`
- 新项目建议统一使用 `[project]`

### `name`

- 类型：`String`
- 说明：项目名
- 当前状态：实际必填；缺失或空字符串时 manifest 不会被视为有效项目配置

### `version`

- 类型：`String`
- 说明：项目版本号
- 默认值：空字符串

### `entry`

- 类型：`String`
- 说明：项目入口文件
- 默认值：`"main.zun"`

## 完整配置树速览

如果你需要一眼看完层级关系，可以先看这张树：

```text
package.toml
├── [project]
│   ├── name
│   ├── version
│   └── entry
├── [server]
│   ├── host
│   ├── port
│   ├── upload_max_size
│   ├── upload_max_file_size
│   └── [auth]
│       ├── enabled
│       ├── default_scheme
│       ├── identity_sources
│       ├── [session]
│       │   ├── enabled
│       │   ├── cookie_name
│       │   ├── cookie_secure
│       │   ├── cookie_http_only
│       │   ├── cookie_same_site
│       │   ├── cookie_path
│       │   ├── ttl_seconds
│       │   ├── idle_timeout_seconds
│       │   ├── rotation
│       │   └── [store]
│       │       ├── driver
│       │       ├── [sqlite]
│       │       │   ├── path
│       │       │   └── table
│       │       └── [postgres]
│       │           ├── url
│       │           ├── url_env
│       │           └── table
│       ├── [jwt]
│       │   ├── enabled
│       │   ├── issuer
│       │   ├── audience
│       │   ├── algorithm
│       │   ├── secret
│       │   ├── secret_env
│       │   ├── access_ttl_seconds
│       │   └── refresh_ttl_seconds
│       └── [authorization]
│           ├── enabled
│           ├── default
│           └── [[rules]]
│               ├── method
│               ├── path
│               ├── require
│               ├── roles_any
│               ├── roles_all
│               ├── permissions_any
│               ├── permissions_all
│               └── claims_all
├── [env]
└── [dependencies]
```

## `[server]`

`serve` 模式直接消费这一块：

```toml
[server]
host = "127.0.0.1"
port = 8080
upload_max_size = 50
upload_max_file_size = 10
```

参数：

- `host`
  - 类型：`String`
  - 默认值：`"127.0.0.1"`
  - 说明：HTTP 服务监听地址
  - 当前只接受：
    - `127.0.0.1`
    - `0.0.0.0`
- `port`
  - 类型：`Int`
  - 默认值：`8080`
  - 说明：HTTP 服务监听端口
- `upload_max_size`
  - 类型：`Int`（MB）
  - 默认值：`50`
  - 说明：单次 `multipart/form-data` 请求允许的**总大小**上限；超出时返回 400
- `upload_max_file_size`
  - 类型：`Int`（MB）
  - 默认值：`10`
  - 说明：单个上传文件的大小上限；超出时返回 400

`host` 语义补充：

- `127.0.0.1` 表示仅本机访问，也是默认值
- `0.0.0.0` 表示绑定所有接口，允许外部访问
- `localhost`、具体局域网 IP、其他字符串当前都不接受，会在启动前报配置错误

启动 `zun serve .` 后，CLI 会输出带颜色的地址提示：

- `127.0.0.1` 时打印 `Local: http://127.0.0.1:<port>`
- `0.0.0.0` 时同时打印：
  - `Local: http://127.0.0.1:<port>`
  - 如果能枚举到本机私网 IPv4，会逐条打印 `Network: http://<LAN_IP>:<port>`
  - 如果当前环境没有可用私网 IPv4，再回退成 `Network: listening on all interfaces (:<port>), use your LAN IP to access`

## `[server.auth]`

认证和授权的总开关：

```toml
[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie", "bearer"]
```

参数：

- `enabled`
  - 类型：`Bool`
  - 默认值：`false`
  - 说明：是否启用认证/授权 runtime
- `default_scheme`
  - 类型：`String`
  - 默认值：`"session"`
  - 当前接受值：`"session"`、`"bearer"`
  - 注意：当前主要用于配置校验；不要把它理解成已经完整改变请求解析行为的“强默认切换器”
- `identity_sources`
  - 类型：`List<String>`
  - 默认值：`["cookie"]`
  - 当前接受值：运行时主路径支持 `cookie`、`bearer`
  - 说明：按顺序解析认证来源；例如 `["cookie", "bearer"]` 表示先尝试 session cookie，再回退 bearer token

## `[server.auth.session]`

session cookie 与 session store 的主配置：

```toml
[server.auth.session]
enabled = true
cookie_name = "zun_session"
cookie_secure = false
cookie_http_only = true
cookie_same_site = "lax"
cookie_path = "/"
ttl_seconds = 86400
idle_timeout_seconds = 7200
rotation = "on_login"
```

参数：

- `enabled`
  - 类型：`Bool`
  - 默认值：`false`
- `cookie_name`
  - 类型：`String`
  - 默认值：`"zun_session"`
- `cookie_secure`
  - 类型：`Bool`
  - 默认值：`false`
- `cookie_http_only`
  - 类型：`Bool`
  - 默认值：`true`
- `cookie_same_site`
  - 类型：`String`
  - 默认值：`"lax"`
  - 当前接受值：`"lax"`、`"strict"`、`"none"`
  - 约束：如果值为 `"none"`，则必须同时设置 `cookie_secure = true`
- `cookie_path`
  - 类型：`String`
  - 默认值：`"/"`
- `ttl_seconds`
  - 类型：`Int`
  - 默认值：`86400`
  - 说明：session 绝对过期时间
- `idle_timeout_seconds`
  - 类型：`Int`
  - 默认值：`7200`
  - 说明：空闲超时；大于 0 时请求会刷新 idle deadline
- `rotation`
  - 类型：`String`
  - 默认值：`"on_login"`
  - 当前接受值：`"off"`、`"on_login"`、`"always"`

`rotation` 语义：

- `off`
  - 不自动轮换 session id
- `on_login`
  - `session.create(...)` 建立新登录态时失效当前 session
- `always`
  - 登录时失效当前 session，且每次已认证请求后轮换 session id

## `[server.auth.session.store]`

配置 session store driver：

```toml
[server.auth.session.store]
driver = "memory"
```

参数：

- `driver`
  - 类型：`String`
  - 默认值：`"memory"`
  - 当前实现支持：`"memory"`、`"sqlite"`、`"postgres"`

## `[server.auth.session.store.sqlite]`

当 `driver = "sqlite"` 时可用：

```toml
[server.auth.session.store.sqlite]
path = ".zun/auth.sqlite3"
table = "auth_sessions"
refresh_table = "auth_refresh_tokens"
```

参数：

- `path`
  - 类型：`String`
  - 默认值：`".zun/auth.sqlite3"`
- `table`
  - 类型：`String`
  - 默认值：`"auth_sessions"`
- `refresh_table`
  - 类型：`String`
  - 默认值：`"auth_refresh_tokens"`

## `[server.auth.session.store.postgres]`

当 `driver = "postgres"` 时可用：

```toml
[server.auth.session.store.postgres]
url = "postgresql://..."
url_env = "SESSION_DATABASE_URL"
table = "auth_sessions"
refresh_table = "auth_refresh_tokens"
```

参数：

- `url`
  - 类型：`String`
  - 默认值：空字符串
  - 说明：直接写 PostgreSQL 连接串
- `url_env`
  - 类型：`String`
  - 默认值：空字符串
  - 说明：从环境变量读取 PostgreSQL 连接串
- `table`
  - 类型：`String`
  - 默认值：`"auth_sessions"`
- `refresh_table`
  - 类型：`String`
  - 默认值：`"auth_refresh_tokens"`

说明：

- `url` 和 `url_env` 二选一即可
- 当前实现里，session store 和 refresh token store 会共用同一套 backend 选择

## `[server.auth.jwt]`

JWT access token / refresh token 配置：

```toml
[server.auth.jwt]
enabled = true
issuer = "zun"
audience = "zun"
algorithm = "HS256"
secret = "replace-me"
secret_env = "JWT_SECRET"
access_ttl_seconds = 3600
refresh_ttl_seconds = 2592000
```

参数：

- `enabled`
  - 类型：`Bool`
  - 默认值：`false`
- `issuer`
  - 类型：`String`
  - 默认值：空字符串
- `audience`
  - 类型：`String`
  - 默认值：空字符串
- `algorithm`
  - 类型：`String`
  - 默认值：`"HS256"`
  - 当前接受值：`"HS256"`、`"HS384"`、`"HS512"`
- `secret`
  - 类型：`String`
  - 默认值：空字符串
- `secret_env`
  - 类型：`String`
  - 默认值：空字符串
- `access_ttl_seconds`
  - 类型：`Int`
  - 默认值：`3600`
- `refresh_ttl_seconds`
  - 类型：`Int`
  - 默认值：`2592000`

说明：

- `enabled = true` 时必须能解析出 JWT secret
- `secret` 与 `secret_env` 的优先级是：先 `secret`，再 `secret_env`

## `[server.auth.authorization]`

授权规则总开关：

```toml
[server.auth.authorization]
enabled = true
default = "public"
```

参数：

- `enabled`
  - 类型：`Bool`
  - 默认值：`false`
- `default`
  - 类型：`String`
  - 默认值：`"public"`
  - 当前主路径语义：
    - `"public"`：未命中规则时默认公开
    - `"authenticated"`：未命中规则时默认要求登录

注意：

- 这是“默认策略块”
- 真正按路由细化授权时，要配合 `[[server.auth.authorization.rules]]`

## `[[server.auth.authorization.rules]]`

按路由声明授权规则：

```toml
[[server.auth.authorization.rules]]
method = "GET"
path = "/admin/*"
require = "authenticated"
roles_any = ["admin"]
roles_all = []
permissions_any = []
permissions_all = []
claims_all = { team_id = "t_001" }
```

参数：

- `method`
  - 类型：`String`
  - 默认值：空字符串
  - 当前支持：
    - 精确 method，如 `GET`、`POST`
    - `"*"` 表示任意 method
- `path`
  - 类型：`String`
  - 默认值：空字符串
  - 当前支持：
    - 精确路径，如 `"/admin"`
    - 前缀通配，如 `"/admin/*"`
- `require`
  - 类型：`String`
  - 当前主路径使用值：`"authenticated"`
- `roles_any`
  - 类型：`List<String>`
  - 默认值：空列表
- `roles_all`
  - 类型：`List<String>`
  - 默认值：空列表
- `permissions_any`
  - 类型：`List<String>`
  - 默认值：空列表
- `permissions_all`
  - 类型：`List<String>`
  - 默认值：空列表
- `claims_all`
  - 类型：`Map<String, String>`
  - 默认值：空对象

说明：

- 多条规则同时命中时，优先使用更具体的路径规则
- 同等具体度时，优先使用声明顺序更早的规则

## `[env]` 与 `[dependencies]` 为什么也写进来

虽然这两块不是 auth/HTTP 配置，但它们同样属于 `package.toml` 当前可被接受的正式文件块，所以这里一并列出，保证这章对“配置文件章节”来说是完整的。

## `[env]`

这块不是 auth 专属，但 serve 项目会一起加载：

```toml
[env]
APP_NAME = "Demo"
```

参数形式：

- 任意 `key = "value"` 字符串键值对

用途：

- 可通过 `$<<CONFIG("APP_NAME")` 读取

## `[dependencies]`

这块也会被 manifest 接受：

```toml
[dependencies]
acme = "0.1.0"
```

参数形式：

- 任意 `module_name = "version-or-spec"` 字符串键值对

说明：

- 它属于项目/模块解析能力的一部分
- 不属于 serve runtime 的 auth / HTTP 配置

## 推荐模板

### 1. 只有 HTTP 服务，没有 auth

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080
```

### 2. 带文件上传限制

```toml
[project]
name = "upload-demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "0.0.0.0"
port = 8083
upload_max_size = 100        # 单次请求总大小上限 100 MB
upload_max_file_size = 20    # 单个文件大小上限 20 MB
```

### 3. session auth

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080

[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie"]

[server.auth.session]
enabled = true
cookie_name = "zun_session"

[server.auth.session.store]
driver = "memory"
```

### 4. session + bearer + route auth

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080

[server.auth]
enabled = true
default_scheme = "session"
identity_sources = ["cookie", "bearer"]

[server.auth.session]
enabled = true

[server.auth.session.store]
driver = "sqlite"

[server.auth.session.store.sqlite]
path = ".zun/auth.sqlite3"
table = "auth_sessions"

[server.auth.jwt]
enabled = true
issuer = "zun"
audience = "zun"
secret_env = "JWT_SECRET"

[server.auth.authorization]
enabled = true
default = "public"

[[server.auth.authorization.rules]]
method = "GET"
path = "/admin/*"
require = "authenticated"
roles_any = ["admin"]
```

## 下一步读什么

- 想看项目入口和 `entry` 解析：回看 [12-projects-and-package.md](12-projects-and-package.md)
- 想看 HTTP handler 和请求语义：继续读 [15-http-basics.md](15-http-basics.md)
- 想看完整 auth 配置与运行时语义：读 [../reference/http.md](../reference/http.md)

## 当前结论

如果你的问题是：

**“`package.toml` 在 serve 模式下，现在到底允许我写哪些块？”**

答案就是本章列出的这一整套，没有更多隐藏文件块。
