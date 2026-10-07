# 11. 模块系统

Zun 通过 `$mod` 组织可复用代码。脚本一旦开始变大，就应该进入模块拆分。

## 导入单个模块

```zun
$mod shared.helper;
$>> helper.answer();
```

模块路径使用点号分隔，对应 `.zun` 文件路径。

## 命名空间访问

导入后，通过文件名命名空间访问：

```zun
$mod helper;
$>> helper.answer();
```

重点是：**不会把函数直接铺平到当前作用域。**

## 通配符导入：`$mod math.*;`

当前实现支持：

```zun
$mod math.*;
```

它的含义是：

- 导入 `math/` 目录下的直接子模块
- 把这些子模块注册成命名空间
- 不会把子模块函数直接平铺到当前作用域

例如：

```text
math/
├── add.zun
└── calc.zun
```

```zun
$mod math.*;
$>> add.answer();
$>> calc.square(3);
```

不要把它理解成下面这种旧预期：

```zun
$mod math.*;
$>> answer();      // 旧预期，当前主线不这样讲
```

## 公有函数与私有函数

```zun
$fn public_api() -> Int {
    $# 1;
}

_$fn hidden() -> Int {
    $# 2;
}
```

规则：

- `$fn` 默认公开
- `_$fn` 默认私有
- `$mod` 只暴露公开函数

## `std.*` 命名空间

标准库模块也通过 `$mod` 导入：

```zun
$mod std.math;
$mod std.str;

$>> math.sqrt(16.0);
$>> str.trim("  hello  ");
```

`std.*` 是保留命名空间，只走标准库解析，不和普通项目模块混用。

## 当前模块解析顺序

普通模块的主顺序：

1. 当前文件所在目录
2. 当前项目根目录
3. 项目根目录下的 `modules/`
4. 如果声明了依赖，再查 `deps/`

`std.*` 则只走标准库解析。

## 一个完整例子

目录：

```text
demo/
├── main.zun
└── shared/
    └── helper.zun
```

`shared/helper.zun`：

```zun
_$fn twice(x) -> Int {
    $# x * 2;
}

$fn answer() -> Int {
    $# twice(21);
}
```

`main.zun`：

```zun
$mod shared.helper;
$>> helper.answer();
```

## 继续查边界

- 主线废弃说明：[modules.md](modules.md)
- 行为边界：[../spec/modules.md](../spec/modules.md)

## 下一章

继续看 [12-projects-and-package.md](12-projects-and-package.md)。
