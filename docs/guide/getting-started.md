# 快速开始

> **Status: Supplemental**
>
> 这是一页迁移期补充材料。
> 如果你要走当前主线，请改读 [02-installation-and-cli.md](02-installation-and-cli.md) 和 [03-first-program.md](03-first-program.md)。
> 本页仍保留的独特价值是“一页式上手”节奏，适合快速预览，但不再承担完整教学职责。

本指南帮助你在 5 分钟内上手 Zun。

---

## 安装

```bash
# 克隆项目
git clone https://github.com/zhanbodev/zun-lang.git
cd zun-lang

# 编译
cargo build --release

# 将二进制加入 PATH（可选）
export PATH="$PATH:$(pwd)/target/release"
```

---

## 运行你的第一个程序

创建文件 `hello.zun`：

```zun
$>> "Hello, Zun!";
```

运行：

```bash
zun run hello.zun
# 输出: Hello, Zun!
```

---

## 使用 REPL

REPL 是交互式解释器，适合快速验证代码：

```bash
zun repl
```

```
Zun REPL v0.1.0
>> $>> "Hello!";
Hello!
>> $ x = 1 + 2;
>> $>> x;
3
>> exit
```

**REPL 快捷键**

| 快捷键 | 功能 |
|--------|------|
| `↑` / `↓` | 历史记录 |
| `Ctrl+A` | 跳到行首 |
| `Ctrl+E` | 跳到行末 |
| `Ctrl+C` | 取消当前输入 |
| `Ctrl+D` | 退出（输入为空时） |

输入 `exit` 或 `quit` 也可退出。

---

## 基本语法速览

### 变量与常量

```zun
$ name = "Zun";       // 变量（可修改）
$@ PI = 3.14159;         // 常量（不可修改）
```

### 输出

```zun
$>> "Hello, World!";
$>> 42;
$>> name;
```

### 注释

```zun
// 这是单行注释
```

### 函数

```zun
$fn add(a, b) {
    $# a + b;            // $# 是 return
}

$>> add(3, 4);           // 7
```

### 条件

```zun
$ score = 85;
$if score >= 90 {
    $>> "A";
} $else $if score >= 80 {
    $>> "B";
} $else {
    $>> "C";
}
```

### 循环

```zun
// for-in 遍历列表
$for item in [1, 2, 3] {
    $>> item;
}

// while 循环
$ i = 0;
$while i < 3 {
    $>> i;
    i += 1;
}
```

---

## 下一步

- [18-patterns-and-recipes.md](18-patterns-and-recipes.md) — 现行可运行配方
- [../reference/stdlib-api.md](../reference/stdlib-api.md) — 标准库与值方法查表
- [11-modules.md](11-modules.md) — 当前模块系统主线
- [15-http-basics.md](15-http-basics.md) / [16-http-organization.md](16-http-organization.md) — 当前 HTTP 主线
