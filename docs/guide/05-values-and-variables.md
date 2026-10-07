# 5. 值、变量与常量

这一章把“怎么存值、改值、打印值、读入值”讲完整。写 Zun 脚本时，这些操作会出现得最频繁。

## 变量

变量使用 `$` 声明：

```zun
$ name = "Zun";
$ answer = 42;
```

一个变量名绑定一个值：

```zun
$ price = 19.9;
$ enabled = true;
```

变量可以重新赋值：

```zun
$ counter = 1;
counter = counter + 1;
counter += 1;
```

也就是说，你可以先声明，再更新：

```zun
$ total = 0;
total = total + 10;
total += 5;
$>> total;
```

## 常量

常量使用 `$@` 声明：

```zun
$@ APP_NAME = "Zun";
$@ MAX = 10;
```

常量声明后不能再修改。

常量适合表达这些内容：

- 不应该被改写的固定值
- 应用名称
- 上限值
- 约定字符串

```zun
$@ APP_NAME = "Zun";
$@ MAX_RETRY = 3;
```

## 变量和常量怎么选

如果一个值会变化，用变量：

```zun
$ counter = 0;
counter += 1;
```

如果一个值不应该变化，用常量：

```zun
$@ API_VERSION = "v1";
```

## 输出

标准输出：

```zun
$>> "hello";
$>> 42;
```

你可以输出任意表达式：

```zun
$ name = "Zun";
$>> name;
$>> 1 + 2 * 3;
$>> f"hello, {name}";
```

标准错误输出：

```zun
$>>ERR("error");
```

它适合打印错误和调试信息：

```zun
$ path = "missing.txt";
$>>ERR(f"cannot open: {path}");
```

## 读入

读取一行输入：

```zun
$ name = $<<LINE();
```

带提示读取：

```zun
$ name = $<<LINE("name: ");
```

你可以把读到的内容继续赋值、打印、传给函数：

```zun
$ name = $<<LINE("your name: ");
$>> f"hello, {name}";
```

这一类读入默认都是字符串输入，后续如果需要数字，可以再做转换：

```zun
$ raw = $<<LINE("age: ");
$ age = raw.to_int();
$>> age + 1;
```

## 变量遮蔽

同名变量可以再次用 `$` 重新声明：

```zun
$ value = 1;
$>> value;

$ value = "hello";
$>> value;
```

这表示你重新定义了一个同名变量，而不是对原值做普通赋值。

## 当前的值类型直觉

从用户视角，当前最常见的值类型包括：

- Int
- Float
- String
- Bool
- List
- Map
- Null

可以先有一个最基础的印象：

```zun
$ i = 1;
$ f = 3.14;
$ s = "hello";
$ ok = true;
$ items = [1, 2, 3];
$ user = {"name": "Tom"};
$ none = null;
```

后续第 8 章会专门讲集合和方法，第 9 章会专门讲类型注解。

## 一个完整例子

```zun
$ name = $<<LINE("name: ");
$@ APP = "demo";
$ count = 1;

$>> f"{APP}: hello, {name}";
$>> count;
count += 1;
$>> count;
```

## 迁移期参考

- [examples.md](examples.md)
- [types.md](types.md)

## 下一章

继续看 [06-control-flow.md](06-control-flow.md)。
