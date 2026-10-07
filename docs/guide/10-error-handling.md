# 10. 错误处理

Zun 当前同时有解析错误、运行时错误和显式抛出的异常。

## 三类错误先怎么分

- 代码还没跑起来就出错：词法 / 语法错误
- 跑到一半因为值或环境问题出错：运行时错误
- 代码里主动抛出：`$throw`

## `$try / $catch`

```zun
$try {
    $throw "oops";
} $catch err {
    $>> err;
}
```

如果 `try` 块内部抛错，就会进入 `catch` 块。

## `err` 到底是什么

在主线文档里，可以先把 `err` 理解成“可打印、可比较、可继续传递的错误值”。

安全做法：

- 直接打印 `err`
- 把 `err` 拼进日志字符串
- 用 `err.type()` 看当前值类型
- 根据上下文返回默认值或改写响应

例如：

```zun
$mod std.fs;

$try {
    $ content = fs.read_text("missing.txt");
    $>> content;
} $catch err {
    $>>ERR(f"read failed: {err}");
}
```

主线不把 `err` 讲成运行时内部结构体，也不承诺固定字段集合。

## `$throw`

```zun
$throw "bad input";
```

适合：

- 主动拒绝非法输入
- 在工具函数里显式抛出失败
- 提前终止一条不该继续的路径

## 可捕获性速查

| 场景 | 能否被 `$catch` 捕获 | 备注 |
|------|----------------------|------|
| `$throw "oops"` | 可以 | 最直接的用法 |
| 除零 | 可以 | 运行时错误 |
| 未定义变量 | 可以 | 运行时错误 |
| 类型不匹配 | 可以 | 运行时错误 |
| 文件读取失败 | 可以 | 例如 `std.fs.read_text(...)` |
| 模块加载失败 | 不作为主线承诺 | 通常发生在装载阶段，优先先修模块路径 |
| 语法错误 | 不可以 | 程序尚未进入执行阶段 |

查表版说明见 [../reference/errors.md](../reference/errors.md)。

## 常见例子

### 文件读取失败

```zun
$mod std.fs;
$try {
    $ content = fs.read_text("missing.txt");
    $>> content;
} $catch err {
    $>> "caught!";
}
```

### 环境变量失败

```zun
$mod std.env;
$try {
    $ token = env.get("APP_TOKEN");
    $>> token;
} $catch err {
    $>> "env error caught";
}
```

### 主动抛错

```zun
$fn must_positive(n) -> Int {
    $if n <= 0 {
        $throw "n must be positive";
    }
    $# n;
}
```

## 怎么排查

1. 先分清是 parse error 还是 runtime error
2. 再看出错位置
3. 看是语法、值类型、模块、文件还是环境问题
4. 尽量把问题缩成最小 `.zun` 文件

## 下一章

继续看 [11-modules.md](11-modules.md)。
