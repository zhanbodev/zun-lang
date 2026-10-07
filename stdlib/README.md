# Zun Standard Library

`stdlib/` 是 Zun 标准库预留目录。

当前阶段的目标不是一次性把标准库做满，而是先固定目录组织方式和贡献入口，为后续社区协作提供稳定边界。

当前建议结构：

```text
stdlib/
├── _template/
├── core/
├── io/
├── math/
├── str/
├── json/
└── http/
```

命名规则：

- 顶层命名空间保留给 `std.*`
- 模块路径建议与目录保持一致
- 每个标准库模块都应附带文档、示例和测试
- 新模块默认稳定级别为 `experimental`

参考文档：

- [贡献指南](./CONTRIBUTING.md)
- [标准库治理文档](../docs/stdlib/README.md)
- [语言行为索引](../docs/spec/README.md)
