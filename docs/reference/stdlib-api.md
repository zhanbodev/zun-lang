# 标准库 API 参考

这是 Zun 当前 stdlib 与常用值方法的权威查表入口。Guide 里的 [../guide/13-stdlib-overview.md](../guide/13-stdlib-overview.md) 负责“认路”，本页负责“查完整 API”。

## 使用说明与稳定性标签

本页按两层组织：

- 值方法
- `std.*` 模块 API

稳定性标签说明：

- `Stable`：已进入当前 Guide 主线，可作为常规用法依赖
- `Preview`：当前仅在 reference / 迁移材料中保留，适合试用，但不应比主线承诺更多语义

如果某个行为和实现有冲突，以当前实现与 `docs/spec/*` 为准。

## 值方法索引

### String

| 方法 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `s.len()` | `Int` | 字符串长度 | Stable |
| `s.upper()` | `String` | 转大写 | Stable |
| `s.lower()` | `String` | 转小写 | Stable |
| `s.trim()` | `String` | 去除首尾空白 | Stable |
| `s.contains(sub)` | `Bool` | 是否包含子串 | Stable |
| `s.starts_with(prefix)` | `Bool` | 前缀判断 | Stable |
| `s.ends_with(suffix)` | `Bool` | 后缀判断 | Stable |
| `s.replace(old, new)` | `String` | 替换所有匹配 | Stable |
| `s.split(sep)` | `List` | 按分隔符拆分 | Stable |
| `s.slice(start, end)` | `String` | 切片 | Stable |
| `s.to_int()` | `Int` | 转整数 | Preview |
| `s.to_float()` | `Float` | 转浮点数 | Preview |
| `s.to_bool()` | `Bool` | 转布尔值 | Preview |
| `s.type()` | `String` | 返回类型名 | Stable |

### List

| 方法 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `list.len()` | `Int` | 列表长度 | Stable |
| `list.contains(x)` | `Bool` | 是否包含元素 | Stable |
| `list.index_of(x)` | `Int` | 第一次出现的索引 | Preview |
| `list.last_index_of(x)` | `Int` | 最后一次出现的索引 | Preview |
| `list.slice(start, end)` | `List` | 切片 | Stable |
| `list.first()` | `Any` | 第一个元素 | Preview |
| `list.last()` | `Any` | 最后一个元素 | Preview |
| `list.is_empty()` | `Bool` | 是否为空 | Preview |
| `list.flatten()` | `List` | 展平一层嵌套 | Stable |
| `list.unique()` | `List` | 去重，保持顺序 | Stable |
| `list.count(x)` | `Int` | 统计元素出现次数 | Stable |
| `list.join(sep)` | `String` | 用分隔符连接元素 | Stable |
| `list.type()` | `String` | 返回 `"List"` | Preview |
| `list.push(x)` | `List` | 末尾追加元素 | Stable |
| `list.pop()` | `Any` | 移除并返回末尾元素 | Stable |
| `list.reverse()` | `List` | 原地反转 | Stable |
| `list.sort()` | `List` | 原地升序排序 | Stable |
| `list.sort_desc()` | `List` | 原地降序排序 | Stable |
| `list.remove_at(i)` | `List` | 移除指定索引元素 | Preview |
| `list.insert(i, x)` | `List` | 在指定索引插入元素 | Preview |
| `list.clear()` | `List` | 清空列表 | Preview |

### Map

| 方法 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `map.len()` | `Int` | 键值对数量 | Stable |
| `map.keys()` | `List` | 所有键 | Stable |
| `map.values()` | `List` | 所有值 | Stable |
| `map.contains_key(key)` | `Bool` | 是否包含某键 | Stable |
| `map.remove(key)` | `Map` | 删除键 | Preview |
| `map.type()` | `String` | 返回 `"Map"` | Preview |

Map 直接写入：

```zun
$ user = {"name": "Tom"};
user["email"] = "tom@example.com";
```

### Int / Float

| 方法 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `n.to_str()` | `String` | 转字符串 | Preview |
| `n.to_int()` | `Int` | 转整数 | Preview |
| `n.to_float()` | `Float` | 转浮点数 | Preview |
| `n.to_bool()` | `Bool` | 转布尔值 | Preview |
| `n.type()` | `String` | 返回类型名 | Preview |

## `std.*` 模块 API

### `std.str`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `str.trim(s)` | `String` | 去除首尾空白 | Stable |
| `str.trim_start(s)` | `String` | 去除开头空白 | Preview |
| `str.trim_end(s)` | `String` | 去除末尾空白 | Preview |
| `str.split(s, sep)` | `List` | 按分隔符拆分 | Stable |
| `str.join(list, sep)` | `String` | 用分隔符连接列表 | Preview |
| `str.replace(s, from, to)` | `String` | 替换子串 | Preview |
| `str.contains(s, sub)` | `Bool` | 包含检查 | Stable |
| `str.starts_with(s, prefix)` | `Bool` | 前缀检查 | Preview |
| `str.ends_with(s, suffix)` | `Bool` | 后缀检查 | Preview |
| `str.to_upper(s)` | `String` | 转大写 | Preview |
| `str.to_lower(s)` | `String` | 转小写 | Preview |
| `str.len(s)` | `Int` | 长度 | Preview |
| `str.parse_int(s)` | `Int` | 解析为整数 | Preview |
| `str.parse_float(s)` | `Float` | 解析为浮点数 | Preview |

### `std.math`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `math.sqrt(x)` | `Float` | 平方根 | Stable |
| `math.pow(base, exp)` | `Float` | 幂运算 | Preview |
| `math.abs(x)` | `Int\|Float` | 绝对值 | Preview |
| `math.floor(x)` | `Int` | 向下取整 | Preview |
| `math.ceil(x)` | `Int` | 向上取整 | Preview |
| `math.round(x)` | `Int` | 四舍五入 | Preview |
| `math.min(a, b)` | `Int\|Float` | 较小值 | Preview |
| `math.max(a, b)` | `Int\|Float` | 较大值 | Preview |
| `math.clamp(x, min, max)` | `Int\|Float` | 限制范围 | Preview |
| `math.hypot(x, y)` | `Float` | 直角三角形斜边 | Preview |
| `math.pi()` | `Float` | 圆周率 | Preview |
| `math.e()` | `Float` | 自然常数 | Preview |
| `math.sin(x)` | `Float` | 正弦 | Preview |
| `math.cos(x)` | `Float` | 余弦 | Preview |
| `math.tan(x)` | `Float` | 正切 | Preview |
| `math.asin(x)` | `Float` | 反正弦 | Preview |
| `math.acos(x)` | `Float` | 反余弦 | Preview |
| `math.atan(x)` | `Float` | 反正切 | Preview |
| `math.atan2(y, x)` | `Float` | 四象限反正切 | Preview |
| `math.exp(x)` | `Float` | 指数函数 | Preview |
| `math.ln(x)` | `Float` | 自然对数 | Preview |
| `math.log(x, base)` | `Float` | 任意底对数 | Preview |
| `math.log2(x)` | `Float` | 以 2 为底 | Preview |
| `math.log10(x)` | `Float` | 以 10 为底 | Preview |
| `math.random()` | `Float` | `[0.0, 1.0)` 随机数 | Preview |
| `math.random_int(min, max)` | `Int` | `[min, max]` 随机整数 | Preview |

### `std.fs`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `fs.write(path, content)` | `Null` | 覆盖写入文本 | Stable |
| `fs.append(path, content)` | `Null` | 追加写入文本 | Stable |
| `fs.delete(path)` | `Null` | 删除文件 | Stable |
| `fs.read_text(path)` | `String` | 读取全文 | Stable |
| `fs.read_lines(path)` | `List` | 按行读取 | Stable |
| `fs.exists(path)` | `Bool` | 文件/目录是否存在 | Stable |
| `fs.size(path)` | `Int` | 文件大小 | Stable |
| `fs.is_dir(path)` | `Bool` | 是否为目录 | Stable |
| `fs.write_text(path, content)` | `Null` | `fs.write(...)` 的兼容别名 | Preview |
| `fs.append_text(path, content)` | `Null` | `fs.append(...)` 的兼容别名 | Preview |
| `fs.copy(src, dst)` | `Null` | 复制文件 | Preview |
| `fs.rename(src, dst)` | `Null` | 移动/重命名 | Preview |
| `fs.list(path)` | `List` | 列出目录内容 | Preview |
| `fs.mkdir(path)` | `Null` | 创建目录 | Preview |
| `fs.mkdir_all(path)` | `Null` | 递归创建目录 | Preview |
| `fs.rmdir(path)` | `Null` | 删除空目录 | Preview |

主推荐写法：

```zun
$mod std.fs;

fs.write("output.txt", "Hello World");
fs.append("output.txt", "\nSecond line");

$ text = fs.read_text("output.txt");
$ lines = fs.read_lines("output.txt");

$>> text;
$>> lines;
```

`$>>FILE(...)` / `$<<FILE(...)` 已移除；继续使用会报 `ZUN-P008`，应迁移到 `std.fs`。

### `std.json`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `json.parse(str)` | `Map\|List` | 解析 JSON 字符串 | Preview |
| `json.stringify(value)` | `String` | 转为 JSON 字符串 | Stable |
| `json.pretty(value)` | `String` | 美化 JSON 输出 | Preview |
| `json.get(obj, key)` | `Any` | 获取对象中某键的值 | Preview |
| `json.has(obj, key)` | `Bool` | 键是否存在 | Preview |
| `json.keys(obj)` | `List` | 所有键 | Preview |
| `json.values(obj)` | `List` | 所有值 | Preview |
| `json.set(obj, key, value)` | `Map` | 设置键值并返回新对象 | Stable |
| `json.delete(obj, key)` | `Map` | 删除键并返回新对象 | Preview |
| `json.merge(a, b)` | `Map` | 合并两个对象 | Preview |

### `std.env`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `env.get(key)` | `String` | 获取环境变量，不存在时报错 | Preview |
| `env.get_or(key, default)` | `String` | 获取环境变量或返回默认值 | Stable |
| `env.has(key)` | `Bool` | 检查环境变量是否存在 | Stable |
| `env.all()` | `Map` | 返回所有环境变量 | Preview |
| `env.set(key, value)` | `Null` | 设置环境变量 | Preview |
| `env.remove(key)` | `Null` | 删除环境变量 | Preview |

### `std.auth.session`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `session.current()` | `Map\|Null` | 返回当前请求 session | Preview |
| `session.exists()` | `Bool` | 当前请求是否已有 session | Preview |
| `session.id()` | `String\|Null` | 返回当前 session id | Preview |
| `session.create(payload)` | `Map` | 创建 session，写入 store，并在响应阶段附加 `Set-Cookie` | Preview |
| `session.get(key)` | `Any\|Null` | 读取当前 session claims 中某个键 | Preview |
| `session.set(key, value)` | `Null` | 更新当前 session claims 并持久化 | Preview |
| `session.delete(key)` | `Null` | 删除当前 session claims 中某个键并持久化 | Preview |
| `session.rotate()` | `Map` | 轮换 session id，更新 store 并附加新的 `Set-Cookie` | Preview |
| `session.destroy()` | `Null` | 删除当前 session，并在响应阶段清除 cookie | Preview |

`session.create(payload)` 当前要求 `payload` 至少包含：

- `subject: String`

可选字段：

- `roles: List<String>`
- `permissions: List<String>`
- `claims: Map`

返回结构包含：

- `id`
- `subject`
- `scheme`
- `roles`
- `permissions`
- `claims`
- `session_id`
- `expires_at`
- `idle_timeout_at`

### `std.auth.jwt`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `jwt.sign(payload)` | `String` | 使用 `[server.auth.jwt]` 默认配置签发 token | Preview |
| `jwt.verify(token)` | `Map` | 校验 token 并返回 claims | Preview |
| `jwt.sign_refresh(payload)` | `String` | 使用 `[server.auth.jwt]` 的 `refresh_ttl_seconds` 签发 refresh token | Preview |
| `jwt.verify_refresh(token)` | `Map` | 校验 refresh token 并返回 claims | Preview |
| `jwt.refresh_pair(refresh_token)` | `Map` | 消费 refresh token 并签发新的 access/refresh pair；旧 refresh token 不能重复使用 | Preview |
| `jwt.revoke_refresh(refresh_token)` | `Null` | 主动失效一个 refresh token，后续再使用会被拒绝 | Preview |
| `jwt.current()` | `Map\|Null` | 返回当前请求 bearer token 的 claims | Preview |
| `jwt.bearer()` | `String\|Null` | 返回当前请求 bearer token 原文 | Preview |

`jwt.sign(payload)` 当前要求 `payload` 至少包含：

- `sub: String`

可选字段：

- `roles: List<String>`
- `permissions: List<String>`
- 其他字段会作为额外 claims 进入 token

返回 claims 当前还会包含：

- `token_use`

`[server.auth.jwt].algorithm` 当前支持 `HS256`、`HS384`、`HS512`。

### `std.auth.password`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `password.hash(password)` | `String` | 生成密码哈希 | Preview |
| `password.verify(password, hash)` | `Bool` | 校验密码与哈希是否匹配 | Preview |

### `std.auth.guard`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `guard.principal()` | `Map\|Null` | 返回当前请求 principal | Preview |
| `guard.authenticated()` | `Bool` | 当前请求是否已认证 | Preview |
| `guard.has_role(role)` | `Bool` | principal 是否拥有某个角色 | Preview |
| `guard.has_any_role(roles)` | `Bool` | principal 是否命中任一角色 | Preview |
| `guard.has_all_roles(roles)` | `Bool` | principal 是否同时拥有全部角色 | Preview |
| `guard.has_permission(permission)` | `Bool` | principal 是否拥有某个权限 | Preview |
| `guard.has_any_permission(permissions)` | `Bool` | principal 是否命中任一权限 | Preview |
| `guard.has_all_permissions(permissions)` | `Bool` | principal 是否同时拥有全部权限 | Preview |
| `guard.require_role(role)` | `Null` | 角色缺失时报错 | Preview |
| `guard.require_any_role(roles)` | `Null` | 任一角色都不满足时报错 | Preview |
| `guard.require_all_roles(roles)` | `Null` | 任一必需角色缺失时报错 | Preview |
| `guard.require_permission(permission)` | `Null` | 权限缺失时报错 | Preview |
| `guard.require_any_permission(permissions)` | `Null` | 任一权限都不满足时报错 | Preview |
| `guard.require_all_permissions(permissions)` | `Null` | 任一必需权限缺失时报错 | Preview |

`guard.principal()` 当前返回结构包含：

- `subject`
- `scheme`
- `roles`
- `permissions`
- `claims`
- `session_id`

### `std.auth.csrf`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `csrf.token()` | `String\|Null` | 返回当前 session 的 CSRF token；无 session 时返回 `Null` | Preview |
| `csrf.rotate()` | `String` | 轮换当前 session 的 CSRF token 并持久化 | Preview |

### `std.path`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `path.join(base, segment)` | `String` | 拼接路径 | Preview |
| `path.basename(path)` | `String` | 获取文件名 | Preview |
| `path.dirname(path)` | `String` | 获取目录部分 | Preview |
| `path.ext(path)` | `String` | 获取扩展名 | Preview |

### `std.str.fmt`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `fmt.pad_left(s, width, char)` | `String` | 左填充 | Preview |
| `fmt.pad_right(s, width, char)` | `String` | 右填充 | Preview |
| `fmt.repeat(s, n)` | `String` | 重复字符串 | Preview |
| `fmt.truncate(s, max_len, suffix)` | `String` | 截断并加后缀 | Preview |
| `fmt.count(s, sub)` | `Int` | 统计子串出现次数 | Preview |

### `std.str.check`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `check.is_empty(s)` | `Bool` | 是否为空字符串 | Preview |
| `check.is_blank(s)` | `Bool` | 是否为空白字符串 | Preview |
| `check.is_numeric(s)` | `Bool` | 是否为纯数字 | Preview |
| `check.is_prefix_of(prefix, s)` | `Bool` | 是否为前缀 | Preview |
| `check.is_suffix_of(suffix, s)` | `Bool` | 是否为后缀 | Preview |

### `std.math.stats`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `stats.sum(list)` | `Int\|Float` | 求和 | Preview |
| `stats.mean(list)` | `Float` | 平均值 | Preview |
| `stats.min_list(list)` | `Any` | 最小值 | Preview |
| `stats.max_list(list)` | `Any` | 最大值 | Preview |
| `stats.median(list)` | `Any` | 中位数 | Preview |
| `stats.clamp_list(list, lo, hi)` | `List` | 限制列表元素范围 | Preview |

### `std.math.trig`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `trig.sin(x)` | `Float` | 正弦 | Preview |
| `trig.cos(x)` | `Float` | 余弦 | Preview |
| `trig.degrees(rad)` | `Float` | 弧度转角度 | Preview |
| `trig.radians(deg)` | `Float` | 角度转弧度 | Preview |
| `trig.tan(x)` | `Float` | 正切 | Preview |
| `trig.sign(n)` | `Int` | 符号：-1 / 0 / 1 | Preview |

### `std.core.iter`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `iter.range(start, end)` | `List` | 生成 `[start, end)` 整数列表 | Preview |
| `iter.range_step(start, end, step)` | `List` | 生成带步长的整数列表 | Preview |

### `std.core.check`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `check.assert(cond, msg)` | `Null` | 断言，失败时抛出异常 | Stable |
| `check.type_of(val)` | `String` | 获取值的类型名 | Stable |
| `check.is_null(val)` | `Bool` | 是否为 `null` | Stable |
| `check.identity(val)` | `Any` | 恒等函数，原样返回 | Preview |

### `std.sqlite`

更完整的 SQL 使用说明、HTTP 中使用、占位符约定与当前边界，见 [sql-database.md](sql-database.md)。
如果你只想先跑通一个数据库例子，先看 [../guide/appendix/sql-quickstart.md](../guide/appendix/sql-quickstart.md)。

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `sqlite.connect(path)` | `Connection` | 打开 SQLite 数据库并返回连接句柄 | Preview |

`Connection` 在 SQLite 路径下支持：

| 方法 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `conn.query(sql, params)` | `List<Map>` | 执行查询并返回结果行列表 | Preview |
| `conn.execute(sql, params)` | `Int` | 执行写操作并返回影响行数 | Preview |
| `conn.close()` | `Null` | 关闭连接句柄 | Preview |

说明：

- SQLite 占位符沿用原生 `?`
- 当前仅支持 `Int` / `Float` / `String` / `Bool` / `Null` 作为绑定参数
- 结果列中的 `BLOB` 当前会报错，不会自动转换

### `std.postgres`

更完整的 SQL 使用说明、HTTP 中使用、占位符约定与当前边界，见 [sql-database.md](sql-database.md)。
如果你只想先跑通一个数据库例子，先看 [../guide/appendix/sql-quickstart.md](../guide/appendix/sql-quickstart.md)。

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `postgres.connect(conninfo)` | `Connection` | 打开 PostgreSQL 连接并返回连接句柄 | Preview |

`Connection` 在 PostgreSQL 路径下支持：

| 方法 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `conn.query(sql, params)` | `List<Map>` | 执行查询并返回结果行列表 | Preview |
| `conn.execute(sql, params)` | `Int` | 执行写操作并返回影响行数 | Preview |
| `conn.close()` | `Null` | 关闭连接句柄 | Preview |

说明：

- PostgreSQL 占位符沿用原生 `$1`, `$2`, ...`
- `conninfo` 接受 PostgreSQL 连接配置字符串，不只限 URL；例如 `postgresql://...` 或 `host=... dbname=... user=...`
- 当前仅支持常见标量参数类型；`Null` 绑定当前会显式报错，要求调用方先提供明确类型
- 不支持的 PostgreSQL 列类型当前会显式报错，不做隐式字符串化
- 如果遇到 `numeric` / `int4` 等兼容边界，优先在 SQL 中显式 `cast`

### `std.time`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `time.now()` | `Int` | 当前 UTC 秒级时间戳 | Stable |
| `time.now_ms()` | `Int` | 当前 UTC 毫秒级时间戳 | Stable |
| `time.format(ts, fmt)` | `String` | 按格式输出时间 | Stable |
| `time.parse(input, fmt)` | `Int` | 按格式解析时间为秒级时间戳 | Preview |
| `time.year(ts)` | `Int` | UTC 年份 | Preview |
| `time.month(ts)` | `Int` | UTC 月份 | Preview |
| `time.day(ts)` | `Int` | UTC 日期 | Preview |
| `time.hour(ts)` | `Int` | UTC 小时 | Preview |
| `time.minute(ts)` | `Int` | UTC 分钟 | Preview |
| `time.second(ts)` | `Int` | UTC 秒 | Preview |
| `time.weekday(ts)` | `String` | UTC 星期名 | Preview |
| `time.add_days(ts, days)` | `Int` | 时间戳平移若干天 | Preview |
| `time.diff_days(lhs, rhs)` | `Int` | 相差天数 | Preview |

### `std.uuid`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `uuid.v4()` | `String` | 生成 UUID v4 | Stable |
| `uuid.is_valid(value)` | `Bool` | 校验 UUID 字符串是否合法 | Stable |

### `std.http`

| 函数 | 返回值 | 说明 | 稳定性 |
|------|--------|------|--------|
| `http.get(url)` | `Map` | 发起 GET 请求 | Stable |
| `http.get(url, headers)` | `Map` | 发起带请求头的 GET 请求 | Preview |
| `http.post(url, body)` | `Map` | 发起 POST 请求，`body` 必须是 `Map` 或 `Json` | Stable |
| `http.post(url, body, headers)` | `Map` | 发起带请求头的 POST 请求 | Preview |
| `http.put(url, body)` | `Map` | 发起 PUT 请求，`body` 必须是 `Map` 或 `Json` | Stable |
| `http.put(url, body, headers)` | `Map` | 发起带请求头的 PUT 请求 | Preview |
| `http.delete(url)` | `Map` | 发起 DELETE 请求 | Preview |
| `http.delete(url, headers)` | `Map` | 发起带请求头的 DELETE 请求 | Preview |
| `http.request(method, url, body, headers)` | `Map` | 发起通用请求 | Preview |

所有 `std.http` 响应统一返回：

```text
{
  "status": Int,
  "body": String,
  "headers": Map
}
```

## 迁移说明

- 本页是当前主线的完整 stdlib 查表入口
- 旧材料见 [../guide/stdlib.md](../guide/stdlib.md)
- Guide 章节只保留学习路径，不再重复完整 API 罗列
