# Standard Library Contributing Guide

本文档定义 Zun 标准库的最低贡献要求。

标准治理规则的正式版本见：

- [`docs/stdlib/README.md`](../docs/stdlib/README.md)
- [`docs/stdlib/contributing.md`](../docs/stdlib/contributing.md)
- [`docs/stdlib/stability-levels.md`](../docs/stdlib/stability-levels.md)

## 目录组织

建议使用下面的目录布局：

```text
stdlib/
├── core/
├── io/
├── str/
├── json/
└── http/
```

模块命名要求：

- 使用小写命名
- 模块路径和目录结构保持一致
- `std.*` 命名空间只用于标准库

## 提交要求

新增或修改标准库模块时，必须同时提供：

- 模块实现
- 模块文档
- 最小示例
- 测试
- 兼容性说明
- 稳定级别标记

## 文档要求

每个标准库模块至少要说明：

- 模块用途
- 暴露的函数或能力
- 最小正确示例
- 常见错误示例

## 测试要求

标准库贡献至少要覆盖：

- 正常路径
- 一个错误路径
- 与现有语言行为不冲突的回归验证

## 向后兼容要求

- 不要随意修改已公开的标准库行为
- 如果必须调整 API，需要先补兼容说明
- 如果行为变更影响 `tests/spec`，必须同步更新相关样例
