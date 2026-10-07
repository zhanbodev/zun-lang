# 18. 模式与实战配方

这一章给的是可复制的起步配方，而不是零散片段。

## 配方 1：单文件脚本

适合：

- 快速数据处理
- 小工具
- 一次性脚本

文件：`sum.zun`

```zun
$ nums = [1, 2, 3, 4, 5];
$ total = 0;

$for n in nums {
    total += n;
}

$>> total;
```

运行：

```bash
zun run sum.zun
```

预期输出：

```text
15
```

回看章节：

- [07-functions.md](07-functions.md)
- [08-collections-and-methods.md](08-collections-and-methods.md)

## 配方 2：读取并处理 JSON 文件

目录：

```text
json-tool/
├── main.zun
└── data.json
```

`data.json`

```json
{"name":"zun","tags":["lang","tool","lang"]}
```

`main.zun`

```zun
$mod std.fs;
$mod std.json;

$ raw = fs.read_text("data.json");
$ obj = json.parse(raw);
$ tags = obj["tags"];

$>> obj["name"];
$>> tags.unique().len();
```

运行：

```bash
zun run main.zun
```

预期输出：

```text
zun
2
```

回看章节：

- [08-collections-and-methods.md](08-collections-and-methods.md)
- [13-stdlib-overview.md](13-stdlib-overview.md)
- [14-io-env-config.md](14-io-env-config.md)

## 配方 3：带 `package.toml` 的小项目

目录：

```text
my-app/
├── package.toml
├── main.zun
└── shared/
    └── helper.zun
```

`package.toml`

```toml
[project]
name = "my-app"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080
```

`shared/helper.zun`

```zun
$fn greeting() -> String {
    $# "hello from helper";
}
```

`main.zun`

```zun
$mod shared.helper;
$>> helper.greeting();
```

运行：

```bash
zun run main.zun
```

预期输出：

```text
hello from helper
```

回看章节：

- [11-modules.md](11-modules.md)
- [12-projects-and-package.md](12-projects-and-package.md)

## 配方 4：模块化 HTTP 服务

目录：

```text
http-app/
├── package.toml
├── main.zun
└── routers/
    └── api.zun
```

`package.toml`

```toml
[project]
name = "http-app"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080
```

`routers/api.zun`

```zun
$GET("/status") status() -> JSON {
    $# $JSON { "ok": true };
}
```

`main.zun`

```zun
@CORS("*")
$main() {
    $>> "[INFO] server starting";
}

$HTTP("/api").link("routers.api");
```

运行：

```bash
zun serve .
```

验证：

```bash
curl http://127.0.0.1:8080/api/status
```

预期结果：

```text
{"ok":true}
```

回看章节：

- [12-projects-and-package.md](12-projects-and-package.md)
- [15-http-basics.md](15-http-basics.md)
- [16-http-organization.md](16-http-organization.md)

## 配方 5：带登录、刷新和 CSRF 的 HTTP 认证服务

如果你要一个可直接运行的完整 auth 起步项目，不要从零拼配置，直接用仓库里的示例：

- [examples/http-auth](/Users/liangzhanbo/CodeStudio/zun/examples/http-auth)

它已经覆盖：

- `POST /login`
- `POST /refresh`
- `POST /logout`
- `GET /me`
- `GET /admin`
- session cookie 与 bearer token 双入口
- cookie 写请求的 `X-CSRF-Token` 校验

启动：

```bash
zun serve examples/http-auth
```

如果你是从 guide 往下走，建议搭配阅读：

- [15-http-basics.md](15-http-basics.md)
- [../reference/http.md](../reference/http.md)
- [../reference/stdlib-api.md](../reference/stdlib-api.md)

## 迁移说明

旧的零散示例页仍在 [examples.md](examples.md)，但主线示例职责已经迁到本章。
