# Standard Library Contribution Policy

本页定义 Zun 标准库模块的最低准入要求。

## Step 1 先判断应不应该进入 stdlib

提交新能力前，先回答这个问题：它应该是 builtin，还是 stdlib？

### 保留为 builtin 的能力

以下能力优先保留在语言 builtin 中：

- 基础类型
- 基础控制结构
- 最小 I/O 原语

原因是这些能力属于解释器运行所必需的语言底座，不适合做成可选标准库模块。

### 优先进入 stdlib 的能力

以下能力优先作为 stdlib 演进：

- JSON 辅助
- 文件工具
- HTTP 辅助封装
- 字符串扩展工具

这类能力更适合独立演进、补文档和社区协作维护。

## Step 2 满足模块最低交付要求

一个 stdlib 模块最少必须包含：

- 代码
- README
- 示例
- 集成测试
- 稳定级别标记

推荐目录布局：

```text
stdlib/<module>/
├── README.md
├── examples/
│   └── basic.zun
└── tests/
    └── README.md
```

如果模块只有单文件实现，也应至少提供对应的 README、example 和 tests 目录。

## 模块命名规范

- 使用小写命名
- 模块路径与目录结构保持一致
- 公开命名使用 `std.<module>`
- 不要把实验性草稿直接暴露成模糊名称

例如：

- `std.io`
- `std.str`
- `std.json`
- `std.http`

## 文档要求

每个模块 README 至少要写清：

- 模块用途
- 稳定级别
- 暴露的公共能力
- 最小正确示例
- 常见错误或限制

## 测试要求

每个模块至少覆盖：

- 一个正常路径
- 一个错误路径或边界路径
- 一个与现有语言行为不冲突的回归验证

## 兼容性要求

- 新模块默认 `experimental`
- 如果修改 `preview` 或 `stable` 模块，要更新兼容说明
- 如果替换或移除公共 API，要同步遵循 `docs/spec/deprecation.md`

## 标准库评审清单

评审至少检查：

- 是否真的应该属于 stdlib，而不是 builtin
- 是否已有重复能力
- 是否有 README
- 是否有最小示例
- 是否有测试
- 是否声明稳定级别
- 是否说明兼容影响
