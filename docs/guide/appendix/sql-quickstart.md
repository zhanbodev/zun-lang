# SQL 快速上手

本页面向脚本作者和项目开发者，只回答“怎么连库、怎么查、怎么避坑”。

如果你要查完整 SQL API，请看 [../../reference/sql-database.md](../../reference/sql-database.md)。  
如果你要看 runtime 内部设计，请看 [../../runtime/sql-runtime.md](../../runtime/sql-runtime.md)。

## 1. 导入模块

SQLite：

```zun
$mod std.sqlite;
```

PostgreSQL：

```zun
$mod std.postgres;
```

两者都会返回 `Connection`，后续统一走：

- `conn.query(sql, params)`
- `conn.execute(sql, params)`
- `conn.close()`

## 2. 最小例子

### SQLite

```zun
$mod std.sqlite;

$ conn = sqlite.connect(":memory:");
$>> conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", []);
$>> conn.execute("INSERT INTO users (id, name) VALUES (?, ?)", [1, "Alice"]);
$ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
$>> rows;
$>> conn.close();
```

### PostgreSQL

```zun
$mod std.postgres;

$ conn = postgres.connect("host=/tmp dbname=postgres user=liangzhanbo");
$ rows = conn.query("SELECT 1 AS n", []);
$>> rows;
$>> conn.close();
```

这里的连接串不是只能写 URL。当前可以用两类常见形式：

- TCP / URL：`postgresql://liangzhanbo@127.0.0.1:5432/postgres`
- `key=value`：`host=127.0.0.1 port=5432 dbname=postgres user=liangzhanbo`
- Unix socket：`host=/tmp dbname=postgres user=liangzhanbo`

## 3. 查询与写入

`query(...)` 用于拿结果行：

```zun
$ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
```

返回值是：

```text
List<Map>
```

`execute(...)` 用于写入或更新：

```zun
$ changed = conn.execute("UPDATE users SET name = $1 WHERE id = $2", ["Alice-2", 1]);
```

返回值是受影响行数：

```text
Int
```

建议在用完后显式关闭：

```zun
$>> conn.close();
```

## 4. 占位符规则

SQLite 用 `?`：

```zun
$ rows = conn.query("SELECT id, name FROM users WHERE id = ?", [1]);
```

PostgreSQL 用 `$1`, `$2`, `$3`：

```zun
$ rows = conn.query("SELECT id, name FROM users WHERE id = $1", [1]);
```

## 5. 连接方式怎么选

TCP 连接：

```text
host=127.0.0.1 port=5432 dbname=postgres user=liangzhanbo
```

Unix socket 连接：

```text
host=/tmp dbname=postgres user=liangzhanbo
```

`host=/tmp` 的意思不是“数据库文件在 `/tmp`”，而是“去 `/tmp` 这个目录下找 PostgreSQL 的 Unix socket 文件并通过 socket 连接本机数据库”。

通常可以这样理解：

- 本机开发、socket 免密可用：优先考虑 Unix socket
- 跨机器、容器、端口明确：用 TCP

## 6. 项目里怎么放连接串

快速示例里可以直接写死连接串，但项目里更推荐：

- 在 `shared/db/connection.zun` 统一封装
- 从环境变量或项目配置读取
- 避免把用户名、密码、本地 socket 路径散落在业务代码里

思路上应接近：

```zun
$mod std.env;
$mod std.postgres;

$fn connect_db() {
    $ conninfo = env.get_or("DATABASE_URL", "host=/tmp dbname=postgres user=liangzhanbo");
    $# postgres.connect(conninfo);
}
```

## 7. HTTP 项目里怎么用

`zun serve` 下现在可以直接用 SQL 标准库。

建议项目里按这个层次组织：

- `shared/db`：连接与通用查询包装
- `data`：只写 SQL
- `services`：业务编排与错误转换
- `routers`：HTTP 参数与返回

最小路由例子：

```zun
$mod std.postgres;

$GET("/users") users() -> List<Map> {
    $ conn = postgres.connect("host=/tmp dbname=postgres user=liangzhanbo");
    $ rows = conn.query("SELECT id, name FROM users ORDER BY id", []);
    $>> conn.close();
    $# rows;
}
```

## 8. 当前最常见的坑

### PostgreSQL `numeric`

当前 runtime 不直接映射所有 PostgreSQL 类型。`numeric` 最稳妥的写法是显式 cast：

```sql
SELECT price::double precision AS price
FROM products
```

### PostgreSQL `int4` 与 Zun `Int`

Zun 的 `Int` 当前按 `i64` 绑定。和 `serial` / `int4` 列比较时，建议把列侧 cast 成 `bigint`：

```sql
SELECT id, name
FROM users
WHERE id::bigint = $1
```

### `Null` 参数绑定

SQLite 支持把 `Null` 绑定成 SQL 参数。  
PostgreSQL 当前对 `Null` 绑定会直接报错，先不要依赖这条路径。

### 复杂值不能直接绑定

以下值不要直接塞进 `params`：

- `List`
- `Map`
- `TypedInstance`

先转成标量，或者在上层把结构拆开。

## 9. 推荐实践

- `query` 只做查询，`execute` 只做写操作
- 显式关闭连接，不要把连接生命周期交给析构时机
- PostgreSQL 里优先自己写清楚 cast，而不是依赖驱动猜测
- 连接串统一收敛到 `shared/db/connection.zun` 或等价模块
- 在 `service` 层用 `$try / $catch` 把底层数据库错误改写成业务错误
- 聚合接口优先在 service 层拼装，不要在 router 里直接写多段 SQL

## 10. 继续阅读

- SQL API 查表：[../../reference/sql-database.md](../../reference/sql-database.md)
- stdlib 总览：[../13-stdlib-overview.md](../13-stdlib-overview.md)
- HTTP 组织：[../16-http-organization.md](../16-http-organization.md)
- SQL runtime 设计：[../../runtime/sql-runtime.md](../../runtime/sql-runtime.md)
