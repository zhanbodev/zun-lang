# Module Specification

本页记录 Zun 当前语言模块系统和 HTTP 路由挂载的边界。

## 模块路径

- 模块路径使用点号分隔：`foo.bar.baz`
- 解析时会被转换为文件系统路径：`foo/bar/baz.zun`
- 模块名默认就是文件名

## 解析顺序

对于普通模块，当前搜索顺序是：

1. 当前文件所在目录
2. 当前项目根目录
3. 项目根目录下的 `modules/`
4. 如果 `package.toml` 声明了依赖，再查 `deps/`

`std.*` 只解析到 `stdlib/`：

- `std.io` -> `stdlib/io.zun`
- `std.json` -> `stdlib/json.zun`

## 语言模块：`$mod`

支持两种形式：

```zun
$mod helper;
$mod math.*;
```

规则：

- `$mod a.b;` 导入单个模块
- `$mod a.*;` 导入目录下的直接子模块
- 导入后通过文件名命名空间访问
- 不会把模块函数平铺到全局函数表

示例：

```zun
$mod helper;
$>> helper.answer();
```

### 导出与私有

- `$fn` 默认公开
- `_$fn` 默认私有
- `$mod` 只暴露公开函数
- 变量、常量、HTTP 路由、`$main` 不属于 `$mod` 导出内容

## HTTP 路由挂载：`$HTTP(...).link("...")`

`.link()` 只用于 HTTP 子系统：

- 读取目标文件中的 HTTP 路由
- 支持顶层 `$GET/$POST/...`
- 支持 `$HTTP { ... }` 块内路由
- 不导入普通函数、变量、常量

示例：

```zun
$HTTP("/v1/api").link("routers.api");
```

## 项目根

- 如果路径向上能找到 `package.toml`，项目根就是该文件所在目录
- 如果找不到 `package.toml`，项目根就是当前脚本所在目录

## 最小正确示例

```zun
$mod helper;
$>> helper.answer();
```

对应测试：

- `tests/spec/valid/modules/relative_link.zun`

## 最小错误示例

```zun
$mod missing.module;
```

对应测试：

- `tests/spec/invalid/modules/missing_module.zun`

## 对应测试位置

- `tests/spec/valid/modules/relative_link.zun`
- `tests/spec/invalid/modules/missing_module.zun`
