# SQL 与数据库

本页作为 SQL / 数据库能力的总索引，整理：

- `std.sqlite`
- `std.postgres`
- `Connection.query(...)`
- `Connection.execute(...)`
- `Connection.close()`
- 使用入口与 runtime 设计文档

如果你只想快速开始，先看 [../guide/appendix/sql-quickstart.md](../guide/appendix/sql-quickstart.md)。  
如果你只想查函数签名，先看 [stdlib-api.md](stdlib-api.md)。  
如果你想知道这些能力在 runtime 里的位置，再看 [../runtime/sql-runtime.md](../runtime/sql-runtime.md)。  
如果你关心能力边界和风险，再看 [../spec/security-model.md](../spec/security-model.md)。

## 总览

Zun 当前通过原生模块提供两条数据库路径：

- SQLite：`$mod std.sqlite;`
- PostgreSQL：`$mod std.postgres;`

两者都会返回 `Connection` 句柄，然后通过值方法完成查询、写入和关闭：

- `conn.query(sql, params)`
- `conn.execute(sql, params)`
- `conn.close()`

当前这条链路在：

- `zun run file.zun`
- `zun serve path`

下都可以使用。

## 阅读路径

按用途分，建议这样看：

- 快速上手和项目示例：[../guide/appendix/sql-quickstart.md](../guide/appendix/sql-quickstart.md)
- 完整 stdlib 查表：[stdlib-api.md](stdlib-api.md)
- runtime 内部实现：[../runtime/sql-runtime.md](../runtime/sql-runtime.md)
- intrinsic 总览：[../runtime/intrinsics.md](../runtime/intrinsics.md)
- 安全边界：[../spec/security-model.md](../spec/security-model.md)

## SQLite

最小例子：

```zun
$mod std.sqlite;

$ conn = sqlite.connect(":memory:");
$>> conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", []);
$>> conn.execute("INSERT INTO users (id, name) VALUES (?, ?)", [1, "Alice"]);
$>> conn.query("SELECT id, name FROM users ORDER BY id", []);
$>> conn.close();
```

### 占位符

SQLite 保持原生占位符：

```text
?
```

例子：

```zun
$ rows = conn.query("SELECT id, name FROM users WHERE id = ?", [1]);
```

### 绑定参数

当前支持：

- `Int`
- `Float`
- `String`
- `Bool`
- `Null`

当前不支持把更复杂的值直接绑定成 SQL 参数，例如：

- `List`
- `Map`
- `TypedInstance`

### 结果集

`conn.query(...)` 返回：

```text
List<Map>
```

每一行是一个 `Map`，列名作为 key。

## PostgreSQL

最小例子：

```zun
$mod std.postgres;

$ conn = postgres.connect("host=/tmp dbname=postgres user=liangzhanbo");
$>> conn.query("SELECT 1 AS n", []);
$>> conn.close();
```

### 连接串

`postgres.connect(...)` 直接接收 PostgreSQL 连接配置字符串。

它不只接受 URL，也接受 PostgreSQL 常见的 `key=value` 形式。

当前仓库开发时验证过的本地例子：

```text
host=/tmp dbname=postgres user=liangzhanbo
```

也可以使用常见的 PostgreSQL URL：

```text
postgresql://liangzhanbo@127.0.0.1:5432/postgres
```

或者显式写成 TCP 连接：

```text
host=127.0.0.1 port=5432 dbname=postgres user=liangzhanbo
```

实际是否能连通仍取决于宿主机环境。

### TCP 与 Unix socket

PostgreSQL 当前既可以通过 TCP 连接，也可以通过 Unix socket 连接。

TCP 例子：

```text
host=127.0.0.1 port=5432 dbname=postgres user=liangzhanbo
```

Unix socket 例子：

```text
host=/tmp dbname=postgres user=liangzhanbo
```

这里的 `host=/tmp` 不是数据库目录，也不是数据文件目录，而是 PostgreSQL 本地 socket 文件所在目录。

也就是说：

- `host=127.0.0.1` / `host=localhost`：通常表示走 TCP
- `host=/tmp`：通常表示去 `/tmp` 下查找 PostgreSQL 的 Unix socket 并通过本地 socket 通信

如果本机 PostgreSQL 配置成通过 socket 免密访问，本地开发用 `host=/tmp ...` 往往更直接。

### 占位符

PostgreSQL 保持原生占位符：

```text
$1, $2, $3, ...
```

例子：

```zun
$ rows = conn.query("SELECT id, name FROM users WHERE id = $1", [1]);
```

### 绑定参数

当前支持常见标量类型：

- `Int`
- `Float`
- `String`
- `Bool`

当前边界：

- `Null` 绑定会显式报错
- 更复杂的值类型不会自动转换为 PostgreSQL 参数

### 结果集

`conn.query(...)` 返回：

```text
List<Map>
```

列类型当前不是“全 PostgreSQL 覆盖”。如果某列类型没有被 runtime 映射支持，会直接报错。

## `Connection`

### `conn.query(sql, params)`

返回：

```text
List<Map>
```

适合：

- `SELECT`
- 需要拿回结果行的查询

例子：

```zun
$ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
$ first = rows[0];
$>> first["name"];
```

### `conn.execute(sql, params)`

返回：

```text
Int
```

表示受影响行数。

适合：

- `INSERT`
- `UPDATE`
- `DELETE`
- DDL

例子：

```zun
$ changed = conn.execute("UPDATE users SET name = $1 WHERE id = $2", ["Alice-2", 1]);
$>> changed;
```

### `conn.close()`

返回：

```text
Null
```

建议显式关闭，不要把连接生命周期完全交给宿主析构时机。

## HTTP 中使用

当前可以在 HTTP handler 中直接使用数据库连接：

```zun
$mod std.postgres;

$GET("/users") users() -> List<Map> {
    $ conn = postgres.connect("host=/tmp dbname=postgres user=liangzhanbo");
    $ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
    $>> conn.close();
    $# rows;
}
```

如果你做项目化开发，更推荐：

- `shared/db` 统一连接
- `data` 只写 SQL
- `services` 做聚合和错误转换
- `routers` 只管 HTTP 参数与返回

连接串本身更推荐从项目配置或环境变量读取，而不是直接把开发机用户名和本地路径写死在业务代码里。

## 当前边界与实用建议

## 类型兼容矩阵

### 参数绑定

| Zun 值类型 | SQLite | PostgreSQL | 说明 |
|------|--------|------|--------|
| `Int` | 支持 | 支持 | PostgreSQL 下与 `int4` 比较时，列侧显式 cast 成 `bigint` 更稳 |
| `Float` | 支持 | 支持 | PostgreSQL `numeric` 字段建议先 cast 再比较或返回 |
| `String` | 支持 | 支持 | 常规文本参数 |
| `Bool` | 支持 | 支持 | 常规布尔参数 |
| `Null` | 支持 | 当前不支持 | PostgreSQL 当前会显式报错 |
| `List` | 不支持 | 不支持 | 不会自动展开 |
| `Map` | 不支持 | 不支持 | 不会自动序列化 |
| `TypedInstance` | 不支持 | 不支持 | 先拆成标量字段 |

### 结果列

| 数据库 | 当前稳定返回 | 当前已知边界 |
|------|--------|--------|
| SQLite | 常见标量列 | `BLOB` 当前会报错 |
| PostgreSQL | 常见标量列 | 不支持完整 PostgreSQL 类型系统，未映射类型会报错 |

### 1. PostgreSQL 不支持所有列类型

当前实现支持常见标量，但不是完整 PostgreSQL 类型系统。

如果遇到：

```text
postgres column 'x' has unsupported type '...'
```

优先做法是显式 cast 到当前已支持的类型。

例如 `numeric` 可以在 SQL 里写成：

```sql
price::double precision AS price
```

### 2. PostgreSQL 参数比较有时需要显式 cast

如果宿主列类型和 Zun 绑定值在驱动侧不完全一致，可能会遇到序列化或比较问题。

比较稳妥的写法是把列侧显式 cast 到你要比较的类型，例如：

```sql
WHERE id::bigint = $1
```

这类写法对当前实现更稳。

### 3. PostgreSQL `Null` 绑定当前不可用

如果你要传空值，不要默认依赖：

```zun
[null]
```

当前 PostgreSQL 路径会直接报错。需要调用方先提供更明确的 SQL / 类型方案。

### 4. SQLite `BLOB` 当前不自动转换

如果结果列是 SQLite `BLOB`，当前会显式报错，不会自动转成字符串或 list。

### 5. 错误处理

数据库错误会作为普通运行时错误抛出，可以配合 `$try / $catch`：

```zun
$try {
    $ conn = postgres.connect("host=/tmp dbname=postgres user=liangzhanbo");
    $ rows = conn.query("SELECT id FROM users", []);
    $>> conn.close();
    $# rows;
} $catch err {
    $>>ERR(err);
}
```

## 推荐写法

### 单文件脚本

```zun
$mod std.postgres;

$ conn = postgres.connect("host=/tmp dbname=postgres user=liangzhanbo");
$ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
$>> rows;
$>> conn.close();
```

### 项目化 HTTP

```zun
$mod shared.db.queries;

$fn list_users_rows() -> List<Map> {
    $# queries.safe_query(
        "SELECT id, name, email, created_at::text AS created_at FROM users ORDER BY id",
        []
    );
}
```

```zun
$GET("/users") get_users() -> List<Map> {
    $# user_service.list_users();
}
```

## 相关页面

- [stdlib-api.md](stdlib-api.md)
- [../guide/13-stdlib-overview.md](../guide/13-stdlib-overview.md)
- [../runtime/intrinsics.md](../runtime/intrinsics.md)
- [../spec/security-model.md](../spec/security-model.md)
