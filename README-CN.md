<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-wordmark-dark.svg">
    <img src="assets/logo-wordmark-light.svg" alt="Zun" width="360">
  </picture>
</p>

<p align="center">
  <strong>小如蜂鸟，快如蜂鸟。</strong><br>
  为 HTTP 服务而生的小而全的语言：一个文件、一条命令、不到 10 MB 内存。
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README-CN.md">中文</a>
</p>

---

```zun
$GET("/hello/:name") hello(name) -> JSON {
    $# { "message": "Hello, " + name + "!" };
}
```

```bash
zun serve
# ➜  Local:   http://127.0.0.1:8080
```

这就是一个完整的 Web 服务。不需要框架，不需要服务器样板代码，也不需要依赖注入容器。

## 为什么是 Zun

蜂鸟只有一枚硬币那么重，却能悬停、倒飞，每秒振翅 50 次。Zun 追求的就是这种平衡：

| | |
|---|---|
| 🪶 **小** | 只有一个二进制文件。一个 `main.zun` 就是一个服务。HTTP 路由是语法本身，而不是一个库。 |
| 🍃 **轻** | 压测时常驻内存只有 **9.8 MB**，大约是 FastAPI 的 1/4、Node + Hono 的 1/6。基于 Rust 构建，没有垃圾回收停顿。 |
| 🧰 **全** | 开箱即用：HTTP 服务端和客户端、SQLite 和 PostgreSQL、Session、JWT、密码哈希、CSRF、JSON、文件系统、测试运行器、REPL 和 LSP。 |
| ⚡ **快** | 压测中约 **4.2 万 QPS**，p99 为 **1.84 ms**，吞吐量大约是 FastAPI 的 6 倍。 |

Zun 专注于后端里一块范围小、价值高的领域：API 网关、边缘函数、Sidecar 和内部工具。在这些场景里，你想要脚本语言的手感，又想要 Rust 级别的资源占用。

## 快速开始

需要先安装 stable 版的 Rust 工具链。

```bash
git clone https://github.com/zhanbodev/zun-lang.git
cd zun-lang
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

新建 `main.zun`：

```zun
$>> "Hello, Zun!";
```

然后运行：

```bash
zun run main.zun
```

## 语言速览

所有关键字都以 `$` 开头，所以语言结构和你自己起的名字一眼就能分开。类型注解只在有帮助的地方才需要写。

```zun
// 变量与常量
$ name = "Zun";
$ version: Int = 1;
$@ MOTTO = "tiny, light, complete, fast";

// 函数
$fn greet(who: Str) -> Str {
    $# "Hello, " + who + "!";
}

// 记录类型
$Type Bird {
    name: Str
    weight: Float
}

$ birds = [
    Bird { name: "Bee hummingbird", weight: 1.6 },
    Bird { name: "Ruby-throated", weight: 3.4 }
];

// 控制流
$for bird in birds {
    $ grams = bird.weight;
    $if grams < 2.0 {
        $>> bird.name;
    }
}

// 错误处理
$try {
    $throw "out of nectar";
} $catch err {
    $>> "caught: " + err;
}

$>> greet(name);
```

| 符号 | 含义 |
|---|---|
| `$ x = …` | 声明变量 |
| `$@ X = …` | 声明常量 |
| `$fn` | 定义函数 |
| `$#` | 返回 |
| `$>>` | 输出到标准输出 |
| `$mod` | 导入模块 |
| `$Type` | 定义记录类型 |

## 第一个 HTTP 服务

```text
hello-api/
├── package.toml
├── main.zun
└── app/router/birds.zun
```

`main.zun` 把一个路由模块挂到指定的路径前缀下：

```zun
$main() {
    $HTTP("/api").link("app.router.birds");
}
```

`app/router/birds.zun` 里写路由。路径参数会作为 handler 的参数传进来，JSON 请求体可以通过 `body` 读取：

```zun
$GET("/hello/:name") hello(name) -> JSON {
    $# { "message": "Hello, " + name + "!" };
}

$POST("/birds") create() -> JSON {
    $ name = body["name"];
    $if name == "" {
        $# $RES(400, { "error": "name is required" });
    }
    $# $RES(201, { "created": name });
}
```

```bash
zun serve --routertab
curl localhost:8080/api/hello/Zun          # {"message":"Hello, Zun!"}
curl -X POST localhost:8080/api/birds \
     -H 'content-type: application/json' \
     -d '{"name":"Bee"}'                    # 201 {"created":"Bee"}
```

项目的所有配置都写在 `package.toml` 里：

```toml
[project]
name    = "hello-api"
version = "0.1.0"
entry   = "main.zun"

[server]
host                 = "0.0.0.0"
port                 = 8080
upload_max_size      = 50   # 请求体总大小上限，单位 MB
upload_max_file_size = 10   # 单个文件大小上限，单位 MB
```

## 命令行

| 命令 | 作用 |
|---|---|
| `zun` | 启动 REPL |
| `zun run <file.zun>` | 运行一个脚本 |
| `zun serve [path]` | 启动项目的 HTTP 服务。`--routertab` 会打印路由表 |
| `zun test [file]` | 运行测试（默认是 `main.zun`）。用 `--route METHOD /path` 和 `--body JSON` 可以单独测试一个路由 |

Zun 只执行 `.zun` 文件。

## 标准库

| 领域 | 模块 |
|---|---|
| 核心 | `std.str` · `std.math` · `std.json` · `std.time` · `std.uuid` · `std.env` · `std.fs` |
| 网络 | `std.http`（出站 HTTP 客户端）。服务端能力内置在语言里 |
| 数据 | `std.sqlite` · `std.postgres` |
| 认证 | `std.auth.session` · `std.auth.jwt` · `std.auth.password` · `std.auth.guard` · `std.auth.csrf` |

完整 API 见 [docs/reference/stdlib-api.md](docs/reference/stdlib-api.md)。

## 性能

三个服务实现了完全相同的 `/hello` 和 `/greet/:name` 接口，在同一台机器上用同一个客户端压测。可以用 [`benches/http-comparison/`](benches/http-comparison/) 复现结果。

| 对象 | 常驻内存 | 冷启动 | `/hello` QPS | p99 |
|:--|--:|--:|--:|--:|
| **Zun** | **9.8 MB** | 1011 ms | **42,431** | **1.84 ms** |
| FastAPI + uvicorn | 43.3 MB | 3108 ms | 6,814 | 16.57 ms |
| Hono + Node.js | 63.0 MB | **76 ms** | 39,025 | 1.96 ms |

如实解读这些数字：

- **内存是最大的优势。** 同一台机器能跑的 Zun 服务数量，大约是 FastAPI 的 4 倍、Hono 的 6 倍。
- **吞吐量和 Hono 打平。** 两者碰到的都是 Python 压测客户端的上限，而不是服务本身的上限。
- **冷启动还是短板。** 目前大约需要 1 秒。对长期运行的服务没有影响，但对 FaaS 场景有影响。

## 目录结构

| 路径 | 内容 |
|---|---|
| `crates/zun-frontend/` | 词法分析、语法分析、AST、诊断信息 |
| `crates/zun-runtime/` | 解释器、模块解析、标准库 |
| `crates/zun-cli/` | `zun` 命令行：REPL、`run`、`serve`、`test` |
| `crates/zun-lsp/` | Language Server Protocol 实现 |
| `stdlib/` | 用 Zun 编写的标准库模块 |
| `benches/` | 微基准测试和 HTTP 对比压测 |
| `docs/` | 教程、参考手册、规范 |
| `tests/` | 集成测试和规范用例 |
| `assets/` | Logo 文件 |

## 文档

- [入门教程](docs/guide/README.md)：一步步学习 Zun
- [语法](docs/reference/syntax.md) · [类型](docs/reference/types.md) · [HTTP](docs/reference/http.md) · [SQL](docs/reference/sql-database.md)
- [项目系统](docs/reference/project-system.md) · [标准库](docs/reference/stdlib-api.md) · [错误码](docs/reference/errors.md)
- [贡献者指南](docs/contributing/dev-guide.md)

## 项目状态

Zun 还很年轻，1.0 之前可能会有不兼容的改动。目前已知的不足：

- 冷启动大约需要 1 秒。
- 数据库只支持 SQLite 和 PostgreSQL，还不支持 MySQL 和 Redis。
- 还没有包管理仓库，项目通过本地模块和 `package.toml` 组织。
- 可观测性比较弱，OpenTelemetry 支持在计划中。

## 参与贡献

提交改动前，请先运行：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

参见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。安全问题请按 [SECURITY.md](SECURITY.md) 的方式报告。

## 许可证

[MIT](LICENSE)
