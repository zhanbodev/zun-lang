# Versioning Policy

本页定义 Zun 当前采用的版本策略。

## 适用对象

本策略同时适用于：

- 语言核心
- CLI 对外命令
- `std.*` 标准库命名空间
- 语言服务相关的用户可见能力

内部实现细节不直接纳入版本承诺，除非它们已经形成用户可见行为。

## 版本格式

Zun 当前采用 SemVer 风格：

- `MAJOR.MINOR.PATCH`

解释如下：

- `MAJOR`
  - 存在 breaking change 时递增
- `MINOR`
  - 向后兼容地新增语言能力、标准库能力或 CLI 能力时递增
- `PATCH`
  - 不改变对外行为的 bug fix、实现修正、文档修正时递增

## 各类变更如何计入版本

### MAJOR

以下变化视为 breaking change，原则上必须进入下一个 major：

- 删除已有语法
- 改变已有语法的解析结果
- 改变既有 runtime 语义
- 删除或重命名已有 CLI 子命令或关键参数
- 删除或重命名稳定的 `std.*` API
- 改变模块解析优先级并影响已有项目
- 改变错误行为，导致现有程序从成功变成失败，或从失败变成不同语义的成功

### MINOR

以下变化可以进入 minor：

- 新增向后兼容语法
- 新增 builtin 能力，但不改变旧行为
- 新增 `std.*` 模块或 API
- 新增 CLI 子命令或可选参数
- 新增错误码，但不改变旧错误的含义
- 将 `experimental` 能力补全文档、测试、示例

### PATCH

以下变化可以进入 patch：

- bug fix，且不改变规范承诺的行为边界
- 诊断信息更清晰，但错误类别与语义不变
- 文档修复
- 测试补充
- 内部重构

## 0.x 阶段说明

在 `0.x` 阶段，Zun 仍然采用 SemVer 风格进行沟通，但以下能力默认只提供较弱承诺：

- 标记为 `experimental` 的标准库模块
- 尚未进入正式 spec 的新能力
- 尚未在 `docs/spec/compatibility.md` 中列入稳定承诺边界的行为

即便如此，breaking change 仍然必须：

- 写明兼容影响
- 更新 `docs/CHANGELOG.md`
- 需要 RFC 或兼容说明

## 与变更记录的关系

所有发布说明都应写入：

- `docs/CHANGELOG.md`

并按固定栏目归档：

- Added
- Changed
- Deprecated
- Removed
- Fixed
