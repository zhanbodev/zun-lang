# 6. 控制流

这一章介绍 Zun 中“让程序按条件和次数运行”的方式。只会写表达式还不够，能组织流程才算能写脚本。

## 条件分支

```zun
$ score = 75;
$if score >= 90 {
    $>> "A";
} $elif score >= 80 {
    $>> "B";
} $else {
    $>> "C";
}
```

这个例子说明了三件事：

- 条件写在 `$if` 后面
- 命中分支后执行对应代码块
- 没命中时落到 `$else`

再看一个更贴近脚本的写法：

```zun
$ name = "zun";
$if name == "zun" {
    $>> "matched";
} $else {
    $>> "not matched";
}
```

## while 循环

```zun
$ i = 0;
$while i < 3 {
    $>> i;
    i += 1;
}
```

`$while` 的语义是：

1. 先判断条件
2. 条件为真时执行循环体
3. 循环体结束后回到条件判断

它适合“直到某个条件不成立”为止的任务。

## loop 无限循环

```zun
$ i = 0;
$loop {
    $>> i;
    i += 1;
    $if i >= 3 {
        $break;
    }
}
```

`$loop` 没有条件，本身就是无限循环，因此通常要配合 `$break` 使用。

## for-in 遍历

```zun
$for item in [1, 2, 3] {
    $>> item;
}
```

当前实现支持遍历：

- List
- Map
- String

### 遍历 List

```zun
$ nums = [1, 2, 3];
$for n in nums {
    $>> n;
}
```

### 遍历 Map

```zun
$ user = {"name": "Tom", "age": 25};
$for key in user {
    $>> key;
}
```

遍历 map 时，当前拿到的是 key。

### 遍历 String

```zun
$for ch in "abc" {
    $>> ch;
}
```

遍历字符串时，拿到的是单字符字符串。

## C 风格 for

```zun
$for i = 0; i < 5; i = i + 1 {
    $>> i;
}
```

可以把它理解成：

- 初始化：`i = 0`
- 条件：`i < 5`
- 更新：`i = i + 1`

它适合“已知循环计数方式”的场景。

## break 与 continue

```zun
$for i = 0; i < 5; i = i + 1 {
    $if i == 2 {
        $continue;
    }
    $if i == 4 {
        $break;
    }
    $>> i;
}
```

- `$continue` 跳过本轮后续逻辑，进入下一轮
- `$break` 直接结束整个循环

## exit

`exit` 会终止当前程序执行链：

```zun
$>> "before";
exit;
$>> "after";
```

写脚本时要谨慎使用，因为它会直接结束流程。

## truthy / falsy

条件判断依赖 truthy / falsy 规则。常见 falsy 值包括：

- `false`
- `0`
- `0.0`
- 空字符串
- 空列表
- 空 map
- `Null`

例如：

```zun
$if "" {
    $>> "truthy";
} $else {
    $>> "falsy";
}
```

```zun
$if [] {
    $>> "truthy";
} $else {
    $>> "falsy";
}
```

更正式的边界以 `docs/spec/semantics.md` 为准。

## 一个完整例子

```zun
$ items = [1, 2, 3, 4, 5];
$ total = 0;

$for item in items {
    $if item % 2 == 0 {
        $continue;
    }
    total += item;
}

$>> total;
```

## 下一章

继续看 [07-functions.md](07-functions.md)。
