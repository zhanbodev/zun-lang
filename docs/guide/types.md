# 类型注解

> **Status: Migration Source**
>
> 这是一页迁移期旧类型文档。
> 当前主线请改读 [09-gradual-typing.md](09-gradual-typing.md)。
> 本页仍保留的独特价值是旧版示例集合，但其中参数类型注解、`JSON<User>` 等写法不应视为当前主线承诺。

Zun 支持**渐进类型注解**：注解完全可选，不影响程序运行，但能提升代码可读性。

---

## 变量类型注解

在变量名后加 `: 类型` 进行注解：

```zun
$ count: Int = 0;
$ name: String = "Alice";
$ ratio: Float = 0.5;
$ active: Bool = true;
$ items: List = [1, 2, 3];
$ config: Map = {"key": "value"};
```

不带注解的写法同样有效：

```zun
$ count = 0;
$ name = "Alice";
```

---

## 函数返回类型注解

在参数列表后加 `-> 类型`：

```zun
$fn add(a, b) -> Int {
    $# a + b;
}

$fn greet(name) -> String {
    $# f"Hello, {name}!";
}

$fn is_even(n) -> Bool {
    $# n % 2 == 0;
}
```

没有返回值的函数可以省略注解：

```zun
$fn log(msg) {
    $>> msg;
}
```

---

## 支持的类型名

| 类型名 | 说明 |
|--------|------|
| `Int` | 整数 |
| `Float` | 浮点数 |
| `String` / `Str` | 字符串 |
| `Bool` | 布尔值 |
| `List` | 列表 |
| `Map` | 字典 |

---

## 自定义类型 `$Type`

使用 `$Type` 定义用户类型与结构形状：

```zun
$Type User {
    id: Int
    name: Str
    email: Str?    // ? 表示可选字段
    age: Int?
}
```

- 字段类型：`Int` `Str` `Bool` `Float`，首字母大写
- 可选字段：在类型后加 `?`
- 无 getter / setter，无私有字段，无方法

当前主线里，`$Type` 已经不只是“文档标签”。如果函数或 HTTP handler 声明返回 `User` 或 `List<User>`，runtime 会校验实际返回值是否为对应的 `User { ... }` 实例。

### 与返回类型相关的迁移提醒

旧文档里曾经把 `$Type` 和 `JSON<User>` 放在一起讲解。

这一轮主线已经不再把 `JSON<User>` 当作稳定教学写法，所以这里不要继续照抄旧签名。当前阅读方式是：

- 结构说明看 `$Type`
- 普通 JSON 返回看 `-> JSON`
- 集合返回看 `-> List<T>`
- 如果声明返回 `T` 或 `List<T>`，实际值也应构造成 `T { ... }`

主线说明以 [09-gradual-typing.md](09-gradual-typing.md) 为准。

---

## 运行时类型查询

通过 `.type()` 方法在运行时获取值的类型名：

```zun
$>> 42.type();          // Int
$>> 3.14.type();        // Float
$>> "hello".type();     // String
$>> true.type();        // Bool
$>> [1, 2].type();      // List
$>> {"a": 1}.type();    // Map
$>> null.type();        // Null
```

配合条件判断：

```zun
$fn describe(val) -> String {
    $ t = val.type();
    $if t == "Int" {
        $# "这是整数";
    } $else $if t == "String" {
        $# "这是字符串";
    } $else {
        $# f"类型: {t}";
    }
}

$>> describe(42);        // 这是整数
$>> describe("hello");   // 这是字符串
```

---

## 常量

使用 `$@` 声明不可修改的常量：

```zun
$@ MAX_SIZE = 100;
$@ PI = 3.14159;
$@ APP_NAME = "Zun";
```

常量支持类型注解：

```zun
$@ MAX_SIZE: Int = 100;
$@ PI: Float = 3.14159;
```

尝试修改常量会报运行时错误：

```zun
$@ X = 10;
X = 20;    // 错误：不能修改常量
```

---

## `std.core.check` 类型工具

```zun
$mod std.core.check;

check.type_of(42);        // "Int"
check.type_of("hello");   // "String"
check.is_null(null);      // true
check.is_null(0);         // false

check.assert(1 + 1 == 2, "数学出错了");    // 通过
check.assert(false, "断言失败");           // 抛出异常
```
