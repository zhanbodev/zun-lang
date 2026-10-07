# Zun Docs

本目录现在按三层组织：

- Guide：按学习路径讲“先学什么、怎么用”
- Reference：按主题查语法、HTTP、错误、stdlib API
- Spec：记录当前实现边界，和 `tests/spec/*` 对齐

如果三层文档出现冲突，优先顺序是：

1. 当前实现与可执行测试
2. `docs/spec/*`
3. Guide / Reference

## 快速入口

- Guide 入口：[guide/README.md](guide/README.md)
- Reference 入口：
  - [reference/syntax.md](reference/syntax.md)
  - [reference/errors.md](reference/errors.md)
  - [reference/http.md](reference/http.md)
  - [reference/project-system.md](reference/project-system.md)
  - [reference/stdlib-api.md](reference/stdlib-api.md)
- Spec 入口：[spec/README.md](spec/README.md)

## 推荐阅读顺序

第一次接触 Zun：

1. 先读 [guide/README.md](guide/README.md)
2. 按编号章节读到 `16`
3. 需要查完整 API 时跳到 [reference/stdlib-api.md](reference/stdlib-api.md)
4. 对语义边界有疑问时再看 [spec/README.md](spec/README.md)

已经在项目里排障：

1. 先查 [reference/errors.md](reference/errors.md)
2. HTTP 问题查 [reference/http.md](reference/http.md)
3. 类型、模块、返回值争议回看 `spec/`

## 迁移状态

`docs/guide/` 下仍保留一批旧页面，用于迁移过渡。它们不再是主线入口，状态分三类：

- `Deprecated`：内容与现有主线冲突，不应继续当教学入口
- `Supplemental`：仍有背景信息或补充价值，但不承担主线职责
- `Migration Source`：保留可迁移内容或旧索引，方便追溯

阅读主线时，优先使用编号章节和 reference 页面。
