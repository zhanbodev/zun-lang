# 2. 安装与 CLI

这一章介绍如何构建 Zun，并理解它当前的几个主要命令入口。

## 安装

如果你在源码仓库中工作，可以直接本地编译：

```bash
git clone https://github.com/zhanbodev/zun-lang.git
cd zun-lang
cargo build --release
```

可选地把二进制加入 `PATH`：

```bash
export PATH="$PATH:$(pwd)/target/release"
```

## CLI 总览

当前 CLI 主入口包括：

- `zun`
- `zun run <file>`
- `zun serve [path]`
- `zun test [options]`

### 进入 REPL

不带参数启动时，进入 REPL：

```bash
zun
```

你可以在里面直接执行语句：

```text
>> $>> "Hello!";
Hello!
>> $ x = 1 + 2;
>> $>> x;
3
```

退出 REPL 的常见方式：

- 输入 `exit`
- 输入 `quit`
- 在空输入时按 `Ctrl+D`

常见快捷操作：

- `↑` / `↓` 查看历史记录
- `Ctrl+A` 跳到行首
- `Ctrl+E` 跳到行末
- `Ctrl+C` 取消当前输入
- `Ctrl+D` 在空行时退出

### 运行单个文件

运行 `.zun` 文件：

```bash
zun run hello.zun
```

`run` 当前要求传入 `.zun` 文件路径。

### 启动服务模式

服务模式用于加载 HTTP 路由并启动服务：

```bash
zun serve .
```

也可以直接指定脚本文件或项目目录：

```bash
zun serve main.zun
zun serve .
```

打印路由表：

```bash
zun serve . --routertab
```

### 测试模式

HTTP 测试入口：

```bash
zun test
zun test main.zun
```

测试某条指定路由：

```bash
zun test --route GET /hello
```

给 `POST` / `PUT` 之类的请求附带 body：

```bash
zun test --route POST /users --body '{"name":"Tom"}'
```

## 命令和运行模式的关系

可以把 CLI 理解成四种工作模式：

- REPL：交互试验
- Run：执行脚本
- Serve：启动 HTTP 服务
- Test：对路由进行测试

其中 `serve` 和 `test` 会更多地涉及项目配置、HTTP 路由和模块加载。

## 一个建议的上手顺序

第一次使用时，建议按这个顺序：

1. 先进入 `zun` 试几行表达式
2. 再用 `zun run hello.zun` 跑单文件
3. 再用 `zun serve main.zun` 启动一个最小路由
4. 最后再看 `zun test`

## 迁移期参考

旧版快速开始仍保留在 [getting-started.md](getting-started.md)。

## 下一章

继续看 [03-first-program.md](03-first-program.md)。
