# 开发者贡献指南

本文档面向希望参与 Zun 核心实现、测试和文档维护的开发者。

## 当前仓库结构

```text
.
├── crates/
│   ├── zun-frontend/     # token / lexer / parser / ast / diagnostics
│   ├── zun-runtime/      # runtime / interpreter / module system
│   ├── zun-cli/          # cli / repl / serve / test
│   └── zun-lsp/          # 基于 frontend 的 LSP crate
├── stdlib/                  # 标准库预留目录与贡献骨架
├── examples/                # 示例项目与脚本
├── src/                     # 根 crate 兼容层（对外保留 zun API）
├── docs/                    # 文档
├── tests/                   # 集成测试与 fixtures
└── refactor-plan.md         # 当前标准化重构路线图
```

## 环境要求

- Rust stable
- Cargo

## 常用命令

### 编译

```bash
cargo build
```

### 运行 REPL

```bash
cargo run
```

### 运行脚本

```bash
cargo run -- run path/to/file.zun
```

### 启动服务

```bash
cargo run -- serve
cargo run -- serve path/to/project
cargo run -- serve --routertab
```

### 运行测试入口

```bash
cargo run -- test
cargo run -- test main.zun
```

### 开发基线检查

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

### 运行 benchmark 基线

```bash
cargo bench --bench phase12_baseline -- --noplot
```

## 测试约定

- 集成测试放在 `tests/`
- 可执行输入样例放在 `tests/fixtures/`
- 脚本级测试辅助函数放在 `tests/support/`
- 语义样例放在 `tests/spec/valid/` 与 `tests/spec/invalid/`
- 多文件或模式级测试优先放在 `tests/integration/`
- 修改语言行为时，优先补 fixture 和集成测试
- 修 bug 时必须补对应回归测试
- 修复 bug 时，新增的回归测试应尽量沉淀为最小 `.zun` fixture
- 修改语言行为、CLI 或 `std.*` API 时，要同步检查 `docs/spec/*` 和 `docs/CHANGELOG.md`
- 修改文件、ENV、HTTP、模块加载等副作用能力时，要同步更新 `docs/spec/security-model.md`
- 新增宿主能力时，必须先进入 runtime intrinsic 层，再决定是否暴露为 `std.*`

## 当前模块职责

### `zun-frontend`

负责词法、语法、AST、诊断等前端能力。

### `zun-runtime`

负责值系统、求值、执行、项目系统和模块解析。
同时负责 runtime intrinsic 层，用于统一承接文件系统、环境变量、配置等宿主能力。

### `zun-cli`

负责命令行入口、REPL、serve/test 模式。

### `zun-lsp`

负责最小语言服务能力，并直接依赖 frontend。

当前最小交付：

- 能接收 `initialize`
- 能处理 `didOpen` / `didChange`
- 能返回 syntax diagnostics

## 贡献建议

### 改语法前先确认三件事

1. 是否已有文档对应位置
2. 是否已有测试覆盖当前行为
3. 是否会影响现有 CLI、模块、HTTP 或类型语义

### 提交代码时建议保持单一主题

例如：

- 一个 PR 只做 parser 重构
- 一个 PR 只补测试
- 一个 PR 只修某个运行时错误

### 修改后至少确认

- 能编译
- 测试通过
- 文档同步
- 如果有兼容性影响，已附 RFC 或兼容说明

## 当前阶段的重点

仓库正在按 `refactor-plan.md` 推进标准化。当前优先级最高的是：

- 建立工程基线
- 收敛运行时上下文
- 统一错误系统
- 建立测试矩阵

在这些基础稳定之前，不建议把大规模新语法和大规模新标准库功能混在同一个改动里。

如果要新增文件、ENV、配置、时间、随机数、网络等宿主能力，推荐顺序是：

1. 先设计 intrinsic
2. 先补 runtime 测试
3. 再决定是否需要 `std.*` 封装
