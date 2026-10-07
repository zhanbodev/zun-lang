# Syntax Specification

本页记录 Zun 当前 parser 的语法结构和表达式优先级。

## 规则描述

### 语句

当前实现支持的主要语句包括：

- 变量声明：`$ name = expr;`
- 常量声明：`$@ name = expr;`
- 赋值：`name = expr;`
- 复合赋值：`name += expr;`
- 表达式语句：`fn_call();`
- 函数声明：`$fn add(a, b) -> Int { ... }`
- 条件分支：`$if / $elif / $else`
- 循环：`$while`、`$loop`、`$for`
- 返回：`$# expr;`
- `break` / `continue`
- 模块声明：`$mod package.path;`
- 主入口：`$main() { ... }`
- 类型声明：`$Type Name { field: Type, field2: Type? }`
- HTTP 相关语句：`$HTTP`、`$GET`、`$POST` 等

语句一般以 `;` 结束；块语句使用 `{ ... }`。

### 表达式

当前实现支持的主要表达式包括：

- 数字、布尔、字符串、字符、f-string
- 变量读取
- 函数调用：`add(1, 2)`
- 方法调用：`value.type()`
- 列表字面量：`[1, 2, 3]`
- map 字面量：`{"name": "zun"}`
- 索引访问：`items[0]`
- 一元运算：`-x`、`!x`
- 二元运算：算术、比较、逻辑
- 内建构造表达式：`$JSON(...)`、`$HTML(...)`、`$RES(...)`

### 表达式优先级

当前 parser 的优先级从低到高为：

1. `||`
2. `&&`
3. `==`、`!=`
4. `>`、`>=`、`<`、`<=`
5. `+`、`-`
6. `*`、`/`、`%`
7. 一元 `!`、一元 `-`
8. 主表达式后的后缀访问
   - `expr[index]`
   - `expr.method(...)`

### 结合性

- 二元运算按左结合解析
- 一元运算按右递归方式解析
- 方法调用和索引访问在主表达式之后连续解析

## 最小正确示例

```zun
$>> 1 + 2 * 3;
```

对应测试：

- `tests/spec/valid/syntax/expression_precedence.zun`

## 最小错误示例

```zun
$ value = ;
```

对应测试：

- `tests/spec/invalid/syntax/missing_rhs.zun`

## 对应测试位置

- `tests/spec/valid/syntax/expression_precedence.zun`
- `tests/spec/invalid/syntax/missing_rhs.zun`
