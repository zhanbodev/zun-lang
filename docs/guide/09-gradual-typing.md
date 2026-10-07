# 9. 渐进类型

Zun 当前采用“默认动态，显式约束”的使用方式。

## 默认动态

不写类型注解时，变量可以改变值类型：

```zun
$ x = 30;
x = "hello";
x = true;
```

## 显式类型注解

```zun
$ count: Int = 0;
$ name: String = "Alice";
$ active: Bool = true;
$ items: List = [1, 2, 3];
$ config: Map = {"name": "demo"};
```

常量也支持类型注解：

```zun
$@ MAX: Int = 100;
```

当前主线可依赖的声明类型名：

- `Int`
- `Float`
- `String`
- `Str`
- `Bool`
- `Boolean`
- `List`
- `Map`

当前不要把 `Json`、`Response` 写成“变量/常量声明可稳定使用”的类型名。

## 函数参数类型注解的态度

当前主线已经支持函数参数类型注解。

例如：

```zun
$fn add(a: Int, b: Int) -> Int {
    $# a + b;
}
```

当前主线可依赖的参数写法包括：

- `a: Int`
- `user: User`
- `items: List<Post>`
- `email: String?`

运行时会在参数绑定阶段立刻校验传入值。

这里保持两条边界：

- `user: User` 只接受真正的 `User { ... }` typed instance，不接受裸 `Map`
- variadic 参数 `...rest` 当前仍不支持类型注解

## 返回类型和 `List<T>`

返回类型是这一章最重要的稳定能力之一。

### 正确写法

```zun
$Type User {
    id: Int
    name: Str
}

$fn list_users() -> List<User> {
    $# [
        User {
            id: 1,
            name: "Alice",
        },
        User {
            id: 2,
            name: "Bob",
        }
    ];
}
```

这里要记住两条：

- `List<T>` 是当前主线可用写法
- 裸 `List` 不应再作为返回类型写法来教学

`T` 可以是内建类型，也可以是先定义好的用户类型。

实现说明：

- 当前主线里，函数返回类型、HTTP 返回类型、`$Type` 字段类型、变量类型注解，内部都已经统一到同一套 `TypeExpr` 模型
- 这次统一主要是解释器内部表示层收口，不代表语言层面额外开放了新的返回类型语义
- 也就是说，用户应继续按当前文档口径使用 `-> T` 和 `-> List<T>`，而不是假定所有 `TypeExpr` 形态都已经成为稳定承诺

## `$Type`

`$Type` 定义的是 Zun 的名义自定义类型，不等于 class，也不是“形状一样就能互换”的 `Map`：

```zun
$Type User {
    id: Int
    name: Str
    email: Str?
}
```

它最适合和 `List<T>`、接口返回说明、阅读文档时的结构表达配合使用。

这里要记住两条：

- `User { ... }` 才是 `User`
- 裸 `Map` / `JSON` 即使字段完全一致，也不是 `User`

如果函数或 HTTP handler 声明返回 `User` 或 `List<User>`，实际返回值也应该是 `User { ... }` 实例，而不是形状相同的裸 `Map`。

当前主线里，`$Type` 构造还会在 runtime 校验：

- 未声明字段
- 缺失必填字段
- 字段值类型不匹配

这意味着如果 `name` 是必填字段，`User { id: 1 }` 会直接报错，而不是构造出一个缺字段实例。

阶段 2 之后，`$Type` 字段还支持：

```zun
$Type Feed {
    items: List<Post>
    next_cursor: String?
}
```

这里的语义是：

- `items` 必须是 `List<Post>`
- `next_cursor` 可以缺失
- `next_cursor` 也可以显式为 `null`

当前主线支持：

- `User`
- `User?`
- `List<User>`
- `List<User>?`

当前主线不支持：

- `List<User?>`

## HTTP 返回里的 `List<T>`

在 HTTP 章节里同样推荐这样写：

```zun
$Type User {
    id: Int
    name: Str
}

$GET("/users") list_users() -> List<User> {
    $# [
        User {
            id: 1,
            name: "Alice",
        },
        User {
            id: 2,
            name: "Bob",
        }
    ];
}
```

如果类型或返回校验失败，它会表现为用户可见错误；如何排查请继续看 [10-error-handling.md](10-error-handling.md)。

## 运行时类型查询

```zun
$>> 42.type();
$>> "hello".type();
$>> [1, 2].type();
$>> {"a": 1}.type();
```

## `std.core.check`

```zun
$mod std.core.check;

$>> check.type_of(42);
check.assert(1 + 1 == 2, "math broken");
```

## 下一章

继续看 [10-error-handling.md](10-error-handling.md)。
