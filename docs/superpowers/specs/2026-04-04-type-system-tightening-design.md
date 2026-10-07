# `$Type` 名义类型强化设计

日期：2026-04-04

## 背景

当前 Zun 已支持 `$Type` 声明、`TypeName { ... }` 构造、以及函数 / HTTP handler 的 `-> T` / `-> List<T>` 返回类型校验。

但现状仍有明显缺口：

- `$Type` 声明目前更像“注册一个名字 + shape 描述”，而不是强约束的自定义类型
- `T { ... }` 构造时只检查类型名是否存在，不检查未知字段、缺失必填字段、字段值类型
- `obj.field = ...` 对用户类型字段只做有限校验，且允许未声明字段漂移
- 普通变量类型注解系统不理解用户类型，`TypedInstance` 也被视为 `Dynamic`
- `$Type` 字段语法只支持单个标识符，无法稳定表达 `List<T>` 或复杂聚合

结果是：用户即使写了 `$Type`，在大量实际路径上也感受不到强约束，`$Type` 容易退化为“可写可不写”的说明性结构。

## 目标

把 `$Type` 明确收敛为 Zun 的一类特色 runtime 自定义类型：

- 它不是结构体别名
- 它不是按形状自动匹配的 Map
- 它必须通过显式 `TypeName { ... }` 构造
- 一旦声明返回 `-> T` 或 `-> List<T>`，就必须返回对应的 typed instance
- 同一套规则在普通函数、HTTP 超函数家族、`run` 模式、`serve` 模式下全部一致

## 非目标

本设计不把 `$Type` 变成 class 系统，也不引入方法、继承、getter/setter。

本设计第一阶段不解决：

- 参数类型注解
- 自动把裸 `Map` / `JSON` 转换为 `$Type`
- 全量静态类型检查
- 泛型系统

## 核心语义

### 1. `$Type` 是名义类型，不是形状类型

`$Type User { ... }` 定义的是一种名义 runtime 类型 `User`。

这意味着：

- `User { id: 1, name: "A" }` 是 `User`
- `{ id: 1, name: "A" }` 只是 `Map`
- `$JSON { "id": 1, "name": "A" }` 只是 `Json`
- 就算字段完全一致，`Map` / `Json` 也不能冒充 `User`

### 2. 显式构造是唯一稳定入口

如果代码要得到 `User`，必须显式写成：

```zun
$Type User {
    id: Int
    name: String
}

$ user = User {
    id: 1,
    name: "Alice",
};
```

不允许在返回边界或赋值过程中隐式把 `Map` / `Json` 自动收紧成 `User`。

### 3. 返回类型必须强校验

如果普通函数或 HTTP 超函数声明：

- `-> User`
- `-> List<User>`

则 runtime 必须严格校验实际返回值。

校验规则：

- `-> User` 必须是 `TypedInstance(User)`
- `-> List<User>` 必须是 `List`，且每个元素都必须是 `TypedInstance(User)`
- 这套规则在 `run`、`serve`、`test` 模式一致

### 4. `$Type` 字段支持用户类型名

第一阶段允许 `$Type` 字段类型直接引用另一个已声明用户类型，例如：

```zun
$Type Post {
    id: Int
    title: String
    author: User
}
```

但第一阶段仍不扩展 `$Type` 字段里的 `List<User>` 语法；这部分在第二阶段处理。

## 分阶段设计

## 第一阶段：名义类型最小闭环

第一阶段目标是让 `$Type` 在最关键入口立即产生约束：

- 声明时注册 shape
- 构造时严格校验
- 字段赋值时严格校验
- 返回边界统一强校验

### 第一阶段范围

包括：

- `T { ... }` 构造严格校验
- `typed_instance.field = ...` 严格校验
- 普通函数 `-> T` / `-> List<T>` 强校验
- HTTP 超函数 `-> T` / `-> List<T>` 强校验
- 用户类型字段引用另一个用户类型名，例如 `author: User`

不包括：

- 变量注解 `: User`
- 变量注解 `: List<User>`
- list 容器写入持续校验
- `$Type` 字段 `List<T>` 语法
- 更复杂的嵌套泛型表达

### 第一阶段构造规则

执行 `User { ... }` 时：

1. 类型名必须已在 runtime type registry 中注册
2. 输入字段不得包含未声明字段
3. 所有非可选字段都必须出现
4. 每个字段值必须匹配声明类型
5. 若字段声明为另一个用户类型名，则值必须是对应 typed instance
6. `@HIDE` 仅影响内部存储键和输出隐藏行为，不影响字段类型匹配

构造成功后，才生成 `TypedInstance { type_name: "User", ... }`。

### 第一阶段字段赋值规则

执行 `user.name = ...` 或 `post.author = ...` 时：

1. 接收方必须是 typed instance
2. 字段必须存在于该类型声明中
3. 新值必须匹配字段声明类型
4. 不允许通过赋值动态增加新字段

这条规则的意义是防止 typed instance 在运行过程中“漂移”出声明 shape。

### 第一阶段返回值规则

普通函数与 HTTP 超函数共用同一套返回校验器。

校验规则：

- 基础类型保持现有逻辑
- `List<T>` 继续递归校验元素
- 当 `T` 为用户类型名时：
  - runtime 必须确认该类型已注册
  - 实际值必须是同名 typed instance
- 不做 shape-based fallback
- 不做 `Map -> T` 自动转换

### 第一阶段错误模型

推荐错误格式统一为“类型 + 字段路径 + 实际类型”的高信息量报错。

典型错误示例：

- `type 'User' is not defined`
- `type 'User' has no field 'nickname'`
- `type 'User' requires field 'name'`
- `field 'age' of type 'User' expects 'Int', got 'String'`
- `field 'author' of type 'Post' expects 'User', got 'Map'`
- `function 'get_user' expects return type 'User' but got 'Map'`
- `http handler 'list_users' expects return type 'List<User>' but item 2 got 'Map'`

### 第一阶段实现影响

需要调整的核心组件：

- `crates/zun-runtime/src/interpreter/eval/constructors.rs`
  - 为 struct constructor 增加统一字段校验
- `crates/zun-runtime/src/interpreter/exec/variables.rs`
  - 收紧 typed instance 字段赋值
- `crates/zun-runtime/src/interpreter/exec/functions.rs`
  - 复用并补强用户类型 / `List<T>` 返回值校验
- `crates/zun-runtime/src/interpreter/exec/http.rs`
  - 继续走共享返回值校验路径
- `crates/zun-runtime/src/runtime/context.rs`
  - 继续作为 `$Type` registry 的统一来源

建议新增一个共享 helper，例如 `validate_value_against_type(...)`，供：

- 构造器
- 字段赋值
- 返回值校验

统一使用，避免逻辑散落成三套。

## 第二阶段：全链路约束增强

第二阶段在第一阶段语义上继续扩展，让 `$Type` 不只在边界严格，也能在变量和容器层持续生效。

### 第二阶段范围

- 变量注解支持 `User`
- 变量注解支持 `List<User>`
- 常量注解支持用户类型
- list 索引写入、后续 append/push 类操作的元素校验
- `$Type` 字段类型语法支持 `List<T>`
- `$Type` 字段支持嵌套自定义类型与可选组合

### 第二阶段语法扩展

当前 `$Type` 字段只支持：

```zun
author: User
author: User?
```

第二阶段扩展到：

```zun
items: List<OrderItem>
items: List<OrderItem>?
author: User
author: User?
```

为此需要扩展：

- parser 字段类型语法
- AST 中 `TypeField.type_name: String` 的表达能力
- runtime 对类型表达式的解析与递归验证

### 第二阶段变量与容器约束

支持：

```zun
$ user: User = User { ... };
$ posts: List<Post> = [ ... ];
```

并要求后续赋值持续符合声明类型。

示例：

```zun
posts[0] = Post { ... };   // 合法
posts[0] = { ... };        // 非法
```

如果后续 list builtins 提供 push/append/extend，一样要走同一套元素类型校验。

## 架构原则

实现时建议坚持以下原则：

### 1. 单一类型校验入口

不要把类型判断散落在构造器、赋值器、返回值校验器里分别写分支。

应抽出统一入口：

- 输入：期望类型表达式、值、上下文、错误路径
- 输出：成功或结构化错误

### 2. 名义优先，不做自动收紧

任何 `Map` / `Json` 到 `$Type` 的转换都必须显式，不做隐式兜底。

### 3. 模式一致

`run`、`serve`、`test` 语义必须一致，不能让 HTTP 路径比普通函数更严格或更宽松。

### 4. 先闭环，再扩展

先把“显式构造 + 字段不可漂移 + 边界强校验”落稳，再扩语法和变量系统。

## 测试策略

第一阶段至少需要覆盖三类测试。

### 构造测试

- 正确构造 typed instance
- 缺失必填字段失败
- 未知字段失败
- 基础字段类型错误失败
- 用户类型字段错误失败
- `@HIDE` 字段构造成功且保持现有隐藏行为

### 变更测试

- 正确字段赋值成功
- 字段类型错误失败
- 未声明字段赋值失败
- 用户类型字段赋值成功 / 失败各一组

### 边界测试

- 普通函数 `-> User` 成功 / 失败
- 普通函数 `-> List<User>` 成功 / 失败
- HTTP handler `-> User` 成功 / 失败
- HTTP handler `-> List<User>` 成功 / 失败
- linked module / imported type 场景下行为一致

## 风险与迁移影响

第一阶段会让一批过去“能跑但不严格”的代码开始失败，包括：

- 少字段构造 typed instance
- 构造时带未声明字段
- 给 typed instance 动态塞新字段
- 返回形状相似但不是 typed instance 的值

这是预期中的语义收紧，不建议为主线保留宽松模式。

如果保留宽松兼容开关，`$Type` 的定位会继续摇摆，用户仍会把它视为可选提示，而不是必须认真使用的类型系统。

## 推荐实施顺序

1. 抽出统一值类型校验 helper
2. 接入构造器校验
3. 接入字段赋值校验
4. 统一普通函数 / HTTP 返回值校验逻辑
5. 增补 spec 与 integration tests
6. 第二阶段再扩 parser / AST / 变量注解 / 容器约束

## 验收标准

第一阶段完成后，以下结论必须成立：

- `$Type` 不再只是文档标签，构造阶段就产生约束
- 普通函数和 HTTP 超函数家族使用同一套用户类型返回规则
- `run` / `serve` / `test` 行为一致
- 裸 `Map` / `Json` 不能冒充 `$Type`
- typed instance 不能在运行时漂移出声明 shape

## 结论

本设计采用“两阶段推进”：

- 第一阶段先完成名义类型最小闭环
- 第二阶段再补变量系统、容器系统、以及 `$Type` 字段泛型表达

这样既能快速提升 `$Type` 的真实约束力，也能为后续更完整的 Zun 自定义类型系统保留清晰的演进路径。
