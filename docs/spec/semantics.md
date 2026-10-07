# Semantics Specification

本页记录 Zun 当前 runtime 的关键语义。

## 规则描述

### 作用域

- 顶层变量保存在当前程序环境中
- 函数调用会创建新的局部环境
- 函数参数进入局部环境
- 函数内部对变量的声明不会自动回写到调用方环境
- 函数定义表会在调用后同步回外层，因此函数定义可以在执行过程中扩展

### Truthy / Falsy

当前实现中：

- `false`、`0`、`0.0`、`NaN`、空字符串、空列表、空 map、空 JSON、`Null` 为 falsy
- 其他值为 truthy

### 类型注解

- 变量声明和常量声明支持类型注解：`$ value: Int = 1;`
- 当前支持的声明类型：
  - `Int`
  - `Float`
  - `String` / `Str`
  - `Bool` / `Boolean`
  - `List`
  - `Map`
- 声明时会立即检查右值类型
- 之后再次赋值时，会继续检查是否与声明类型一致

当前实现中，`Json`、`Response` 仍然不是变量声明可写的类型注解关键字。

### 函数与返回

- 函数声明语法：`$fn name(a, b) -> Int { ... }`
- 返回使用 `$#`
- 如果函数声明了返回类型，则返回值会在运行时检查
- 当前实现支持的函数返回类型检查：
  - `Int`
  - `Float`
  - `String`
  - `Bool`
  - `List`
  - `Map`
  - `Json`
  - `Response`
  - 用户自定义类型（如 `User`）
  - `List<T>`，其中 `T` 可以是内建类型或用户自定义类型
- `RuntimeMode::Test` 下，HTTP handler 在注册阶段也会执行同级别的返回类型检查
- 没有 `$#` 的函数返回 `Null`

### 控制流

- `$if / $elif / $else` 使用 truthy/falsy 规则判断条件
- `$while` 先判断条件，再执行循环体
- `$loop` 为无限循环，直到 `break`
- `$for item in iterable` 当前支持遍历：
  - `List`
  - `Map`，迭代值为 key
  - `String`，迭代值为单字符字符串
- `break` 和 `continue` 只能在循环语义中使用
- `exit` 会终止当前程序执行链

## 最小正确示例

```zun
$ value: Int = 2;

$fn describe(input) -> String {
    $if input {
        $# "truthy";
    }
    $# "falsy";
}

$>> describe(value);
```

对应测试：

- `tests/spec/valid/semantics/type_annotation_and_truthiness.zun`

## 最小错误示例

```zun
$ value: Int = "text";
```

对应测试：

- `tests/spec/invalid/semantics/type_annotation_mismatch.zun`

另一个运行时错误样例：

```zun
$>> missing;
```

对应测试：

- `tests/spec/invalid/semantics/undefined_variable.zun`

## 对应测试位置

- `tests/spec/valid/semantics/type_annotation_and_truthiness.zun`
- `tests/spec/invalid/semantics/type_annotation_mismatch.zun`
- `tests/spec/invalid/semantics/undefined_variable.zun`
