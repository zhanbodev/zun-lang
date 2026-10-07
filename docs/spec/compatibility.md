# Compatibility Policy

本页定义 Zun 当前的兼容边界，以及社区在判断 breaking change 时应遵循的标准。

## 兼容边界

Phase 9 先明确两套边界：

- 语言核心兼容边界
- 标准库 API 兼容边界

## 语言核心兼容边界

以下内容属于语言核心兼容承诺的一部分：

- `docs/spec/lexical.md` 中定义的 token 与注释行为
- `docs/spec/syntax.md` 中定义的语句结构与表达式优先级
- `docs/spec/semantics.md` 中定义的 truthy/falsy、控制流、函数返回与类型注解语义
- `docs/spec/modules.md` 中定义的模块解析顺序与 `std.*` 保留命名空间
- `docs/spec/runtime-errors.md` 中定义的错误类别和错误码含义

如果这些行为发生变化，并可能影响已有脚本的运行结果，就按 breaking change 处理。

## 标准库 API 兼容边界

当前标准库兼容边界只覆盖：

- `std.*` 命名空间本身
- 已发布且被标记为 `stable` 的标准库模块
- 已在标准库文档中明确列出的公共函数、方法、输入输出约定

以下内容当前默认不提供强兼容承诺：

- `experimental` 模块
- 仅在示例中出现、但未正式文档化的 API
- 尚未通过稳定级别评审的模块

## 对外承诺

### Parser 行为

以下内容视为 parser 的对外承诺：

- 已进入 spec 的语法结构
- 表达式优先级和结合性
- 已进入稳定状态的 parse 诊断类别

### Runtime 语义

以下内容视为 runtime 的对外承诺：

- 条件判断的 truthy/falsy 规则
- 函数调用与返回语义
- 类型注解检查规则
- `$mod` 模块导入和 `$HTTP(...).link()` 路由挂载的基本行为

### CLI 命令

以下内容视为 CLI 的对外承诺：

- `run`
- `serve`
- `test`
- 默认 REPL 入口

删除、重命名或改变这些命令的基本行为，需要按 breaking change 处理。

### `std.*` 命名空间

以下内容视为当前对外承诺：

- `std.*` 为保留命名空间
- 用户项目不得依赖未来可被官方回收的同名空间
- 官方标准库模块一旦进入 `stable`，其公共 API 不得随意重写

## 兼容性判断准则

以下情况通常判定为 breaking change：

- 旧脚本无法解析
- 旧脚本能解析但语义改变
- 旧脚本原本成功运行，现在失败
- 旧脚本原本失败，现在成功但结果语义改变
- 已稳定的 `std.*` API 被删除、重命名或改参
- 已稳定的 CLI 命令被删除、重命名或改默认行为

以下情况通常不算 breaking change：

- 增加新语法，但不影响旧语法解析
- 增加新 `std.*` 模块
- 增加可选 CLI 参数
- 改善错误消息文本，但不改变错误类别与语义
- 内部重构，不改变用户可见行为

## Breaking Change 的处理门槛

未来任何 breaking change，必须至少满足以下之一：

- 提交 RFC，见 `docs/process/rfc.md`
- 附带独立兼容说明文档

同时必须更新：

- `docs/spec/*`
- `docs/CHANGELOG.md`
- 相关测试
