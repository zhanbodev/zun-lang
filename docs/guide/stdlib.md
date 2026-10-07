# 标准库参考

> **Status: Migration Source**
>
> 这是一页迁移期旧参考文档。
> 当前主线的总览请读 [13-stdlib-overview.md](13-stdlib-overview.md)，完整查表请读 [../reference/stdlib-api.md](../reference/stdlib-api.md)。
> 本页仍保留的独特价值是旧版 API 罗列，可作为迁移素材，但不应继续作为主线入口。

Zun 的标准库分为两层：

- **内置方法**：所有值都自带的方法，无需 import
- **标准库模块**：通过 `$mod std.xxx` 导入，分为 native（Rust 实现）和纯 .zun 实现两种

---

## 内置方法

### String

```zun
$ s = "Hello, World";
```

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `s.len()` | `Int` | 字符串长度 |
| `s.upper()` | `String` | 转大写 |
| `s.lower()` | `String` | 转小写 |
| `s.trim()` | `String` | 去除首尾空白 |
| `s.contains(sub)` | `Bool` | 是否包含子串 |
| `s.starts_with(prefix)` | `Bool` | 是否以 prefix 开头 |
| `s.ends_with(suffix)` | `Bool` | 是否以 suffix 结尾 |
| `s.replace(old, new)` | `String` | 替换所有匹配 |
| `s.split(sep)` | `List` | 按分隔符拆分 |
| `s.slice(start, end)` | `String` | 切片，支持负索引 |
| `s.to_int()` | `Int` | 转整数 |
| `s.to_float()` | `Float` | 转浮点数 |
| `s.to_bool()` | `Bool` | 转布尔值 |
| `s.type()` | `String` | 返回 `"String"` |

---

### List

```zun
$ nums = [3, 1, 4, 1, 5];
```

**读操作**

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `list.len()` | `Int` | 列表长度 |
| `list.contains(x)` | `Bool` | 是否包含元素 |
| `list.index_of(x)` | `Int` | 第一次出现的索引，不存在返回 -1 |
| `list.last_index_of(x)` | `Int` | 最后一次出现的索引 |
| `list.slice(start, end)` | `List` | 切片，支持负索引 |
| `list.first()` | `Any` | 第一个元素，空列表返回 `null` |
| `list.last()` | `Any` | 最后一个元素，空列表返回 `null` |
| `list.is_empty()` | `Bool` | 是否为空 |
| `list.flatten()` | `List` | 展平一层嵌套 |
| `list.unique()` | `List` | 去重，保持顺序 |
| `list.count(x)` | `Int` | 统计元素出现次数 |
| `list.join(sep)` | `String` | 用分隔符连接元素 |
| `list.type()` | `String` | 返回 `"List"` |

**写操作（原地修改）**

| 方法 | 说明 |
|------|------|
| `list.push(x)` | 末尾追加元素 |
| `list.pop()` | 移除并返回末尾元素 |
| `list.reverse()` | 原地反转 |
| `list.sort()` | 原地升序排序（纯数字或纯字符串） |
| `list.sort_desc()` | 原地降序排序 |
| `list.remove_at(i)` | 移除指定索引的元素 |
| `list.insert(i, x)` | 在指定索引插入元素 |
| `list.clear()` | 清空列表 |

---

### Map

```zun
$ user = {"name": "Alice", "age": 30};
```

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `map.len()` | `Int` | 键值对数量 |
| `map.keys()` | `List` | 所有键 |
| `map.values()` | `List` | 所有值 |
| `map.contains_key(key)` | `Bool` | 是否包含某键 |
| `map.remove(key)` | `Map` | 删除键（原地） |
| `map.type()` | `String` | 返回 `"Map"` |

---

### Int / Float

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `n.to_str()` | `String` | 转字符串 |
| `n.to_int()` | `Int` | 转整数（Float 截断） |
| `n.to_float()` | `Float` | 转浮点数 |
| `n.to_bool()` | `Bool` | 转布尔值（仅 Int，0→false） |
| `n.type()` | `String` | `"Int"` 或 `"Float"` |

---

## Native 标准库模块

### std.str

```zun
$mod std.str;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `str.trim(s)` | `String` | 去除首尾空白 |
| `str.trim_start(s)` | `String` | 去除开头空白 |
| `str.trim_end(s)` | `String` | 去除末尾空白 |
| `str.split(s, sep)` | `List` | 按分隔符拆分 |
| `str.join(list, sep)` | `String` | 用分隔符连接列表 |
| `str.replace(s, from, to)` | `String` | 替换子串 |
| `str.contains(s, sub)` | `Bool` | 包含检查 |
| `str.starts_with(s, prefix)` | `Bool` | 前缀检查 |
| `str.ends_with(s, suffix)` | `Bool` | 后缀检查 |
| `str.to_upper(s)` | `String` | 转大写 |
| `str.to_lower(s)` | `String` | 转小写 |
| `str.len(s)` | `Int` | 长度 |
| `str.parse_int(s)` | `Int` | 解析为整数 |
| `str.parse_float(s)` | `Float` | 解析为浮点数 |

---

### std.math

```zun
$mod std.math;
```

**基础**

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `math.sqrt(x)` | `Float` | 平方根 |
| `math.pow(base, exp)` | `Float` | 幂运算 |
| `math.abs(x)` | `Int\|Float` | 绝对值 |
| `math.floor(x)` | `Int` | 向下取整 |
| `math.ceil(x)` | `Int` | 向上取整 |
| `math.round(x)` | `Int` | 四舍五入 |
| `math.min(a, b)` | `Int\|Float` | 较小值（同类型） |
| `math.max(a, b)` | `Int\|Float` | 较大值（同类型） |
| `math.clamp(x, min, max)` | `Int\|Float` | 限制范围 |
| `math.hypot(x, y)` | `Float` | 直角三角形斜边 |

**常数**

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `math.pi()` | `Float` | π ≈ 3.14159 |
| `math.e()` | `Float` | e ≈ 2.71828 |

**三角函数**

| 函数 | 说明 |
|------|------|
| `math.sin(x)` | 正弦 |
| `math.cos(x)` | 余弦 |
| `math.tan(x)` | 正切 |
| `math.asin(x)` | 反正弦 |
| `math.acos(x)` | 反余弦 |
| `math.atan(x)` | 反正切 |
| `math.atan2(y, x)` | 四象限反正切 |

**对数 / 指数**

| 函数 | 说明 |
|------|------|
| `math.exp(x)` | e^x |
| `math.ln(x)` | 自然对数 |
| `math.log(x, base)` | 任意底对数 |
| `math.log2(x)` | 以 2 为底 |
| `math.log10(x)` | 以 10 为底 |

**随机数**

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `math.random()` | `Float` | [0.0, 1.0) 随机浮点数 |
| `math.random_int(min, max)` | `Int` | [min, max] 随机整数 |

---

### std.fs

```zun
$mod std.fs;
```

**文件操作**

| 函数 | 说明 |
|------|------|
| `fs.read_text(path)` | 读取文件全部内容 → `String` |
| `fs.read_lines(path)` | 按行读取 → `List` |
| `fs.write_text(path, content)` | 覆盖写入文件 |
| `fs.append_text(path, content)` | 追加写入文件 |
| `fs.delete(path)` | 删除文件 |
| `fs.exists(path)` | 文件/目录是否存在 → `Bool` |
| `fs.size(path)` | 文件大小（字节）→ `Int` |
| `fs.is_dir(path)` | 是否为目录 → `Bool` |
| `fs.copy(src, dst)` | 复制文件 |
| `fs.rename(src, dst)` | 移动/重命名文件或目录 |

**目录操作**

| 函数 | 说明 |
|------|------|
| `fs.list(path)` | 列出目录内容（文件名列表）→ `List` |
| `fs.mkdir(path)` | 创建目录 |
| `fs.mkdir_all(path)` | 递归创建目录（mkdir -p） |
| `fs.rmdir(path)` | 删除空目录 |

---

### std.json

```zun
$mod std.json;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `json.parse(str)` | `Map\|List` | 解析 JSON 字符串 |
| `json.stringify(value)` | `String` | 转为 JSON 字符串 |
| `json.pretty(value)` | `String` | 美化 JSON 输出 |
| `json.get(obj, key)` | `Any` | 获取对象中某键的值 |
| `json.has(obj, key)` | `Bool` | 键是否存在 |
| `json.keys(obj)` | `List` | 所有键 |
| `json.values(obj)` | `List` | 所有值 |
| `json.set(obj, key, value)` | `Map` | 设置键值（返回新对象） |
| `json.delete(obj, key)` | `Map` | 删除键（返回新对象） |
| `json.merge(a, b)` | `Map` | 合并两个对象（b 覆盖 a） |

---

### std.env

```zun
$mod std.env;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `env.get(key)` | `String` | 获取环境变量（不存在则报错） |
| `env.get_or(key, default)` | `String` | 获取环境变量或返回默认值 |
| `env.has(key)` | `Bool` | 检查环境变量是否存在 |
| `env.all()` | `Map` | 返回所有环境变量 |
| `env.set(key, value)` | — | 设置环境变量（仅非 serve 模式） |
| `env.remove(key)` | — | 删除环境变量（仅非 serve 模式） |

---

## 纯 Zun 标准库模块

### std.path

```zun
$mod std.path;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `path.join(base, segment)` | `String` | 拼接路径 |
| `path.basename(path)` | `String` | 获取文件名 |
| `path.dirname(path)` | `String` | 获取目录部分 |
| `path.ext(path)` | `String` | 获取扩展名（含 `.`） |

---

### std.str.fmt

```zun
$mod std.str.fmt;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `fmt.pad_left(s, width, char)` | `String` | 左填充至指定宽度 |
| `fmt.pad_right(s, width, char)` | `String` | 右填充至指定宽度 |
| `fmt.repeat(s, n)` | `String` | 重复字符串 n 次 |
| `fmt.truncate(s, max_len, suffix)` | `String` | 截断并加后缀 |
| `fmt.count(s, sub)` | `Int` | 统计子串出现次数 |

---

### std.str.check

```zun
$mod std.str.check;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `check.is_empty(s)` | `Bool` | 是否为空字符串 |
| `check.is_blank(s)` | `Bool` | 是否为空白字符串 |
| `check.is_numeric(s)` | `Bool` | 是否为纯数字 |
| `check.is_prefix_of(prefix, s)` | `Bool` | prefix 是否为 s 的前缀 |
| `check.is_suffix_of(suffix, s)` | `Bool` | suffix 是否为 s 的后缀 |

---

### std.math.stats

```zun
$mod std.math.stats;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `stats.sum(list)` | `Int\|Float` | 求和 |
| `stats.mean(list)` | `Float` | 平均值 |
| `stats.min_list(list)` | `Any` | 最小值 |
| `stats.max_list(list)` | `Any` | 最大值 |
| `stats.median(list)` | `Any` | 中位数 |
| `stats.clamp_list(list, lo, hi)` | `List` | 将列表元素限制在 [lo, hi] 内 |

---

### std.math.trig

```zun
$mod std.math.trig;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `trig.degrees(rad)` | `Float` | 弧度转角度 |
| `trig.radians(deg)` | `Float` | 角度转弧度 |
| `trig.tan(x)` | `Float` | 正切 |
| `trig.sign(n)` | `Int` | 符号：-1 / 0 / 1 |

---

### std.core.iter

```zun
$mod std.core.iter;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `iter.range(start, end)` | `List` | 生成 `[start, end)` 整数列表 |
| `iter.range_step(start, end, step)` | `List` | 生成带步长的整数列表 |

---

### std.core.check

```zun
$mod std.core.check;
```

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `check.assert(cond, msg)` | — | 断言，失败时抛出异常 |
| `check.type_of(val)` | `String` | 获取值的类型名 |
| `check.is_null(val)` | `Bool` | 是否为 null |
| `check.identity(val)` | `Any` | 恒等函数，原样返回 |
