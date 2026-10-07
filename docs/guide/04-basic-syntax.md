# 4. 基本语法

这一章不是“扫一眼语法名词”，而是先把 Zun 程序最常见的组成方式讲清楚。读完这一章，你应该已经能看懂大部分简单脚本。

## 语句与分号

Zun 的普通语句通常以 `;` 结束：

```zun
$ value = 1;
$>> value;
```

块语句使用 `{ ... }` 组织。

```zun
$if true {
    $>> "in block";
}
```

可以先记住一个简单规则：

- 普通语句通常以 `;` 结束
- 带代码块的语句用 `{ ... }`
- 函数、条件、循环、HTTP 路由都会用到代码块

## 注释

单行注释：

```zun
// comment
$ value = 1;
```

多行注释：

```zun
/* multi
line */
$>> "ok";
```

注释不会参与执行，适合：

- 解释一段逻辑
- 临时屏蔽一小段代码
- 给示例加说明

## 标识符与 `$` 前缀关键字

Zun 中很多语言结构都带 `$` 前缀：

```zun
$ value = 1;
$@ NAME = "zun";
$fn greet(name) -> String {
    $# f"hi {name}";
}
```

其中：

- `$` 用来声明变量
- `$@` 用来声明常量
- `$fn` 用来声明函数
- `$#` 用来返回值

不要把它们当作普通变量名的一部分，它们本身就是语法关键字。

## 常见字面量

```zun
$ n = 42;
$ f = 3.14;
$ s = "hello";
$ c = 'a';
$ b = true;
$ empty = null;
```

### 数字

整数和浮点数都可以直接写：

```zun
$ a = 42;
$ b = -7;
$ c = 3.14;
```

### 字符串

字符串使用双引号：

```zun
$ name = "Zun";
$>> "hello";
```

支持常见转义：

```zun
$>> "line1\nline2";
$>> "tab\tvalue";
```

### 字符

字符使用单引号：

```zun
$ ch = 'a';
$>> ch;
```

### 布尔与 null

```zun
$ ok = true;
$ failed = false;
$ empty = null;
```

## f-string

```zun
$ name = "Zun";
$>> f"Hello, {name}";
```

`f"..."` 允许把表达式插入字符串中：

```zun
$ score = 100;
$>> f"user={name}, score={score}";
$>> f"1 + 2 = {1 + 2}";
```

这在打印调试信息时尤其常用。

## 列表与 map 字面量

### 列表

```zun
$ nums = [1, 2, 3];
$ names = ["tom", "jerry"];
$ mixed = [1, "two", true];
```

### map

```zun
$ user = {"name": "Tom", "age": 25};
$ settings = {"debug": true, "port": 8080};
```

map 的 key 当前最常见写法是字符串 key。

## 表达式

当前常见表达式包括：

- 数字、字符串、布尔、字符、`null`
- 列表字面量
- map 字面量
- 变量读取
- 函数调用
- 方法调用
- 索引访问
- 一元与二元运算

下面把这些表达式逐个看一遍。

### 1. 变量读取

```zun
$ value = 10;
$>> value;
```

### 2. 函数调用

```zun
$fn add(a, b) -> Int {
    $# a + b;
}

$>> add(1, 2);
```

### 3. 方法调用

```zun
$ text = " hello ";
$>> text.trim();
$>> text.type();
```

### 4. 索引访问

```zun
$ items = ["a", "b", "c"];
$>> items[0];

$ user = {"name": "Tom"};
$>> user["name"];
```

### 5. 一元运算

```zun
$>> -3;
$>> !false;
```

### 6. 二元运算

```zun
$>> 1 + 2;
$>> 5 - 3;
$>> 4 * 2;
$>> 7 / 2;
$>> 7 % 3;
$>> 1 < 2;
$>> 1 == 1;
$>> true && false;
$>> true || false;
```

## 一个综合例子

```zun
$ user = {"name": "Zun", "scores": [90, 95, 100]};
$ first = user["scores"][0];
$ name = user["name"];
$>> f"{name} first={first}";
```

这个例子里同时出现了：

- map 字面量
- 列表字面量
- 索引访问
- 变量读取
- f-string

## 最小优先级直觉

```zun
$>> 1 + 2 * 3;
```

结果是 `7`，不是 `9`，因为乘除优先于加减。

再看几个例子：

```zun
$>> (1 + 2) * 3;
$>> !false && true;
$>> [1, 2, 3].len();
```

可以先记住这几个最重要的规则：

- `()` 可以强制改变计算顺序
- `* / %` 优先于 `+ -`
- 比较运算在算术运算之后
- 方法调用与索引访问会紧跟在主表达式后面继续解析

## 迁移期参考

- `docs/reference/syntax.md`
- `docs/spec/lexical.md`
- `docs/spec/syntax.md`

## 下一章

继续看 [05-values-and-variables.md](05-values-and-variables.md)。
