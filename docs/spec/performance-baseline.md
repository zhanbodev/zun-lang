# Performance Baseline

本页定义 Zun 当前的最小 benchmark 基线。

Phase 12 的目标不是强行做性能优化，而是建立一套可重复运行、可横向比较、可记录回归的最小基线。

## 当前基线框架

当前采用：

- `criterion`

选择理由：

- 依赖链较轻
- 能稳定重复运行
- 适合建立最小基线
- 方便后续重构前后做横向比较

## 当前 benchmark 覆盖范围

第一批 benchmark 覆盖：

- lexer 处理小型脚本
- parser 处理小型脚本
- runtime 执行基础表达式
- 函数调用
- 模块加载
- HTTP 路由 handler 执行最小样例

对应实现位于：

- `benches/phase12_baseline.rs`

固定输入位于：

- `benches/fixtures/`

## 固定样例说明

每个样例都应记录：

- 输入规模
- 使用场景
- 预期关注点

当前样例概览：

- `lexer_parser_small.zun`
  - 输入规模：小型脚本
  - 使用场景：frontend 处理
  - 关注点：lexer / parser 基础吞吐
- `runtime_expression.zun`
  - 输入规模：单表达式脚本
  - 使用场景：runtime 基础执行
  - 关注点：求值路径
- `function_call.zun`
  - 输入规模：单函数定义与调用
  - 使用场景：函数执行
  - 关注点：调用与返回
- `modules/main.zun`
  - 输入规模：最小模块装载
  - 使用场景：模块解析与加载
  - 关注点：路径解析、文件读取、模块函数装载
- `http/health_route.zun`
  - 输入规模：最小路由
  - 使用场景：HTTP handler 执行
  - 关注点：handler dispatch

## 运行方式

执行基线：

```bash
cargo bench --bench phase12_baseline -- --noplot
```

如果只想先确认 benchmark 能编译：

```bash
cargo bench --bench phase12_baseline --no-run
```

## 基线记录模板

每次记录至少包含：

- 输入规模
- 执行时间
- 测试环境说明
- 对应版本号

建议记录模板：

```md
## Benchmark Run

- Version: 0.1.0
- Date: YYYY-MM-DD
- Machine:
- Rust:
- Command: `cargo bench --bench phase12_baseline -- --noplot`

### Results

- lexer / small_script:
- parser / small_script:
- runtime / execute_expression:
- runtime / function_call:
- runtime / module_load:
- runtime / http_handler:
```

## 运行规则

- benchmark 不作为当前主线合并门槛
- 大重构前后必须运行一次
- parser/runtime 重构 PR 要附带“无明显性能退化”说明

## 性能目标

- benchmark 的用途是观察退化，不是当前阶段强行做性能优化
- 允许小幅波动
- 不允许长期无记录退化
