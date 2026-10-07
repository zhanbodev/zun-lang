# RFC Process

以下内容建议走 RFC：

- 新语法
- 标准库公共 API
- 模块系统规则变更
- 兼容性策略调整
- 任何 breaking change

## 强制要求

如果一个改动会造成 breaking change，则必须至少满足以下之一：

- 提交 RFC
- 附带兼容说明文档

同时必须更新：

- `docs/spec/*`
- `docs/CHANGELOG.md`
- 对应测试

## 最小 RFC 模板

1. 背景
2. 问题陈述
3. 提案内容
4. 兼容性影响
5. 测试与文档计划
6. 未决问题
