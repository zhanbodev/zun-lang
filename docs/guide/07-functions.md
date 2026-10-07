# 7. 函数

函数是 Zun 组织逻辑、消除重复、拆模块的主要方式。

## 定义函数

```zun
$fn greet(name) -> String {
    $# f"Hello, {name}";
}
```

调用：

```zun
$>> greet("World");
```

## 返回值和返回类型

使用 `$#` 返回：

```zun
$fn add(a, b) -> Int {
    $# a + b;
}
```

没有显式 `$#` 的函数会返回 `Null`：

```zun
$fn log(msg) {
    $>> msg;
}
```

当前主线可稳定讲解的返回类型包括：

- `Int`
- `Float`
- `String`
- `Bool`
- `List`
- `Map`
- `JSON`
- `HTML`
- `Response`
- `List<T>`，其中 `T` 可以是内建类型或已定义的用户类型

需要注意的是，当前主线只把 `List<T>` 当作可教学的泛型返回写法，不把 `JSON<T>` 当作同级语法来承诺。

## 参数类型注解

当前主线已经支持函数参数类型注解：

```zun
$fn add(a: Int, b: Int) -> Int {
    $# a + b;
}
```

支持的主线写法包括：

- `a: Int`
- `user: User`
- `items: List<Post>`
- `email: String?`

运行时会在参数绑定阶段立刻校验传入值是否满足声明类型。

这里也要记住一条：

- `user: User` 只接受真正的 `User { ... }` typed instance，不接受形状相同的裸 `Map`

当前这一轮仍然保持保守边界：

- variadic 参数 `...rest` 还不支持类型注解

## 可变参数

```zun
$fn sum(...nums) -> Int {
    $ total = 0;
    $for n in nums {
        total += n;
    }
    $# total;
}
```

## 递归

```zun
$fn factorial(n) -> Int {
    $if n <= 1 {
        $# 1;
    } $else {
        $# n * factorial(n - 1);
    }
}
```

## 私有函数

```zun
_$fn normalize(name) -> String {
    $# name.trim().lower();
}

$fn public_name(name) -> String {
    $# normalize(name);
}
```

`_$fn` 只给当前模块内部使用。

## 匿名函数

```zun
$ double = $fn(x) {
    $# x * 2;
};

$>> double(5);
```

## 一个完整例子

```zun
_$fn twice(x) -> Int {
    $# x * 2;
}

$fn compute(n) -> Int {
    $ value = twice(n);
    $# value + 1;
}

$>> compute(10);
```

## 下一章

继续看 [08-collections-and-methods.md](08-collections-and-methods.md)。
