# 模块系统

> **Status: Deprecated**
>
> 这是一页已废弃的旧版模块文档。
> 当前主线请改读 [11-modules.md](11-modules.md) 和 [../spec/modules.md](../spec/modules.md)。
> 本页保留仅用于迁移追溯；其中关于通配符导入会把函数平铺到当前作用域的说法已经失效，不能再作为当前语义依据。

Zun 的模块系统分为三层：**native 内置模块**、**纯 .zun 标准库模块**、**项目自定义模块**。

---

## 导入模块

### 导入标准库模块

```zun
$mod std.math;
$mod std.fs;
$mod std.json;
```

导入后，模块以最后一级名称作为命名空间前缀使用：

```zun
$mod std.math;
$>> math.sqrt(9.0);    // 3.0

$mod std.fs;
$ ok = fs.exists("./data.txt");
```

### 导入子模块

```zun
$mod std.math.stats;   // 对应 stdlib/math/stats.zun
$mod std.str.fmt;      // 对应 stdlib/str/fmt.zun
$mod std.core.iter;    // 对应 stdlib/core/iter.zun
```

```zun
$mod std.core.iter;
$for i in iter.range(0, 5) {
    $>> i;             // 0 1 2 3 4
}
```

---

## 自定义模块

在项目目录中创建 `.zun` 文件，即可作为模块使用。

### 项目结构

```
my-app/
├── package.toml
├── main.zun
└── shared/
    └── helper.zun
```

### `shared/helper.zun`

```zun
$fn greet(name) -> String {
    $# f"Hello, {name}!";
}

// 私有函数，不可被外部调用
_$fn internal_helper(x) {
    $# x * 2;
}
```

### `main.zun`

```zun
$mod shared.helper;

$>> helper.greet("Zun");    // Hello, Zun!
```

---

## 公有函数与私有函数

| 声明方式 | 可见性 | 说明 |
|----------|--------|------|
| `$fn name(...) { }` | 公有 | 可被外部模块调用 |
| `_$fn name(...) { }` | 私有 | 仅限当前文件内部使用 |

```zun
// utils.zun
$fn public_api(x) {
    $# _transform(x) * 2;
}

_$fn _transform(x) {    // 私有，外部不可访问
    $# x + 1;
}
```

---

## 模块解析顺序

当你写 `$mod foo.bar;` 时，解释器按以下顺序查找：

1. **Native 注册表** — Rust 实现的内置模块（如 `std.math`、`std.fs`）
2. **`stdlib/` 目录** — 纯 .zun 实现的标准库（如 `std.core.iter` → `stdlib/core/iter.zun`）
3. **项目本地文件** — 当前项目中的 `.zun` 文件（如 `shared.helper` → `shared/helper.zun`）

`std` 前缀由解释器特殊处理，会映射到标准库目录。

---

## 项目配置文件

在项目根目录创建 `package.toml` 可以配置项目元信息、环境变量和服务器参数：

```toml
name = "my-app"
version = "0.1.0"
entry = "main.zun"

[env]
APP_NAME = "My Zun App"
DB_HOST = "localhost"

[server]
port = 3000
host = "0.0.0.0"
```

通过 `$<<CONFIG("KEY")` 读取 `[env]` 中的值：

```zun
$ app_name = $<<CONFIG("APP_NAME");
$>> app_name;    // My Zun App
```

运行项目（自动加载 `package.toml`）：

```bash
zun run .
zun serve .
```

---

## 通配符导入

以下示例属于旧写法说明，勿作为当前主线语义：

```zun
$mod std.core.iter.*;

$for i in range(0, 3) {    // 旧写法示例
    $>> i;
}
```

当前实现中，`$mod a.*;` 导入的是直接子模块命名空间，不会把函数平铺到当前作用域。请以 [11-modules.md](11-modules.md) 为准。
