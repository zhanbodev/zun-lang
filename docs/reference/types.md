# 类型系统

> 迁移提示
>
> 本页是旧 reference 页面，仍保留历史示例。
> 当前主线类型口径请优先参考 [../guide/09-gradual-typing.md](../guide/09-gradual-typing.md)、[errors.md](errors.md) 和 [syntax.md](syntax.md)。
> 如果本页出现旧的泛型或返回类型写法，以主线和当前实现为准。

Zun v1.3 采用**渐进式类型系统**，结合了动态语言的灵活性和静态类型的安全性。

> **设计原则：能跑就不用加，怕出错再加。**

---

## 快速入门

### 动态模式（默认）

不写类型注解 → 完全动态，变量可以随时改类型：

```zun
$ x = 30;
x = "hello";   // ✅ 允许，类型从 Int 变为 String
x = true;      // ✅ 允许，类型从 String 变为 Bool
```

### 静态模式（可选）

写了类型注解 → 静态检查，类型保护生效：

```zun
$ x: Int = 30;
x = 100;       // ✅ 允许，同为 Int
x = "hello";   // ❌ 报错，类型不匹配
```

---

## 支持的数据类型

## 整数 (Integer)

表示整数值，支持正数和负数：

```zun
$>> 42;
42

$>> -17;
-17
```

**数值范围**：取决于实现，通常为 64 位有符号整数

**整数方法**：

```zun
$>> (42).to_str();      // 转换为字符串: "42"
$>> (42).to_float();    // 转换为浮点数: 42.0
$>> (1).to_bool();      // 转换为布尔: true
$>> (0).to_bool();      // 转换为布尔: false

$>> (42).type();        // 获取类型名称: Int
```

> 注意：整数方法调用需要用括号包裹，如 `(42).to_str()`

---

## 浮点数 (Float)

表示小数数值：

```zun
$>> 3.14;
3.14

$>> -2.5;
-2.5

$>> 0.5;
0.5
```

**浮点数方法**：

```zun
$>> (3.14).to_str();    // 转换为字符串: "3.14"
$>> (3.14).to_int();    // 转换为整数（截断）: 3

$>> (3.14).type();      // 获取类型名称: Float
```

> 注意：浮点数方法调用需要用括号包裹，如 `(3.14).to_int()`

---

## 字符串 (String)

使用双引号 `"..."` 包裹的文本：

```zun
$>> "Hello World";
Hello World

$>> "";
(空输出)

$>> "Dao" + "Lang";
Zun
```

**转义字符**：

```zun
$>> "Hello\nWorld";         // 换行: Hello
                           //          World
$>> "Hello\tWorld";         // 制表符: Hello   World
$>> "He said \"Hi\"";       // 双引号: He said "Hi"
$>> "C:\\path\\file";       // 反斜杠: C:\path\file
$>> "Line1\rLine2";        // 回车
$>> "Hello\0World";        // 空字符
```

**f-string 同样支持转义**：

```zun
$ name = "World";
$>> f"Hello\n{name}";      // 输出: Hello
                           //          World
```

**特性**：
- 支持 `+` 运算符进行字符串连接
- 内部统一存储为字符串类型
- 支持转义字符：`\n`, `\t`, `\r`, `\0`, `\\`, `\"`

**字符串方法**：

```zun
$ s = "Hello, World";

$>> s.len();              // 获取字符串长度: 12
$>> s.upper();            // 转大写: HELLO, WORLD
$>> s.lower();            // 转小写: hello, world

$>> "  hello  ".trim();   // 去除首尾空格: hello

$>> "hello".contains("ell");      // 判断包含: true
$>> "hello".starts_with("he");   // 判断前缀: true
$>> "hello".ends_with("lo");     // 判断后缀: true

$>> "hello".replace("l", "r");  // 替换: herro

$>> "a,b,c".split(",");         // 分割返回列表: [a, b, c]

$>> "hello".slice(1, 3);       // 截取子串: el

$>> "42".to_int();      // 转换为整数: 42
$>> "3.14".to_float();  // 转换为浮点数: 3.14

$>> "true".to_bool();   // 转换为布尔: true
$>> "false".to_bool();  // 转换为布尔: false

$>> "hello".type();     // 获取类型名称: String
```

**格式化字符串 (f-string)**：

```zun
$ name <& "World";
$>> f"Hello {name}";              // 输出: Hello World

$ age <& 25;
$>> f"I'm {age} years old";       // 输出: I'm 25 years old

$>> f"1 + 2 = {1 + 2}";          // 输出: 1 + 2 = 3

$ price <& 19.99;
$>> f"Price: {price.to_str()}";   // 输出: Price: 19.99
```

**f-string 特性**：
- 使用 `f"..."` 语法
- 花括号 `{...}` 内可以是变量、表达式或方法调用
- 支持类型自动转换（Int/Float/Bool 会自动调用 to_str()）

**错误示例**：

```zun
$>> f"Hello {}";
// [ERROR] Parse error: f-string syntax error: empty expression in "{}"

$>> f"Hello";
// [ERROR] f-string variable "undefined" is not defined
```

---

## 字符 (Char)

使用单引号 `'...'` 包裹的单个字符：

```zun
$>> 'a';
a

$>> 'Z';
Z
```

**注意**：字符类型在内部也以字符串形式存储

---

## 布尔 (Boolean)

表示真值，`true` 或 `false`：

```zun
$>> true;
true

$>> false;
false
```

**注意**：布尔类型主要用于条件判断

**布尔方法**：

```zun
$>> true.to_str();      // 转换为字符串: "true"
$>> false.to_str();     // 转换为字符串: "false"

$>> true.type();        // 获取类型名称: Bool
```

---

## 列表 (List)

表示一组有序的值集合：

```zun
$ arr = [1, 2, 3];
$>> arr[0];
1

$>> arr[2];
3
```

**空列表**：

```zun
$ empty = [];
$>> empty;
[]
```

**混合类型**：

```zun
$ mixed = [1, "hello", true];
$>> mixed[0];
1

$>> mixed[1];
hello

$>> mixed[2];
true
```

**嵌套列表**：

```zun
$ matrix = [[1, 2], [3, 4]];
$>> matrix[0][0];
1

$>> matrix[1][1];
4
```

**修改列表元素**：

```zun
$ arr = [1, 2, 3];
arr[1] = 99;
$>> arr[1];
99
```

**列表方法**：

```zun
$ arr = [1, 2, 3];
$>> arr.len();           // 获取列表长度: 3

$>> arr.contains(1);    // 检查是否包含元素: true
$>> arr.contains(5);   // 检查是否包含元素: false

$>> arr.push(4);        // 末尾添加元素: [1, 2, 3, 4]
$>> arr.pop();          // 移除末尾元素并返回: 4

$>> arr.reverse();       // 反转列表: [3, 2, 1]

$>> arr.join(", ");     // 拼接为字符串: "1, 2, 3"

$>> arr.type();         // 获取类型名称: List
```

---

## 字典 (Map)

表示键值对集合：

```zun
$ user = {"name": "Tom", "age": 18};
$>> user["name"];
Tom

$>> user["age"];
18
```

**空字典**：

```zun
$ empty = {};
$>> empty;
{}
```

**修改字典值**：

```zun
$ user = {"name": "Tom"};
user["email"] = "tom@example.com";
$>> user["email"];
tom@example.com
```

**嵌套字典**：

```zun
$ config = {"db": {"host": "localhost", "port": 5432}};
$>> config["db"]["host"];
localhost

$>> config["db"]["port"];
5432
```

**字典与列表混用**：

```zun
$ data = {"tags": [1, 2, 3], "name": "test"};
$>> data["tags"][0];
1

$>> data["name"];
test
```

**字典方法**：

```zun
$ user = {"name": "Tom", "age": 18};
$>> user.len();              // 获取键值对数量: 2

$>> user.keys();            // 获取所有键: [name, age]
$>> user.values();         // 获取所有值: [Tom, 18]

$>> user.contains_key("name");    // 检查键是否存在: true
$>> user.contains_key("email");  // 检查键是否存在: false

$>> user.remove("age");   // 移除键值对并返回新字典: {name: Tom}

$>> user.type();           // 获取类型名称: Map
```

---

## 类型注解（可选）

使用 `$ 变量名: 类型 = 值` 语法指定类型：

```zun
$ count: Int = 0;
$ name: String = "Zun";
$ price: Float = 9.9;
$ active: Bool = true;
```

### 支持的类型注解

| 类型注解 | 别名 | 说明 |
|---|---|---|
| `Int` | `Integer` | 整数 |
| `Float` | - | 浮点数 |
| `String` | `Str` | 字符串 |
| `Bool` | `Boolean` | 布尔值 |

---

## 动态模式 vs 静态模式

### 动态模式（默认）

变量声明时不指定类型，类型随赋值自动变化，无任何限制：

```zun
$ x = 30;
x = "hello";   // ✅ 允许，类型从 Int 变为 String
x = true;      // ✅ 允许，类型从 String 变为 Bool
x = 3.14;      // ✅ 允许，类型从 Bool 变为 Float
```

**适用场景**：快速验证想法、原型开发、摆摊阶段。

### 静态模式（显式声明）

变量声明时指定类型注解，后续赋值必须保持类型一致：

```zun
$ x: Int = 30;
x = 100;       // ✅ 允许，同为 Int
x = "hello";   // ❌ 报错，类型不匹配
x = 3.14;      // ❌ 报错，类型不匹配
```

**适用场景**：涉及金额、ID、状态码等关键变量，项目规模变大后。

---

### 常量类型注解

常量 `$@` 同样支持类型注解：

```zun
$@ MAX_RETRY: Int = 3;
$@ API_URL: String = "https://api.zun-lang.dev";
```

类型注解对常量而言是**文档性质**，增强可读性（常量本身已不可修改）。

---

### 两种模式混用

同一个文件里，两种模式可以共存，互不影响：

```zun
$ name = "Zun";   // 动态模式
$ price: Float = 9.9; // 静态模式

name = 123;           // ✅ 动态，允许
price = "free";       // ❌ 静态，报错
```

---

## 变量遮蔽 (Shadowing)

使用 `$` 前缀可以声明同名变量（遮蔽），允许改变类型：

```zun
$ a = 20;           // a = 20 (整数)
$ a = "hello";      // 遮蔽，新 a = "hello" (字符串)
$>> a;              // 输出: hello
```

---

## 类型注解 (静态模式)

Zun 支持两种模式：**动态模式**（默认）和**静态模式**（类型注解）。

### 动态模式（默认）

变量声明时不指定类型，类型随赋值自动变化：

```zun
$ x = 30;
x = "hello";   // ✅ 允许，类型从 Int 变为 String
x = true;      // ✅ 允许，类型从 String 变为 Bool
```

### 静态模式（类型注解）

变量声明时指定类型，后续赋值必须保持类型一致：

```zun
$ x: Int <& 30;
x = 100;       // ✅ 允许，同为 Int
x = "hello";   // ❌ 报错，类型不匹配
```

**支持的类型注解**：

| 类型注解 | 说明 |
|---|---|
| `Int` / `Integer` | 整数 |
| `Float` | 浮点数 |
| `String` / `Str` | 字符串 |
| `Bool` / `Boolean` | 布尔值 |

**类型不匹配错误**：

```zun
$ x: Int <& 30;
x = "hello";
// [ERROR] runtime error: type mismatch: cannot assign hello to variable 'x' of type Int
```

---

## 统一的数据表示

所有数据在内部统一存储为字符串，简化实现：

```zun
$>> 42;        // 输出: 42
$>> "hello";   // 输出: hello
$>> 'a';       // 输出: a
```

---

## 运行时类型获取

使用 `.type()` 方法可以获取值的当前类型信息：

```zun
$>> 42.type();          // 输出: Int
$>> 3.14.type();        // 输出: Float
$>> "hello".type();     // 输出: String
$>> true.type();        // 输出: Bool
$>> [1,2,3].type();    // 输出: List
$>> {"a":1}.type();    // 输出: Map
```

> 注意：对于数值类型，方法调用需要用括号包裹，如 `(42).type()`

---

---

## 自定义类型 `$Type`

使用 `$Type` 定义用户类型与结构形状。当前主线里，这不再只是文档层描述；`$Type` 是 Zun 的名义自定义类型，不是和 `Map` 按形状自动互换的结构标签。

```zun
$Type User {
    id: Int
    name: Str
    email: Str?    // ? 表示可选字段
    age: Int?
}
```

**字段规则**：

| 写法 | 说明 |
|------|------|
| `id: Int` | 必填字段，类型为 Int |
| `email: Str?` | 可选字段，类型为 Str，加 `?` |
| 字段类型 | `Int` `Str` `Bool` `Float`，首字母大写 |

**无 getter / setter**，无私有字段，无方法。

### 当前主线语义

- `User { ... }` 才是 `User`
- 裸 `Map` / `JSON` 即使字段一致，也不是 `User`
- 构造 `User { ... }` 时，runtime 会校验缺失必填字段、未声明字段、字段类型不匹配
- 当函数或 HTTP handler 声明返回 `User` 或 `List<User>` 时，实际值也必须是对应的 typed instance
- `$Type` 字段当前支持 `User`、`User?`、`List<User>`、`List<User>?`
- `T?` 表示字段可以缺失；如果出现，也允许显式 `null`

如果你只是要返回普通 JSON，对外仍然可以写 `-> JSON`。但如果声明返回用户类型，实际值应写成：

```zun
$Type User {
    id: Int
    name: Str
}

$fn get_user() -> User {
    $# User {
        id: 1,
        name: "Alice",
    };
}
```

例如下面这种写法现在会直接报错，因为缺少必填字段 `name`：

```zun
$Type User {
    id: Int
    name: Str
}

$ user = User {
    id: 1,
};
```

也可以写带列表字段和可选字段的类型：

```zun
$Type Feed {
    items: List<Post>
    next_cursor: String?
}
```

当前主线不支持：

```zun
$Type Feed {
    items: List<Post?>
}
```

如果你需要“整个列表可选”，应写成 `List<Post>?`，而不是 `List<Post?>`。

---

### 历史写法提示：`-> JSON<User>`

这一节保留是为了说明旧文档里曾经出现过 `JSON<User>` 这类签名。

当前主线不再把它当作稳定教学写法。请改用：

- `-> JSON` 表示响应格式
- `-> List<T>` 表示集合返回
- `$Type` 用于结构说明，也参与当前主线的用户类型返回校验

如果你在旧材料里看到 `JSON<User>`，应回到 [../guide/09-gradual-typing.md](../guide/09-gradual-typing.md) 采用当前口径。

---

### 推荐项目结构

```
project/
├── main.zun
├── models/          ← $Type 定义集中存放
│   ├── user.zun
│   └── order.zun
├── routers/         ← 薄层：收参数 → 调 service → 返回响应
├── service/         ← 业务逻辑：校验、组装、调 data
└── data/            ← 数据操作：DB 查询、外部 API
```

---

## 方法调用必须使用括号

Zun 中所有方法调用**必须使用括号**：

```zun
$ s = "hello";
$>> s.upper();     // ✅ 正确: HELLO
$>> s.upper;      // ❌ 错误

$ arr = [1, 2, 3];
$>> arr.len();    // ✅ 正确: 3
$>> arr.len;     // ❌ 错误
```

**错误信息**：

```
[ERROR] Parse error at line 1:1
  found: Ident upper
  expected: (
  method 'upper' requires parentheses, use 'upper(...)' instead
```
