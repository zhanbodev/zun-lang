# Zun Standard Library Governance

本目录定义 Zun 标准库的治理规则，而不是某一个具体模块的 API 文档。

Phase 10 的目标是先解决“标准库怎么长出来”的问题，让社区在扩展 `std.*` 时有一致的边界、模板和评审口径。

## 本目录包含

- `contributing.md`
  - 标准库贡献要求与评审清单
- `stability-levels.md`
  - `experimental / preview / stable` 的定义与升级规则

## 当前结论

- `std.*` 是官方保留命名空间
- 新模块默认 `experimental`
- 每个标准库模块至少需要代码、README、示例、测试
- 是否应进入 stdlib，要先和 builtin 边界对齐

## 与仓库结构的关系

当前 `stdlib/` 下已经预留：

- `core/`
- `io/`
- `str/`
- `json/`
- `http/`
- `_template/`

其中 `_template/` 用于给后续贡献者提供模块模板，不属于公开 `std.*` API。
