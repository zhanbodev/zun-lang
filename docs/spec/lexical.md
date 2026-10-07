# Lexical Specification

本页记录 Zun 当前 lexer 的词法行为。

## 规则描述

### Token 家族

当前实现支持的主要 token 家族包括：

- 标点与分隔符：`; , : . ... ( ) [ ] { } ?`
- 算术运算符：`+ - * / %`
- 复合赋值：`+= -= *= /= %=`
- 比较运算符：`== != > >= < <=`
- 逻辑运算符：`&& || !`
- 赋值与箭头：`=`、`->`
- 标识符与字面量：`IDENT`、`NUMBER`、`STRING`、`FSTRING`、`CHAR`、`BOOL`
- 以 `$` 开头的关键字和内建语法：
  - `$`、`$@`
  - `$if`、`$elif`、`$else`
  - `$while`、`$loop`、`$for`、`$continue`、`$break`
  - `$fn`、`$#`
  - `$mod`、`$main`、`$Type`
  - `$GET`、`$POST`、`$PUT`、`$DEL`、`$PATCH`、`$HTTP`
  - `$JSON`、`$HTML`、`$RES`、`$STATIC`
  - `$<<`、`$<<CONFIG`、`$HDR`
  - `$>>`

### 空白与注释

- 空白字符会被跳过
- 单行注释以 `//` 开始，到行尾结束
- 多行注释以 `/*` 开始，以 `*/` 结束
- 未闭合的多行注释会报词法错误 `ZUN-L002`

### 字符串

- 普通字符串使用双引号：`"text"`
- 支持转义：`\n`、`\t`、`\r`、`\0`、`\\`、`\"`
- 未闭合字符串会报 `ZUN-L003`
- 非法转义会报 `ZUN-L004`

### f-string

- 语法形式：`f"hello {name}"`
- `{}` 内允许写表达式
- 支持转义：`\n`、`\t`、`\r`、`\0`、`\\`、`\"`、`\{`、`\}`
- 右花括号多余或左花括号未闭合会报 `ZUN-L004`
- 未闭合 f-string 会报 `ZUN-L003`

### 字符

- 字符字面量使用单引号：`'a'`
- 当前实现不支持字符转义
- `Char` 在运行时内部按字符串值处理

### 数字

- 支持整数和浮点数
- `-` 可以作为数字字面量的一部分
- 浮点数要求小数点后至少有一位数字；否则 `.` 会被单独当作 token

## 最小正确示例

```zun
// lexical valid sample
$ value = "Zun";
/* comment */
$>> f"hello {value}";
```

对应测试：

- `tests/spec/valid/lexical/comments_and_fstrings.zun`

## 最小错误示例

```zun
$ value = "oops;
```

对应测试：

- `tests/spec/invalid/lexical/unterminated_string.zun`

## 对应测试位置

- `tests/spec/valid/lexical/comments_and_fstrings.zun`
- `tests/spec/invalid/lexical/unterminated_string.zun`
