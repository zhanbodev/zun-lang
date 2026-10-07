# Common Errors

## 解析阶段

- 缺少分号
- 表达式右侧缺失
- 括号或大括号不匹配
- 语法还没进入执行阶段，因此不能靠 `$catch` 处理

## 运行阶段

- 未定义变量
- 类型不匹配
- 模块未找到
- 文件读取失败
- 环境变量不存在
- HTTP handler 返回类型不匹配

## 快速排查顺序

1. 先看是 parse error 还是 runtime error
2. 再看报错位置
3. 缩成最小 `.zun` 文件
4. 用 REPL 或 `zun test --route ...` 复现

## 交叉阅读

- 主线说明：[../10-error-handling.md](../10-error-handling.md)
- 查表说明：[../../reference/errors.md](../../reference/errors.md)
