# Runtime Intrinsics

本页记录 Zun runtime intrinsic 层的设计目标和当前实现边界。

## 目标

runtime intrinsic 层用于承接所有直接触达宿主 OS / 进程环境的能力，避免这些逻辑继续散落在：

- `eval/exec`
- builtin 方法
- 其他 runtime 子模块

这层不是面向普通脚本用户的公开 API，而是未来 `std.*` 的底座。

## 当前结构

当前实现包含：

- `IntrinsicId`
  - 使用枚举标识 intrinsic，避免字符串漂移
- `IntrinsicCall`
  - 统一调用签名：`&[ZunValue] -> Result<ZunValue, Error>`
- `IntrinsicRegistry`
  - 负责注册和分发 intrinsic
- `RuntimePolicy`
  - 负责允许或拒绝某个 intrinsic

## 当前首批 intrinsic

- `FsReadText`
- `FsReadLines`
- `FsWriteText`
- `FsAppendText`
- `FsDelete`
- `FsExists`
- `FsSize`
- `FsIsDir`
- `EnvGet`
- `ConfigGet`
- `SqlSqliteConnect`
- `SqlPostgresConnect`
- `SqlQuery`
- `SqlExecute`
- `SqlClose`

## 当前调用原则

- `std.fs`、`std.env`、`$<<CONFIG(...)` 等宿主能力统一落到 intrinsic 层
- `$>>` / `$>>ERR(...)` 继续保留为输出语法
- `$<<FILE(...)` / `$>>FILE(...)` 已移除；文件能力统一走 `std.fs`
- `RuntimePolicy` 当前默认 allow-all，不改变现有用户行为

## 与标准库的关系

本层不直接等于标准库。

关系是：

```text
language surface / builtin
  -> runtime intrinsic
  -> host OS / process environment
```

未来标准库将建立在这层之上，例如：

- `std.fs.read_text()` -> `FsReadText`
- `std.env.get()` -> `EnvGet`
- `std.config.get()` -> `ConfigGet`
- `std.sqlite.connect()` -> `SqlSqliteConnect`
- `std.postgres.connect()` -> `SqlPostgresConnect`
- `Connection.query()` / `Connection.execute()` / `Connection.close()` -> `SqlQuery` / `SqlExecute` / `SqlClose`

数据库连接能力现在也遵循同一规则：先进入 runtime intrinsic 层，再由 `std.sqlite` / `std.postgres` 暴露为脚本 API。

如果你要单独看 SQL 这条链路的对象模型、`serve` 模式修复和当前边界，请继续看 [sql-runtime.md](sql-runtime.md)。

## 变更规则

后续如果新增宿主能力：

1. 先进入 intrinsic 层
2. 再决定是否公开为 `std.*`
3. 同步更新安全模型与开发文档
