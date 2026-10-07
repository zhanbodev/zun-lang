# SQL Runtime 设计说明

本页面向 Zun 解释器和 runtime 开发者，整理 SQL 能力在 runtime 内部的分层、对象模型、已修复问题和当前边界。

如果你是脚本使用者，请先看 [../reference/sql-database.md](../reference/sql-database.md)。  
如果你只想快速上手，请看 [../guide/appendix/sql-quickstart.md](../guide/appendix/sql-quickstart.md)。

## 目标

SQL 能力的设计目标不是把数据库 API 散落在解释器执行路径中，而是统一收敛到 runtime intrinsic 层，再由标准库暴露给脚本。

当前目标有三条：

- 让 `std.sqlite` / `std.postgres` 共享一套 `Connection` 值模型
- 让 SQL 能力在 `run` 和 `serve` 模式下行为一致
- 保持脚本表面 API 简单，但把宿主相关复杂度压在 runtime 内部

## 总体分层

当前分层如下：

```text
script / router / std module
  -> Connection builtin dispatch
  -> runtime intrinsic
  -> sql registry + host client
  -> sqlite / postgres driver
```

对应到代码，大致是：

- `stdlib_native/sqlite.rs`
- `stdlib_native/postgres.rs`
- `interpreter/builtins/sql_connection.rs`
- `runtime/sql_intrinsics.rs`
- `runtime/sql_registry.rs`

如果继续细看职责：

- `stdlib_native/sqlite.rs` / `stdlib_native/postgres.rs`：向脚本注册 `sqlite.connect(...)` / `postgres.connect(...)`
- `interpreter/builtins/sql_connection.rs`：把 `Connection.query/execute/close` 分发到 intrinsic
- `runtime/sql_intrinsics.rs`：做参数校验、连接创建、query/execute/close 调用
- `runtime/sql_registry.rs`：保存底层连接对象并管理句柄生命周期

## 脚本表面 API

标准库暴露两条入口：

- `sqlite.connect(...)`
- `postgres.connect(...)`

两者都返回 `ZunValue::Connection`，随后脚本通过值方法继续操作：

- `conn.query(sql, params)`
- `conn.execute(sql, params)`
- `conn.close()`

这里的关键点是：脚本层并不知道底层是 SQLite 还是 PostgreSQL，区别只体现在连接创建与 SQL 占位符语法。

## runtime 里的核心对象

### `ZunValue::Connection`

解释器层不直接持有驱动对象，而是持有一个 runtime 连接句柄。这个值负责：

- 作为脚本层可传递对象存在
- 被 `Connection.query/execute/close` builtin 识别
- 作为 registry 查找底层连接的 key

### SQL registry

registry 是 SQL runtime 的中心状态，负责：

- 保存活跃连接
- 为连接分配和回收句柄
- 根据句柄找到真实驱动对象

当前它同时管理 SQLite 与 PostgreSQL 两类连接。

### SQL intrinsics

当前 intrinsic 层已经包含：

- `SqlSqliteConnect`
- `SqlPostgresConnect`
- `SqlQuery`
- `SqlExecute`
- `SqlClose`

它们的职责是把脚本值转换成 runtime 可执行调用，并统一报错出口。

## `Connection` 方法分发

`Connection.query(...)`、`Connection.execute(...)`、`Connection.close()` 并不是脚本模块函数，而是 builtin 方法分发。

这一层的职责是：

- 校验接收者是否为 `Connection`
- 校验参数个数与基本形状
- 把调用转给 intrinsic

这么做的原因是 `Connection` 是 runtime 句柄值，不适合再经一层脚本模块包装。

## SQLite 与 PostgreSQL 的分工

### SQLite

SQLite 侧相对直接：

- connect 时打开连接
- query / execute 时直接调用驱动
- close 时从 registry 移除并析构

SQLite 当前更适合作为最小可用数据库路径和脚本示例。

### PostgreSQL

PostgreSQL 侧更复杂，主要是驱动对象与 async runtime 的交互问题。

当前 `std.postgres` 走同步 `postgres` client，但需要保证它在 `serve` 模式里不会因为析构时机错误而触发 runtime 嵌套 panic。

## 已修复的 `serve` 模式问题

### 问题现象

在 `zun serve` 的 HTTP handler 里执行：

- `postgres.connect(...)`
- `conn.query(...)`
- `conn.close()`

早期会触发：

```text
Cannot start a runtime from within a runtime
```

脚本模式 `zun run file.zun` 正常，但 `serve` 模式失败。

### 根因

底层同步 `postgres::Client` 在 Tokio worker 上 drop 时，会触发内部 runtime 行为；而 `serve` 本身已经运行在 runtime 中，于是出现嵌套 runtime panic。

### 修复策略

当前 runtime 用 `PostgresClientHandle` 包装真实 `postgres::Client`，核心目标是把真实 client 的 drop 挪到普通线程中执行，而不是在 HTTP worker 线程里直接析构。

这个包装存在的原因不是为了改变脚本 API，而是为了修正宿主资源释放时机。

这样修复后：

- `run` 模式继续可用
- `serve` 模式也能安全完成 connect/query/close

## 路由模块与类型传播

SQL 文档里还需要同时记住一个相关修复：`.link()` 路由不是 SQL 问题本身，但会直接影响数据库项目能否正常组织。

修复前，linked router 在独立模块上下文中运行，导入的 `$Type` 不会稳定传播回主 runtime，上层项目在 HTTP 请求路径里容易丢类型定义。

当前 runtime 已补两层保证：

- linked module 加载后把类型定义合并回主 `RuntimeContext`
- 模块函数声明时捕获 `module_env`，调用时再注入

这保证了数据库项目里常见的：

- router 调 service
- service 调 data
- data 返回 `$Type` 实例

可以在 `.link()` 组织方式下继续正常工作。

## 当前已知边界

### PostgreSQL 类型覆盖不完整

runtime 当前只覆盖常见标量结果类型，不是完整 PostgreSQL 类型系统。

如果列类型不在当前映射内，query 会直接报错。实际项目里应优先：

- 在 SQL 里显式 cast
- 把对外返回列收敛到已支持类型

### 参数绑定是“Zun 标量 -> 驱动参数”

当前稳定支持的仍是常见标量：

- `Int`
- `Float`
- `String`
- `Bool`

SQLite 支持 `Null`，PostgreSQL 当前不支持 `Null` 绑定。

### `Int` 与 PostgreSQL `int4`

Zun `Int` 现在按 `i64` 绑定。与 PostgreSQL `serial` / `int4` 列比较时，实际项目里更稳妥的写法是列侧显式 cast：

```sql
WHERE id::bigint = $1
```

### `numeric` 建议显式转 `double precision`

如果业务字段是 `numeric`，当前更推荐：

```sql
price::double precision AS price
```

否则 runtime 结果解码可能失败。

## 当前建议补的文档与测试

为了让 SQL runtime 后续更稳，文档和测试层目前最值得继续补的是：

- 一张更完整的 PostgreSQL 类型兼容矩阵
- `conninfo` 连接形式的专门说明与示例
- 针对 TCP 与 Unix socket 两种连接形式的集成测试
- 针对 `Null`、`numeric`、`int4` 边界的回归测试

## 设计约束

当前 SQL runtime 仍遵循这些约束：

- 不把驱动对象直接暴露给脚本
- 不让 `std.*` 绕过 intrinsic 直接改 runtime 内部状态
- `run` 和 `serve` 必须走同一套 `Connection` 表面语义
- 报错优先保持可定位，不优先做“智能猜测修复”

## 后续演进方向

比较明确的后续工作有：

- 扩大 PostgreSQL 结果类型映射
- 补 `Null` 参数绑定
- 统一 SQLite / PostgreSQL 的更多错误码转译
- 在文档与 spec 中补更清晰的 SQL 类型兼容矩阵
- 为项目化数据库开发补更多真实 fixture 与 integration test

## 相关文档

- SQL 使用说明：[../reference/sql-database.md](../reference/sql-database.md)
- SQL 快速上手：[../guide/appendix/sql-quickstart.md](../guide/appendix/sql-quickstart.md)
- runtime intrinsic 总览：[intrinsics.md](intrinsics.md)
- 安全模型：[../spec/security-model.md](../spec/security-model.md)
