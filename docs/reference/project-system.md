# Zun Project System

## `package.toml` Schema

Zun 项目根目录使用 `package.toml` 作为 manifest。当前正式支持的字段只有这些：

```toml
[project]
name = "demo"
version = "0.1.0"
entry = "main.zun"

[server]
host = "127.0.0.1"
port = 8080
upload_max_size = 50
upload_max_file_size = 10

[env]
APP_ENV = "dev"

[dependencies]
acme = "0.1.0"
```

字段说明：

- `[project].name`: 项目名，当前为必填
- `[project].version`: 项目版本
- `[project].entry`: 项目入口文件，默认 `main.zun`
- `[server]`: serve/test 模式使用的服务配置
  - `host`: 监听地址，默认 `"127.0.0.1"`
  - `port`: 监听端口，默认 `8080`
  - `upload_max_size`: 单次 multipart 请求总大小上限（MB），默认 `50`
  - `upload_max_file_size`: 单个上传文件大小上限（MB），默认 `10`
- `[env]`: 通过 `$<<CONFIG()` 可读取的项目配置项
- `[dependencies]`: 预留给第三方依赖解析的依赖清单

兼容性说明：

- 当前实现仍兼容旧写法：顶层 `name` / `version` / `entry`
- 新项目推荐使用 `[project]` 作为规范写法

## Module Resolution Order

Zun 当前的模块解析顺序是：

1. 当前文件相对路径
2. 项目根目录
3. 项目根目录下的 `modules/`
4. 项目根目录下的 `stdlib/`，仅用于 `std.*` 命名空间
5. 项目根目录下的 `deps/`，仅用于在 `[dependencies]` 中声明过的依赖

示例：

- `$mod shared.user;`
  - 先查当前文件同级的 `shared/user.zun`
  - 再查项目根的 `shared/user.zun`
  - 再查 `modules/shared/user.zun`

- `$mod std.io;`
  - 只查 `stdlib/io.zun`
  - 不会落到项目内普通模块，避免标准库命名空间被覆盖

## Entry Resolution

- 当 CLI 传入的是文件路径时，直接把该文件作为入口
  - `zun run <file.zun>` 按脚本模式执行，不读取上层 `package.toml`
  - 脚本所在目录就是该次运行的 root
- 当 CLI 传入的是目录时：
  - 先定位项目根
  - 再读取 `package.toml` 的 `[project].entry`
  - 如果没有 manifest，则默认使用 `main.zun`

## Relative Imports

相对导入不是通过 `./` 语法表达，而是通过“当前文件优先”的模块解析规则实现。

例如当前文件位于：

```text
app/main.zun
```

当它执行：

```zun
$mod shared.helper;
```

解释器会优先查找：

```text
app/shared/helper.zun
```

## Reserved Namespace

`std.*` 是保留命名空间。

建议标准库模块后续统一放在：

```text
stdlib/
├── io.zun
├── http.zun
├── json.zun
└── str.zun
```

当前阶段只保留命名空间和解析规则，不强行一次性填满标准库内容。
