# 17. 测试与调试

这一章给一条最实用的工作流：先验证最小功能，再缩小问题面，再做最小复现。

## 1. 断言与基本验证

最轻量的验证方式仍然是直接运行并看输出：

```zun
$fn add(a, b) -> Int {
    $# a + b;
}

$>> add(1, 2);
```

运行：

```bash
zun run main.zun
```

预期看到：

```text
3
```

对脚本逻辑来说，先把“输入是什么、输出是什么”写清楚，比一开始就堆复杂测试框架更重要。

如果你想把“预期必须成立”写进代码里，优先用 `std.core.check.assert`：

```zun
$mod std.core.check;

check.assert(1 + 1 == 2, "基础算术失败");
check.assert("hello".len() == 5, "字符串长度失败");
$>> "checks passed";
```

运行：

```bash
zun run test_math.zun
```

通过时，你会看到：

```text
checks passed
```

断言失败时，程序会直接报错并停止，这很适合写最小脚本验证。

## 2. 测试文件组织建议

当例子开始变大，推荐直接使用 `.zun` 测试脚本：

```text
my-app/
├── main.zun
├── shared/
│   └── helper.zun
└── tests/
    ├── test_math.zun
    └── test_users.zun
```

例如 `tests/test_math.zun`：

```zun
$mod std.core.check;

check.assert(1 + 1 == 2, "1 + 1 应该等于 2");
check.assert([1, 2, 2].count(2) == 2, "count(2) 应该等于 2");
$>> "test_math ok";
```

当前仓库也可以直接参考：

- `tests/spec/valid/`
- `tests/spec/invalid/`
- `tests/fixtures/`

这些目录最适合看两件事：

- 最小正确写法是什么
- 最小错误复现应该缩到多小

## 3. 用 `zun test` 验证 HTTP

最常用命令：

```bash
zun test
zun test main.zun
zun test --route GET /hello
zun test --route POST /echo --body '{"name":"Tom"}'
```

### 脚本验证路径

对纯脚本逻辑，推荐顺序：

1. 先 `zun run tests/test_xxx.zun`
2. 必要时拆成更小函数
3. 再把问题缩成最小文件

### HTTP 验证路径

对 HTTP handler，推荐顺序：

1. 先 `zun test --route ...`
2. 再 `zun serve ...` 做真实请求验证
3. 只在需要时再扩到完整项目路径

## 4. 终端输出怎么看

### 一个通过的例子

```text
$ zun test --route GET /hello
[OK] GET /hello
```

### 一个失败的例子

```text
$ zun run bad.zun
[ERROR] runtime error: variable 'x' not found
```

遇到失败时，先做三件事：

1. 看错误类别
2. 看报错位置
3. 看是语法、类型、模块、文件还是 HTTP 路由问题

## 5. 调试工作流

### 用 REPL 做最小复现

REPL 适合：

- 试表达式
- 试类型行为
- 试字符串和列表方法

### 用打印定位问题

```zun
$>> "debug";
$>> value;
$>>ERR(f"unexpected: {value}");
```

最适合放打印的位置：

- 分支判断前后
- 文件读取之后
- 返回之前
- `catch` 块里

### 用 `$try / $catch` 包住不稳定点

```zun
$mod std.fs;
$try {
    $ content = fs.read_text("missing.txt");
    $>> content;
} $catch err {
    $>>ERR(f"read failed: {err}");
}
```

## 6. 常见排障配方

### handler 返回空 body

- 现象：请求成功，但没有你预期的响应体
- 检查点：handler 是否真的执行到 `$#`
- 推荐动作：先改成最小固定返回，例如 `# "ok"` 或固定 JSON

### 模块函数没被调用

- 现象：程序能跑，但结果像是走了旧逻辑
- 检查点：是否写了正确的 `$mod` 路径，是否通过命名空间调用
- 推荐动作：先在入口处打印模块调用结果，确认不是旧页面里的平铺导入预期

### 类型注解相关报错

- 现象：声明或返回时提示类型不匹配
- 检查点：变量注解是否与实际值一致，返回值是否用了 `List<T>`
- 推荐动作：先缩成最小函数，再回看 [09-gradual-typing.md](09-gradual-typing.md) 和 [10-error-handling.md](10-error-handling.md)

## 下一章

继续看 [18-patterns-and-recipes.md](18-patterns-and-recipes.md)。
