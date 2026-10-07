# Zun Language Specification

本目录是 Zun 的正式语言规范入口。

Phase 8 的原则是“先记录当前实现行为，再逐步收敛规范”。因此本目录描述的是当前仓库中的真实 lexer、parser、runtime 行为，而不是一套脱离实现的理想设计。

## 适用范围

本目录覆盖：

- 词法规则
- 语法结构
- 运行时语义
- 模块解析规则
- 运行时错误与错误码

本目录暂不覆盖：

- 标准库稳定级别
- 安全模型
- benchmark 基线

这些内容在后续 Phase 中单独维护。

## 规范与实现的关系

当前规范来源如下：

- 词法规则以 `src/lexer/` 为准
- 语法规则以 `src/parser/` 为准
- 运行时语义以 `src/interpreter/` 和 `src/runtime/` 为准
- 模块解析以 `src/module/` 为准
- 诊断与错误码以 `src/diagnostics/` 和 `src/error/` 为准

如果实现与文档冲突，以“当前主分支实现 + 对应 spec 测试”作为暂时事实来源，并在后续提交中修正文档或实现。

## 文档结构

- `lexical.md`
  - token、注释、字符串、f-string、字符、数字字面量
- `syntax.md`
  - 语句、表达式、优先级、结合性
- `semantics.md`
  - 作用域、truthy/falsy、类型注解、控制流、函数行为
- `modules.md`
  - 项目根、模块命名、解析顺序、`std.*`
- `runtime-errors.md`
  - 错误类别、错误码、典型触发条件
- `versioning.md`
  - 版本策略与 SemVer 风格规则
- `compatibility.md`
  - 兼容边界、breaking change 判断标准
- `deprecation.md`
  - 弃用周期、移除窗口、迁移要求
- `security-model.md`
  - 默认能力模型、副作用边界、未来可受限入口
- `performance-baseline.md`
  - benchmark 基线、固定样例、记录模板

## 规范与测试映射

`tests/spec/` 按主题组织：

- `tests/spec/valid/lexical/`
- `tests/spec/valid/syntax/`
- `tests/spec/valid/semantics/`
- `tests/spec/valid/modules/`
- `tests/spec/invalid/lexical/`
- `tests/spec/invalid/syntax/`
- `tests/spec/invalid/semantics/`
- `tests/spec/invalid/modules/`

文档中的最小正确示例和最小错误示例，都必须能在 `tests/spec/` 中找到对应样例。

## 维护规则

任何语言行为改动，都必须同步更新：

1. 本目录中的相关规范文档
2. `tests/spec/` 中对应主题样例
3. 如果属于用户可见变化，再更新 `docs/CHANGELOG.md`

允许在文档中记录“当前实现如此，但未来可能调整”的注记；不允许在文档中虚构当前实现不存在的行为。
