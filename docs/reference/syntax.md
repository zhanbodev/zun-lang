# 语法参考

本文档按“能快速查关键写法”的方式整理 Zun 常用语法。

## 语句结束符

所有语句必须以 `;` 结尾：

```zun
$>> "Hello";
```

## 声明与函数

```zun
$ value = 1;
$ value: Int = 1;
$@ NAME = "zun";

$fn greet(name) -> String {
    $# name;
}
```

参数类型注解现在已经进入主线，可用于普通函数、HTTP handler 和匿名函数。

例如：

```zun
$fn add(a: Int, b: Int) -> Int {
    $# a + b;
}
```

当前保守边界：

- variadic 参数 `...rest` 仍不支持类型注解
- `$Type` 参数不接受自动 `Map -> User` 转换

## 输入与输出

```zun
$>> "Hello";
$>>ERR("bad input");
$ name = $<<LINE();
$ home = $<<ENV("HOME");
$>> fs.read_text("notes.txt");
```

文件能力主线推荐走 `std.fs`：

```zun
$mod std.fs;

$ text = fs.read_text("notes.txt");
$ lines = fs.read_lines("notes.txt");
fs.write("output.txt", text);
```

`$<<FILE(...)` / `$>>FILE(...)` 已移除；继续使用会报 `ZUN-P008`，文件读写请改用 `std.fs`。

## 控制流

```zun
$if cond {
} $elif other {
} $else {
}

$while cond {
}

$loop {
    $break;
}

$for item in items {
    $continue;
}
```

## 模块

```zun
$mod helper;
$mod math.*;
$mod std.math;
```

`$mod a.*;` 导入直接子模块命名空间，不会把函数平铺到当前作用域。

## 类型与返回

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
        }
    ];
}
```

返回集合时，主线推荐写 `List<T>`，而不是裸 `List`。

## HTTP

```zun
$GET("/hello") hello() -> String {
    $# "world";
}

$POST("/echo") echo() -> JSON {
    $# $JSON { "body": body };
}

$HTTP("/api").link("routers.api");
```

## 构造器与入口

```zun
$# $JSON { "ok": true };
$# $HTML("<h1>Hello</h1>");
$# $HTML().link("pages.index");
$# $RES(201, {"created": true});

$main() {
    $>> "starting";
}
```
