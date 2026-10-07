# 12. 项目系统与 package.toml

当脚本开始变大，通常就该进入项目目录组织。

## 一个最小项目

```text
my-app/
├── package.toml
├── main.zun
└── shared/
    └── helper.zun
```

## 最小 `package.toml`

```toml
[project]
name = "my-app"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080

[env]
APP_NAME = "My App"
```

## 当前重要字段

- `[project]`
- `[server]`
- `[env]`
- `[dependencies]`

其中 `[dependencies]` 当前更适合理解为模块解析边界的一部分，而不是成熟包管理工作流。

## 入口文件和 `entry`

当 CLI 传入目录时：

1. 先定位项目根
2. 读取 `package.toml`
3. 使用 `[project].entry` 指定的入口文件
4. 如果没有 manifest，则默认尝试 `main.zun`

所以：

- `entry` 决定“从哪个文件启动项目”
- 它不等于 `$main()`

兼容性说明：

- 当前实现仍兼容旧写法：顶层 `name` / `version` / `entry`
- 新项目建议统一写到 `[project]`

## `$main()` 什么时候需要

`$main()` 是 serve 模式下的顶层入口，用来放全局初始化逻辑。

例如：

```zun
@CORS("*")
$main() {
    $>> "[INFO] server starting";
}

$GET("/health") health() -> String {
    $# "ok";
}
```

可以这样理解：

- 只需要一个简单路由时，可以不写 `$main()`
- 需要全局 `@CORS`、启动日志、启动前准备动作时，再写 `$main()`
- `entry` 负责选入口文件，`$main()` 负责该文件里的服务入口逻辑

## `$<<CONFIG(...)` 的使用边界

`[env]` 中的值可以通过 `$<<CONFIG(...)` 读取：

```zun
$ app = $<<CONFIG("APP_NAME");
```

但要记住：

- 它依赖项目配置上下文
- 当前实现里只在 `serve` 模式下可用
- 不应把它当成 `run` 模式下的通用配置入口

推荐的跨模式兜底写法是优先读环境变量，把 `CONFIG` 留给 serve 项目：

```zun
$mod std.env;

$ app = env.get_or("APP_NAME", "My App");
$>> app;
```

如果你明确只在 `serve` 项目里运行，再使用 `$<<CONFIG(...)`。

## `server` 配置

服务模式会读取 `[server]`：

```toml
[server]
host = "127.0.0.1"
port = 8080
```

`host` 当前只接受两个值：

- `127.0.0.1`
  - 仅本机访问，也是默认值
- `0.0.0.0`
  - 绑定所有网卡接口，允许外部访问

像 `localhost`、具体局域网 IP 或其他字符串，当前都会在加载阶段直接报配置错误。

启动 `zun serve .` 后，CLI 会用带颜色的地址提示输出：

- `127.0.0.1` 时打印 `Local: http://127.0.0.1:<port>`
- `0.0.0.0` 时同时打印：
  - `Local: http://127.0.0.1:<port>`
  - 如果能枚举到本机私网 IPv4，会逐条打印 `Network: http://<LAN_IP>:<port>`
  - 如果当前环境没有可用私网 IPv4，再回退成 `Network: listening on all interfaces (:<port>), use your LAN IP to access`

执行：

```bash
zun serve .
```

时，CLI 会加载这部分配置。

## 项目和单文件的区别

单文件适合：

- demo
- 小工具
- 单路由原型

项目目录适合：

- 多模块
- 配置读取
- 服务模式
- 后续继续扩展

## 进一步参考

- [12a-serve-config-file.md](12a-serve-config-file.md)
- [14-io-env-config.md](14-io-env-config.md)
- [16-http-organization.md](16-http-organization.md)
- [../reference/project-system.md](../reference/project-system.md)

## 下一章

继续看 [13-stdlib-overview.md](13-stdlib-overview.md)。
