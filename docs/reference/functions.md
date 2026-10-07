# 函数系统

> 迁移提示
>
> 本页是旧 reference 页面，仍有部分历史示例。
> 当前主线函数口径请优先参考 [../guide/07-functions.md](../guide/07-functions.md)、[../guide/09-gradual-typing.md](../guide/09-gradual-typing.md) 和 [syntax.md](syntax.md)。
> 如果本页示例与这些页面冲突，以主线和当前实现为准。

Zun 支持完整的函数功能，包括函数定义、参数传递、返回值、递归等。

## 函数定义

使用 `$fn` 声明函数，`$#` 返回值：

```zun
$fn add(a, b) {
    $ result = a + b;
    $# result;
}
```

**语法**：

```zun
$fn 函数名(参数1, 参数2, ...) {
    函数体;
    $# 返回值;
}
```

**使用技巧**：

- 函数名在全局唯一，不能重复定义
- 参数是函数的局部变量，外部无法直接访问
- `$#` 语句会立即返回，后面的代码不会执行

---

## 函数参数传递

参数按值传递，函数内部修改参数不会影响外部变量：

```zun
$fn increment(x) {
    x = x + 1;
    $>> x;
}

$ a = 10;
increment(a);    // 输出: 11
$>> a;           // 输出: 10 (a 未改变)
```

---

## 可变参数函数

Zun 支持可变参数函数，使用 `...参数名` 语法收集多余参数到列表。

### 基本用法

```zun
$fn sum(...nums) {
    $# nums;
};

$>> sum(1, 2, 3);
[1, 2, 3]
```

**语法**：

```zun
$fn 函数名(...参数名) {
    函数体;
}
```

### 混合普通参数和可变参数

可变参数必须是最后一个参数：

```zun
$fn greet(name, ...others) {
    $# [name, others];
};

$>> greet("Tom");
[Tom, []]
$>> greet("Tom", "Jerry", "Bob");
[Tom, [Jerry, Bob]]
```

### 获取参数个数

使用 `.len()` 方法获取传入的参数个数：

```zun
$fn countArgs(...args) {
    $# args.len();
};

$>> countArgs();
0
$>> countArgs(1);
1
$>> countArgs(1, 2, 3);
3
```

### 匿名可变参数函数

```zun
$result = $fn(...args) {
    $# args;
};

$>> result(1, 2);
[1, 2]
$>> result();
[]
```

---

## 返回语句

使用 `$#` 返回值：

```zun
$fn double(x) {
    $# x * 2;
}

$>> double(5);
10
```

---

## 返回类型声明

可以使用 `-> Type` 指定函数返回类型：

```zun
$fn double(x) -> Int {
    $# x * 2;
}

$>> double(5);
10
```

实现说明：

- 当前主线里，函数返回类型和 HTTP handler 返回类型在解释器内部已经统一使用结构化 `TypeExpr`
- 这属于内部表示层统一，不等于额外新增了一批新的稳定语法承诺
- 阅读本页时，仍应以 `-> T`、`-> List<T>`、`-> JSON`、`-> HTML` 这些已写明的口径为准

支持的类型：
- `Int` / `Integer` - 整数类型
- `Float` - 浮点数类型
- `String` - 字符串类型
- `Bool` / `Boolean` - 布尔类型

### HTTP 返回类型

在 HTTP 路由中，还可以使用以下返回类型：

- `JSON` - JSON 响应（默认）
- `HTML` - HTML 页面响应
- `String` - 纯文本响应

```zun
// JSON 响应
$GET("/api/users") get_users() -> JSON {
    $# $JSON { "users": ["tom", "jerry"] };
}

// HTML 响应
$GET("/pages/home") home() -> HTML {
    $# "<h1>Welcome</h1><p>Hello World!</p>";
}
```

### 历史写法提示：`JSON<TypeName>`

以下是旧 reference 中曾出现过的签名形式：`$fn getUser(id) -> JSON<User> { ... }`

当前主线不再把 `JSON<TypeName>` 当作稳定教学写法。阅读函数签名时，优先使用：

- 普通 `-> JSON`
- 或集合返回写成 `-> List<T>`

涉及 `$Type` 的结构表达，请以 [../guide/09-gradual-typing.md](../guide/09-gradual-typing.md) 为准。

---

**返回类型不匹配时报错**：

```zun
$fn test() -> String {
    $# 123;
}
```

运行结果：
```
[ERROR] runtime error: function 'test' expects return type 'String' but got 'Int'
```

**函数有返回值但未声明返回类型时报错**：

```zun
$fn add(x, y) {
    $# x + y;
}
```

运行结果：
```
[ERROR] runtime error: function 'add' has no return type declared but returns a value
```

---

## 无返回值函数

```zun
$fn greet(name) {
    $>> "Hello, " + name;
}

greet("Zun");
```

运行结果：
```
Hello, Zun
```

---

## 函数调用

```zun
$fn add(a, b) {
    $# a + b;
}

$ sum = add(1, 2);
$>> sum;
3
```

---

## 匿名函数

Zun 支持将函数赋值给变量，实现函数作为一等公民（first-class functions）：

```zun
$a = 20;
$b = 30;

$result = $fn(x,y) {
 $>> x + y;
};

result(a,b);
50
```

**语法**：

```zun
$变量名 = $fn(参数1, 参数2, ...) {
    函数体;
};
```

- 匿名函数不需要函数名
- 函数体末尾需要分号结束
- 可以像调用普通函数一样调用通过变量持有的函数

**带返回值的匿名函数**：

```zun
$result = $fn(x, y) -> Int {
    $# x + y;
};

$>> result(10, 20);
30
```

**无返回值的匿名函数**：

```zun
$result = $fn(x, y) {
    $>> x + y;
};

result(10, 20);
30
```

---

## 递归支持

Zun 支持递归函数：

```zun
$fn factorial(n) {
    $if n <= 1 {
        $# 1;
    } $else {
        $# n * factorial(n - 1);
    }
}

$>> factorial(5);
120
```

---

## 作用域规则

函数内部有自己的作用域，外部变量无法直接访问：

```zun
$ x = 10;

$fn test() {
    $>> x;  // ERROR: variable 'x' not found
}

test();
```

要访问外部变量，需要通过参数传递：

```zun
$fn print_val(x) {
    $>> x;
}

$ x = 10;
print_val(x);  // 输出: 10
```

---

## 函数调用错误

### 调用未定义的函数

```zun
foo();
```

运行结果：
```
[ERROR] runtime error: function 'foo' is not defined
```

### 函数重定义错误

```zun
$fn add(a, b) {
    $# a + b;
}
$fn add(a, b) {
    $# a;
}
```

运行结果：
```
[ERROR] runtime error: function 'add' is already defined
```

---

## 方法调用

Zun 支持对数据进行方法调用，方法调用**必须使用括号**。

### 语法

```zun
对象.方法名(参数);
```

### 示例

```zun
$ s = "hello";
$>> s.upper();        // 输出: HELLO
$>> s.lower();        // 输出: hello
$>> s.len();          // 输出: 5

$ arr = [1, 2, 3];
$>> arr.len();        // 输出: 3
$>> arr.push(4);     // 输出: [1, 2, 3, 4]

$ n = 123;
$>> n.to_str();       // 输出: "123"
```

### 错误：方法调用必须使用括号

```zun
$ s = "hello";
$>> s.upper;          // ❌ 错误
```

运行结果：
```
[ERROR] Parse error at line 1:1
  found: Ident upper
  expected: (
  method 'upper' requires parentheses, use 'upper(...)' instead
```

---

## 标准输入

Zun 支持从标准输入读取数据，使用 `$<<` 关键字。

### 读取环境变量 `$<<ENV`

```zun
$ home = $<<ENV("HOME");
$>> home;             // 输出: /Users/xxx
```

**错误情况**：

```zun
$ x = $<<ENV("NOT_EXIST");
```

运行结果：
```
[ERROR] runtime error: environment variable 'NOT_EXIST' is not defined
```

### 读取 stdin `$<<LINE`

```zun
// 无提示输入
$ name = $<<LINE();
$>> name;

// 有提示输入
$ age = $<<LINE("请输入年龄: ");
$>> age;
```

### 不赋值（pause 效果）

单独使用 `$<<LINE()` 会等待用户输入，按回车后继续，不保存输入：

```zun
$>> "按回车继续...";
$<<LINE();
$>> "程序继续执行";
```

---

## 文件读写

Zun 当前主线通过 `std.fs` 提供文件读写。

### 主推荐写法：`std.fs`

```zun
$mod std.fs;

fs.write("output.txt", "Hello World");
fs.append("output.txt", "\nSecond line");

$ text = fs.read_text("output.txt");
$ lines = fs.read_lines("output.txt");
$ exists = fs.exists("output.txt");
$ size = fs.size("output.txt");

$>> text;
$>> lines;
$>> exists;
$>> size;

fs.delete("output.txt");
```

常用能力：

- `fs.write(path, content)`：覆盖写入
- `fs.append(path, content)`：追加写入
- `fs.delete(path)`：删除文件
- `fs.read_text(path)`：读取全文
- `fs.read_lines(path)`：按行读取
- `fs.exists(path)`：检查文件或目录是否存在
- `fs.size(path)`：读取文件大小
- `fs.is_dir(path)`：判断路径是否是目录

### 与 `$>>` 的配合

`$>>` 只是输出表达式结果，所以标准库返回值可以直接打印：

```zun
$mod std.fs;

$>> fs.read_text("notes.txt");
$>> fs.exists("notes.txt");
```

### legacy 兼容说明

`$>>FILE(...)` / `$<<FILE(...)` 已经移除；继续使用时会报 `ZUN-P008`，并提示迁移到 `std.fs`。新文档和新项目应统一使用 `std.fs`。

---

## JSON 和 HTML 方法

JSON 和 HTML 类型支持以下内置方法：

### JSON 方法

```zun
$ j = $JSON {"name": "John", "age": 30};

// 转换为字符串
$ str = j.to_str();

// 获取类型名
$ t = j.type();
```

**可用方法**：
- `.to_str()` - 将 JSON 对象转换为字符串表示
- `.type()` - 返回类型名 "Json"

### HTML 方法

```zun
$ h = $HTML("Hello World");

// 转换为字符串
$ str = h.to_str();

// 获取类型名
$ t = h.type();

// 链接外部文件
$ h = $HTML().link("pages.index");
```

**可用方法**：
- `.to_str()` - 将 HTML 内容转换为字符串
- `.type()` - 返回类型名 "Html"
- `.link("module.path")` - 链接外部 HTML/CSS/JS/XML 文件

---

## 模块系统

Zun 支持模块系统，允许将代码拆分到多个文件中，通过 `$mod` 关键字导入。

### 模块声明 `$mod`

使用 `$mod 路径;` 导入模块：

```zun
$mod dao.user;
```

**语法**：

```zun
$mod 模块路径;
$mod 目录路径.*;
```

模块路径支持点分隔符：
- `$mod dao.user;` 会加载 `dao/user.zun` 文件
- `$mod services.*;` 会导入 `services/` 目录下的直接子模块

### 访问方式

模块导入后通过文件名命名空间访问，不会平铺到全局函数表：

```zun
$mod dao.user;

$main() {
    $ result = user.get_user("123");
    $>> result;
};
```

### 公开与私有

- `$fn` 默认公开
- `_$fn` 默认私有
- 私有函数不会被 `$mod` 导入
- `$mod` 当前只导入函数，不导入变量、常量、HTTP 路由或 `$main`

### 模块搜索路径

模块按以下顺序搜索：
1. 当前目录：`dao/user.zun`
2. modules 目录：`modules/dao/user.zun`

### 示例

**项目结构**：

```
my-project/
├── main.zun
├── package.toml
└── dao/
    └── user.zun
```

**dao/user.zun**（模块文件）：

```zun
$fn get_user(id) -> String {
    $# "User-" + id;
};

_$fn format_email(name, email) -> String {
    $# "Created: " + name + " (" + email + ")";
};

$fn create_user(name, email) -> String {
    $# format_email(name, email);
};
```

**main.zun**（主文件）：

```zun
$mod dao.user;

$main() {
    $ result = user.get_user("123");
    $>> result;           // 输出: User-123

    $ created = user.create_user("Tom", "tom@example.com");
    $>> created;         // 输出: Created: Tom (tom@example.com)
};
```

### 错误情况

```zun
$mod non.existent.module;
```

运行结果（如果模块文件不存在）：
```
[ERROR] runtime error: module not found: 'non.existent.module' (tried: non/existent/module.zun, modules/non/existent/module.zun)
```

如果尝试访问私有函数：

```zun
$mod dao.user;
$ value = user.format_email("Tom", "tom@example.com");
```

会报模块函数不存在错误。

---

## 项目系统

Zun 支持项目系统，提供服务模式、配置管理和主入口功能。

### CLI 命令

```bash
zun               # REPL 交互模式
zun run <file>   # 运行单个 .zun 文件
zun serve [path] # 服务模式（默认当前目录）
```

### 服务模式 `zun serve`

服务模式用于运行项目，会自动加载配置和执行主入口：

```bash
zun serve .           # 从当前目录加载
zun serve main.zun    # 指定主入口文件
```

### 主入口 `$main`

使用 `$main()` 定义程序入口，仅在服务模式下执行：

```zun
$main() {
    $>> "Server started!";
    $>> "Loading configuration...";
};
```

**注意**：
- `$main()` 只能在 `main.zun` 文件中使用
- 只有在 `zun serve` 模式下才会执行

### 配置文件 package.toml

在项目根目录创建 `package.toml` 文件：

```toml
[project]
name = "my-project"
version = "0.1.0"
entry = "main.zun"

[server]
port = 3000
host = "127.0.0.1"

[env]
DB_URL = "postgres://localhost/db"
API_KEY = "your-api-key"
```

**配置项**：
| 键 | 说明 | 默认值 |
|---|---|---|
| `[project].name` | 项目名称 | - |
| `[project].version` | 项目版本 | - |
| `[project].entry` | 入口文件 | `main.zun` |
| `[server].port` | 服务器端口 | `8080` |
| `[server].host` | 服务器地址，仅支持 `127.0.0.1` 或 `0.0.0.0` | `127.0.0.1` |
| `[env].*` | 环境变量键值对 | - |

### 配置读取 `$<<CONFIG`

在服务模式下，可以使用 `$<<CONFIG("KEY")` 读取配置：

```zun
$mod dao.user;

$main() {
    $ db_url = $<<CONFIG("DB_URL");
    $>> f"Connecting to: {db_url}";

    $ api_key = $<<CONFIG("API_KEY");
    $>> f"API Key loaded: {api_key}";
};
```

**注意**：`$<<CONFIG` 仅在服务模式下可用

运行结果：
```
$ zun serve .
Loaded project: my-project v0.1.0
Connecting to: postgres://localhost/db
API Key loaded: your-api-key
```

### 错误情况

```zun
// 在 REPL 模式下使用 $<<CONFIG
$ a = $<<CONFIG("DB_URL");
```

运行结果：
```
[ERROR] runtime error: $<<CONFIG() is only available in serve mode
```
