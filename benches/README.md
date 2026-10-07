# Benchmark Fixtures

本目录保存 Phase 12 的固定 benchmark 输入样例。

这些样例的目标不是覆盖所有语言能力，而是提供一组稳定、可重复、可横向比较的最小基线。

## 当前样例

- `lexer_parser_small.zun`
  - 场景：小型脚本词法和语法处理
  - 关注点：lexer / parser 基础吞吐
- `runtime_expression.zun`
  - 场景：基础表达式执行
  - 关注点：runtime 基础求值
- `function_call.zun`
  - 场景：函数声明与调用
  - 关注点：函数调用链路
- `modules/main.zun`
  - 场景：模块加载与 `$mod`
  - 关注点：模块解析、文件读取、模块命名空间装载
- `http/health_route.zun`
  - 场景：最小 HTTP 路由注册与 handler 执行
  - 关注点：HTTP route handler 执行路径

## 固定输入原则

- 样例一旦进入基线，尽量不要频繁改内容
- 如果确实需要改动，必须同步更新 `docs/spec/performance-baseline.md`
- 大重构前后应对同一组输入重复执行 benchmark
