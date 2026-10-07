# Module Resolution

当前模块解析主路径：

1. 当前文件所在目录
2. 项目根目录
3. 项目根目录下的 `modules/`
4. `stdlib/`，仅用于 `std.*`
5. `deps/`，仅用于已声明依赖

普通模块示例：

```zun
$mod shared.helper;
```

标准库示例：

```zun
$mod std.math;
```
