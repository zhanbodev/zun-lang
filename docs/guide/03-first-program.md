# 3. 第一个程序

这一章只做三件事：跑第一个脚本、试一次 REPL、起第一个 HTTP 路由。

## 第一个脚本

创建 `hello.zun`：

```zun
$>> "Hello, Zun!";
```

运行：

```bash
zun run hello.zun
```

### 怎么确认成功

- 终端应该输出 `Hello, Zun!`
- 如果没有输出，先检查语句末尾是否有分号
- 如果提示找不到文件，确认你在 `hello.zun` 所在目录执行命令

## 第一个 REPL 交互

进入 REPL：

```bash
zun
```

输入：

```text
>> $ name = "builder";
>> $>> f"hello, {name}";
hello, builder
```

REPL 适合做这些事情：

- 试表达式
- 试字符串与列表方法
- 快速复现小问题

## 第一个 HTTP 路由

创建 `main.zun`：

```zun
$GET("/hello") hello() -> String {
    $# "world";
}
```

启动：

```bash
zun serve main.zun
```

### 怎么确认成功

先看终端没有报错，再用浏览器或 `curl` 验证：

```bash
curl http://127.0.0.1:8080/hello
```

预期结果：

- 响应体是 `world`
- 如果你改了 `[server]` 端口，就把 `8080` 换成对应端口
- 浏览器直接访问 `http://127.0.0.1:8080/hello` 也应看到同样结果

## 单文件和项目目录的区别

单文件适合：

- 小脚本
- 小型 demo
- 单路由接口

项目目录适合：

- 需要 `package.toml`
- 需要多个模块
- 需要统一 server 配置

项目目录的入口、`entry` 和 `$main()` 在 [12-projects-and-package.md](12-projects-and-package.md) 里详细说明。

## 下一章

继续看 [04-basic-syntax.md](04-basic-syntax.md)。
