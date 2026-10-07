# Zun 渐进式类型系统设计文档

> **设计原则：能跑就不用加，怕出错再加。**

---

## 一、为什么引入渐进式类型

### 当前问题

Zun v1.2 的类型系统存在设计矛盾：

- 定位是**动态语言**，但重赋值时强制要求类型一致
- 用户看到解释型语言，默认期待动态类型行为
- `to_str()` 等方法返回值无法直接赋值给原变量（Bug #4）

### 核心矛盾

```zun
$ x = 30;
x = "hello";   // ❌ 现在报错，但用户期待动态语言应该允许
$ x = "hello"; // ✅ 遮蔽才能改类型，语义不够直觉
```

### 解决思路

引入**渐进式类型（Gradual Typing）**：

- **不写类型注解** → 完全动态，变量可以随时改类型
- **写了类型注解** → 静态检查，类型保护生效

这是 TypeScript 对 JavaScript 的成功路径，Zun 复刻它。

---

## 二、概念定义

### 2.1 动态模式（默认）

变量声明时**不指定类型**，类型随赋值自动变化，无任何限制。

```zun
$ x = 30;
x = "hello";   // ✅ 允许，类型从 Int 变为 String
x = true;      // ✅ 允许，类型从 String 变为 Bool
x = 3.14;      // ✅ 允许，类型从 Bool 变为 Float
```

**适用场景**：快速验证想法、原型开发、摆摊阶段。

---

### 2.2 静态模式（显式声明）

变量声明时**指定类型注解**，后续赋值必须保持类型一致。

```zun
$ x: Int = 30;
x = 100;       // ✅ 允许，同为 Int
x = "hello";   // ❌ 报错，类型不匹配
x = 3.14;      // ❌ 报错，类型不匹配
```

**适用场景**：涉及金额、ID、状态码等关键变量，项目规模变大后。

---

### 2.3 变量遮蔽（重新定义）

无论动态模式还是静态模式，使用 `$` 重新声明同名变量，均可改变类型和注解。

```zun
$ x: Int = 30;
$ x = "hello";      // ✅ 遮蔽，新变量，动态模式
$ x: String = "hi"; // ✅ 遮蔽，新变量，重新声明为 String
```

**遮蔽的语义**：我知道我在重新定义这个变量，是故意的。

---

### 2.4 两种模式的混用

同一个文件里，两种模式可以共存，互不影响。

```zun
$ name = "Zun";   // 动态模式
$ price: Float = 9.9; // 静态模式

name = 123;           // ✅ 动态，允许
price = "free";       // ❌ 静态，报错
```

---

## 三、语法规范

### 3.1 类型注解语法

```
$ 变量名: 类型 = 值;
```

**示例**：

```zun
$ count: Int = 0;
$ name: String = "Zun";
$ price: Float = 9.9;
$ active: Bool = true;
```

---

### 3.2 支持的类型注解

| 类型注解 | 别名 | 说明 | 示例 |
|---|---|---|---|
| `Int` | `Integer` | 整数 | `$ x: Int = 30;` |
| `Float` | - | 浮点数 | `$ x: Float = 3.14;` |
| `String` | `Str` | 字符串 | `$ x: String = "hi";` |
| `Bool` | `Boolean` | 布尔值 | `$ x: Bool = true;` |

---

### 3.3 常量类型注解

常量 `$@` 同样支持类型注解，语法一致：

```zun
$@ MAX_RETRY: Int = 3;
$@ API_URL: String = "https://api.zun.co";
```

常量本身已经不可修改，类型注解对常量而言是**文档性质**，增强可读性。

---

### 3.4 函数参数与返回值（已有，保持不变）

```zun
$fn add(a, b) -> Int {
    $# a + b;
}
```

函数参数暂不支持类型注解（后续版本规划），返回值类型注解保持现有语法。

---

### 3.5 动态模式下的重赋值（新行为）

去掉现有的重赋值类型检查，允许动态模式下随意改类型：

```zun
// 之前行为（v1.2）
$ x = 30;
x = "hello";  // ❌ [ERROR] type mismatch

// 新行为（v1.3）
$ x = 30;
x = "hello";  // ✅ 允许，动态模式无限制
```

---

## 四、错误信息规范

### 4.1 静态模式类型不匹配

**触发条件**：对有类型注解的变量赋予不匹配类型的值

**错误格式**：
```
[ERROR] type error: variable 'x' is declared as 'Int', cannot assign 'String' value
  hint: use '$ x: String = ...' to redeclare with a new type
```

**示例**：
```zun
$ price: Float = 9.9;
price = "free";
```
```
[ERROR] type error: variable 'price' is declared as 'Float', cannot assign 'String' value
  hint: use '$ price: String = ...' to redeclare with a new type
```

---

### 4.2 类型注解与初始值不匹配

**触发条件**：声明时类型注解和右侧值的实际类型不一致

**错误格式**：
```
[ERROR] type error: declared type 'Int' does not match value type 'String'
  hint: change the annotation or the value
```

**示例**：
```zun
$ x: Int = "hello";
```
```
[ERROR] type error: declared type 'Int' does not match value type 'String'
  hint: change the annotation or the value
```

---

### 4.3 未知类型注解

**触发条件**：用户写了 Zun 不支持的类型名

**错误格式**：
```
[ERROR] type error: unknown type 'Xxx', supported types are: Int, Float, String, Bool
```

**示例**：
```zun
$ x: Number = 30;
```
```
[ERROR] type error: unknown type 'Number', supported types are: Int, Float, String, Bool
```

---

### 4.4 移除旧错误（Breaking Change）

以下错误信息在 v1.3 中**不再出现**（动态模式放开）：

```
// 废弃，不再报错
[ERROR] runtime error: type mismatch: cannot assign hello to variable 'x' of type Number
```

同时修复 Bug #4，`to_str()` 等方法返回值在动态模式下可以直接赋值：

```zun
$ x = 30;
x = x.to_str();  // ✅ 动态模式，允许
$>> x;           // "30"
```

---

## 五、用户学习路径

### 第一阶段：摆摊，不用想类型

```zun
$ price = 99;
$ qty = 3;
$ total = price * qty;
$>> total;  // 297
```

和 Python 一样简单，零心智负担。

---

### 第二阶段：项目变大，保护关键变量

```zun
$ user_id: Int = 12345;
$ amount: Float = 99.0;
$ status: Int = 200;
```

涉及钱、ID、状态码，加上类型注解，防止静默错误。

---

### 判断标准（一句话写进文档）

> **能跑就不用加，怕出错再加。**

---

## 六、实现优先级

| 优先级 | 内容 | 说明 |
|---|---|---|
| P0 | 放开动态模式重赋值限制 | 修复 Bug #4，去掉旧类型检查 |
| P0 | 解析类型注解语法 `$ x: Int = 30` | 核心新功能 |
| P1 | 静态模式类型检查 | 有注解时才检查 |
| P1 | 错误信息更新 | 按本文档规范输出 |
| P2 | 常量类型注解支持 | 文档性质，低风险 |
| P3 | 函数参数类型注解 | 后续版本 |

---

## 七、版本说明

| 版本 | 内容 |
|---|---|
| v1.2（当前） | 半静态类型，重赋值强制类型一致 |
| **v1.3（本文档）** | **渐进式类型，动态默认，注解可选** |

---

*Zun Team · Gradual Typing Spec v1.0 · 2026-03-01*
