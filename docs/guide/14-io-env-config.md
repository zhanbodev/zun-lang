# 14. I/O、环境变量与配置

这一章把和宿主环境直接交互的能力收在一起：环境变量、项目配置、文件系统。

## 环境变量

### 语言表面：`$<<ENV(...)`

```zun
$ home = $<<ENV("HOME");
```

如果环境变量不存在，这类调用会失败。

### 标准库：`std.env`

```zun
$mod std.env;
$>> env.get_or("APP_ENV", "dev");
```

当你需要兜底值时，优先选 `env.get_or(...)`。

## 项目配置：`$<<CONFIG(...)`

```zun
$ app = $<<CONFIG("APP_NAME");
```

当前实现里，它有两个重要边界：

- 依赖项目配置上下文
- 只在 `serve` 模式下可用

也就是说，在 `run` 模式里把它当成通用配置读取入口会直接报错。

### 推荐的跨模式写法

如果你希望脚本和服务都能运行，先用环境变量作为兜底主路径：

```zun
$mod std.env;

$ app = env.get_or("APP_NAME", "My App");
$>> app;
```

如果某段代码只在服务项目里运行，再使用 `$<<CONFIG(...)`。

## 文件系统：`std.fs`

文件能力现在统一主推 `std.fs`：

```zun
$mod std.fs;

$ text = fs.read_text("notes.txt");
$ lines = fs.read_lines("notes.txt");
$ exists = fs.exists("notes.txt");
$ is_dir = fs.is_dir("data");
```

如果你需要写入：

```zun
$mod std.fs;

fs.write("output.txt", "hello");
fs.append("output.txt", "\nworld");
$>> fs.read_text("output.txt");
fs.delete("output.txt");
```

主线推荐这样理解：

- `$>>`：输出表达式结果
- `std.fs`：文件读写与元信息能力
- `$>> fs.read_text(...)`：输出标准库返回值

## legacy 兼容语法

旧语法示例如下，但当前已经不再可用：

```zun
$ text = $<<FILE("notes.txt");
$ lines = $<<FILE("notes.txt", "LINES");
```

如果继续使用这类写法，parser 会直接报 `ZUN-P008`，并提示迁移到 `std.fs`。新的文件读写示例和项目代码应统一使用 `std.fs`。

## 为什么主推 `std.fs`

- API 更明确
- 支持写入、追加、删除
- 支持 `exists`、`is_dir`、`list` 等元信息
- 在 `run` 和 `serve` 模式下统一按项目根解析相对路径

## 文件读取示例

```zun
$mod std.fs;

$ content = fs.read_text("tests/fixtures/stdlib/hello.txt");
$ lines = fs.read_lines("tests/fixtures/stdlib/lines.txt");
```

## 文件写入

```zun
$mod std.fs;

fs.write("/tmp/zun-demo.txt", "hello");
$>> fs.read_text("/tmp/zun-demo.txt");
fs.delete("/tmp/zun-demo.txt");
```

## 常见坑

- 环境变量不存在会抛错，除非你用 `get_or`
- `$<<CONFIG(...)` 不是跨模式通用入口
- 文件路径错误会导致运行时错误
- `std.fs` 相对路径以项目根为基准
- 有副作用的能力最好配合 `$try / $catch`

## 下一章

继续看 [15-http-basics.md](15-http-basics.md)。
