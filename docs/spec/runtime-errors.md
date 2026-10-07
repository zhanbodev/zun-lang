# Runtime Errors Specification

本页记录 Zun 当前错误类别和错误码。

## 规则描述

### 错误类别

当前实现中的用户可见错误主要分为：

- 词法错误
- 语法错误
- 运行时错误
- 项目装载错误

### 当前错误码

#### 词法错误

- `ZUN-L001`
  - 非法字符或不完整符号
- `ZUN-L002`
  - 多行注释未闭合
- `ZUN-L003`
  - 字符串或 f-string 未闭合
- `ZUN-L004`
  - 非法转义、字符字面量错误、f-string 结构错误

#### 语法错误

- `ZUN-P001`
  - 通用 parse 错误
- `ZUN-P002`
  - 缺失预期 token 或非法 statement

#### 运行时错误

- `ZUN-R001`
  - 通用运行时错误
- `ZUN-R002`
  - 除零
- `ZUN-R003`
  - 模零
- `ZUN-R004`
  - 未定义变量
- `ZUN-R005`
  - 模块装载失败
- `ZUN-R006`
  - 类型不匹配
- `ZUN-R007`
  - 非法表达式
- `ZUN-R008`
  - 非法赋值

#### 项目装载错误

- `ZUN-C001`
  - 读取脚本、解析入口、装载项目时失败

### 诊断格式

当前诊断通常包含：

- 错误码
- 主消息
- 文件路径
- 行列信息
- 可选 note

### 典型触发条件

- `$ value = "oops;`
  - 触发 `ZUN-L003`
- `$ value = ;`
  - 触发 parse 错误，当前样例包含 `ZUN-P001`
- `$>> missing;`
  - 触发 `ZUN-R004`
- `$ value = 1 / 0;`
  - 触发 `ZUN-R002`
- `$mod missing.module;`
  - 触发 `ZUN-R005`

## 最小正确示例

正确程序不会产生诊断：

```zun
$ value = 1;
$>> value;
```

对应测试：

- `tests/spec/valid/var_decl.zun`

## 最小错误示例

```zun
$>> missing;
```

对应测试：

- `tests/spec/invalid/semantics/undefined_variable.zun`

## 对应测试位置

- `tests/spec/valid/var_decl.zun`
- `tests/spec/invalid/lexical/unterminated_string.zun`
- `tests/spec/invalid/syntax/missing_rhs.zun`
- `tests/spec/invalid/semantics/undefined_variable.zun`
- `tests/spec/invalid/modules/missing_module.zun`
