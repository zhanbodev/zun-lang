# 8. 集合与常用方法

这一章集中讲三类最常用的值：字符串、列表、Map。重点是“创建、读取、修改、查询”和“什么时候该去 `std.*`”。

## 字符串

```zun
$ text = "  hello,world  ";
$>> text.trim();
$>> text.upper();
$>> text.split(",");
$>> text.contains("hello");
```

`split()` 返回 `List`，所以可以继续接列表处理：

```zun
$ parts = "a,b,c".split(",");
$>> parts.len();
$>> parts.join("-");
```

## 列表

```zun
$ nums = [3, 1, 4, 1, 5];
$>> nums.len();
$>> nums.contains(4);
$>> nums.index_of(4);
$>> nums.count(1);
$>> nums.slice(1, 4);
```

常用原地修改方法：

```zun
nums.push(9);
nums.sort();
nums.sort_desc();
nums.reverse();
```

常见读法：

```zun
$>> nums.first();
$>> nums.last();
$>> nums.unique();
$>> [[1, 2], [3, 4]].flatten();
```

### 索引越界

当前实现里，列表索引越界会报运行时错误，不会返回 `null`。

例如下面这种写法不应作为主线常规用法：

```zun
$ items = [1, 2, 3];
$>> items[10];
```

更安全的写法是先看长度：

```zun
$if nums.len() > 2 {
    $>> nums[2];
}
```

## Map

```zun
$ user = {"name": "Tom", "age": 25};
$>> user["name"];
$>> user.len();
$>> user.keys();
$>> user.values();
$>> user.contains_key("age");
```

### Map 写入

当前主线写法直接使用索引赋值：

```zun
$ user = {"name": "Tom"};
user["email"] = "tom@example.com";
user["age"] = 25;
$>> user["email"];
```

删除键：

```zun
user.remove("age");
```

### 什么时候改用 `std.json`

如果你操作的是通用 JSON 结构，或者想做“按路径更新、合并、格式化”，优先用 `std.json`：

```zun
$mod std.json;

$ obj = {"name": "Tom"};
$ obj = json.set(obj, "email", "tom@example.com");
$>> json.stringify(obj);
```

可以把两者这样区分：

- `Map` 原生操作：当前变量上的直接读写
- `std.json`：更偏通用 JSON 工具和转换函数

## 值方法和标准库的区别

- 值方法：`nums.push(1)`、`user.keys()`、`"a,b".split(",")`
- 标准库：`str.trim(text)`、`json.set(obj, key, value)`、`math.sqrt(9.0)`

如果只是处理当前值本身，先选值方法；如果要做通用转换或组合工具，再去 `std.*`。

## 一个完整例子

```zun
$ raw = "alice,bob,bob,carol";
$ names = raw.split(",");
names.sort_desc();

$ stats = {"count": names.len()};
stats["unique_count"] = names.unique().len();

$>> names;
$>> stats["count"];
$>> stats["unique_count"];
```

## 继续查表

- 值方法与模块索引：[13-stdlib-overview.md](13-stdlib-overview.md)
- 完整 API：[../reference/stdlib-api.md](../reference/stdlib-api.md)

## 下一章

继续看 [09-gradual-typing.md](09-gradual-typing.md)。
