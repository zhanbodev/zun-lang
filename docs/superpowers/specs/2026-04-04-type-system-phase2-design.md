# `$Type` 阶段 2 扩展设计

日期：2026-04-04

## 背景

`$Type` 第一阶段已经完成名义类型最小闭环：

- `TypeName { ... }` 构造会做严格 shape 校验
- typed instance 字段赋值会做严格类型校验
- 普通函数与 HTTP 超函数的 `-> T` / `-> List<T>` 返回值会做统一强校验
- 裸 `Map` / `Json` 不会再隐式冒充 `$Type`

但第一阶段仍有三个明显缺口：

- `$Type` 字段语法不能表达 `List<T>`
- 变量 / 常量类型注解仍不支持用户类型与 `List<T>`
- list 容器在变量层缺少持续约束，`items[0] = ...` 仍可能绕过类型系统

阶段 2 的目标，就是在不引入完整静态类型系统的前提下，把 `$Type` 扩展成更可组合的 runtime 自定义类型体系。

## 目标

阶段 2 明确实现以下能力：

- `$Type` 字段支持 `List<T>`
- `$Type` 字段支持 `T?` 和 `List<T>?`
- 变量 / 常量注解支持 `T` 和 `List<T>`
- list 索引赋值会按声明元素类型持续校验
- `?` 在阶段 2 明确允许 `null`

## 非目标

本设计不包括：

- 参数类型注解
- 所有 list methods 的写入校验，例如 `push` / `append`
- `List<User?>` 这类“可空元素”语义
- 联合类型、泛型扩展、复杂类型运算
- 自动把 `Map` / `Json` 转换为 `$Type`

## 核心语义

### 1. 引入结构化类型表达 `TypeExpr`

阶段 2 不再继续把类型系统建立在散落的字符串解析上。

建议引入一个小型类型表达模型：

```rust
pub enum TypeExpr {
    Named(String),
    List(Box<TypeExpr>),
    Optional(Box<TypeExpr>),
}
```

语义如下：

- `Named("User")` 表示 `User`
- `List(Named("User"))` 表示 `List<User>`
- `Optional(Named("User"))` 表示 `User?`
- `Optional(List(Named("User")))` 表示 `List<User>?`

### 2. `?` 的语义

阶段 2 明确将 `?` 定义为：

- 字段可以缺失
- 字段如果出现，值可以是 `null`
- 或者值必须满足底层类型表达

例如：

```zun
$Type UserProfile {
    nickname: String?
    posts: List<Post>?
}
```

意味着：

- `nickname` 可以不存在
- `nickname` 可以是 `null`
- `nickname` 也可以是字符串
- `posts` 可以不存在
- `posts` 可以是 `null`
- `posts` 也可以是 `List<Post>`

### 3. `@HIDE` 与 `?` 不冲突

- `@HIDE` 表示字段存在，但在外部展示 / 序列化时隐藏
- `?` 表示字段可以缺失，且出现时可为 `null`

例如：

```zun
$Type User {
    @HIDE token: String?
}
```

表示：

- `token` 可以没有
- `token` 可以是 `null`
- `token` 可以是字符串
- 一旦输出到外部，它仍按隐藏字段处理

### 4. 本阶段不支持可空元素

阶段 2 明确不支持：

```zun
items: List<User?>
```

原因：

- 这会引入“元素可空”语义，增加 parser 和 validator 复杂度
- 当前主线更需要先稳定 optional container，而不是 optional element

因此本阶段只允许：

- `List<User>`
- `List<User>?`

不允许：

- `List<User?>`
- `List<List<User?>?>`

## 范围

### 阶段 2 包括

- `$Type` 字段解析为 `TypeExpr`
- 变量 / 常量注解解析为 `TypeExpr`
- runtime validator 按 `TypeExpr` 递归校验
- `TypeEnv` 从基础 `ValueType` 升级为结构化类型表达
- list 索引赋值按 `List<T>` 元素类型校验

### 阶段 2 不包括

- 函数参数类型注解
- list methods 的所有写路径
- 对所有旧字符串类型节点做全仓库立即迁移

## 架构设计

## 1. 前端 AST 调整

### `$Type` 字段

当前：

```rust
pub struct TypeField {
    pub name: String,
    pub type_name: String,
    pub optional: bool,
    pub hidden: bool,
}
```

阶段 2 改为：

```rust
pub struct TypeField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub hidden: bool,
}
```

这样：

- `author: User` -> `Named("User")`
- `author: User?` -> `Optional(Named("User"))`
- `items: List<Post>` -> `List(Named("Post"))`
- `items: List<Post>?` -> `Optional(List(Named("Post")))`

### 变量 / 常量注解

当前：

```rust
pub type_annotation: Option<String>
```

阶段 2 改为：

```rust
pub type_annotation: Option<TypeExpr>
```

## 2. Parser 设计

阶段 2 只需要一个很小的类型表达解析器。

允许：

- `Ident`
- `Ident?`
- `List<Ident>`
- `List<Ident>?`
- `List<List<Ident>>`
- `List<List<Ident>>?`

解析策略：

1. 先解析主体类型
2. 如果看到 `List<...>`，递归解析内部类型
3. 主体解析完后，再看尾部是否有 `?`
4. `?` 作用于整个前面类型表达

例如：

- `List<User>?` -> `Optional(List(Named("User")))`
- `User?` -> `Optional(Named("User"))`

阶段 2 明确拒绝：

- `List<User?>`
- `??`
- 裸 `List`

## 3. Runtime 类型表达桥接

不要求本阶段一次把所有返回类型 AST 也改为 `TypeExpr`。

为了控制改动面，建议：

- `$Type` 字段、变量注解、常量注解直接持有 `TypeExpr`
- 函数 / HTTP 返回类型暂时仍可保持字符串
- 进入 runtime validator 时，再把返回类型字符串桥接解析成 `TypeExpr`

这样可以避免为了阶段 2 一次性重写所有现有签名 AST。

## 4. `TypeEnv` 升级

当前：

```rust
pub type TypeEnv = HashMap<String, ValueType>;
```

阶段 2 建议改为：

```rust
pub type TypeEnv = HashMap<String, TypeExpr>;
```

这样变量声明才能真正表达：

- `user: User`
- `posts: List<Post>`

### 变量 / 常量声明语义

在：

```zun
$ posts: List<Post> = [ ... ];
$@ current: User = User { ... };
```

执行时：

1. 用共享 validator 校验初始值
2. 成功后把 `TypeExpr` 存入 `TypeEnv`
3. 后续再赋值时继续按该类型表达校验

## 5. 共享 validator 升级

第一阶段的 validator 以字符串类型为中心。

阶段 2 建议新增或升级为：

```rust
validate_value_against_type_expr(type_expr, value, context)
```

递归规则：

- `Named("Int")` / `Named("String")` 等走基础类型
- `Named("User")` 走用户类型名义校验
- `List(inner)` 要求值是 list，且所有元素递归满足 `inner`
- `Optional(inner)`：
  - 缺失字段由外层 shape 逻辑处理
  - 若字段存在，值为 `null` 或满足 `inner`

这样构造器、字段赋值、变量赋值、list 索引赋值都能复用同一套类型逻辑。

## 6. list 索引赋值校验

阶段 2 只补这一条容器写路径：

```zun
$ posts: List<Post> = [post1];
posts[0] = post2;      // 合法
posts[0] = user;       // 非法
posts[0] = {"id": 1};  // 非法
```

运行时规则：

1. 如果被索引的变量在 `TypeEnv` 中声明为 `List<T>`
2. 则 `items[index] = value` 时必须校验 `value` 满足 `T`
3. 若变量没有声明类型，仍保持动态行为

本阶段不扩展到 list methods。

## 错误模型

阶段 2 的错误应分为三类。

### 1. 类型表达语法错误

- `bare 'List' is not allowed here; use 'List<T>'`
- `unsupported optional list element type 'List<User?>'`
- `expected '>' after list item type`

### 2. 变量与容器错误

- `variable 'posts' expects type 'List<Post>' but got 'List<User>'`
- `list element for 'posts' expects type 'Post', got 'User'`

### 3. `$Type` 字段错误

- `field 'items' of type 'Feed' expects 'List<Post>', got 'List<User>'`
- `field 'email' of type 'Profile' expects 'String?', got 'Int'`

建议错误信息仍保持高信息量，但不必在阶段 2 一次引入特别复杂的字段路径渲染。

## 测试策略

## 1. Parser / AST

需要覆盖：

- `$Type Feed { items: List<Post> }`
- `$Type Feed { items: List<Post>? }`
- `$ posts: List<Post> = []`
- 非法 `List<User?>`

## 2. Validator

需要覆盖：

- `List<User>` 接受全 `User`
- `List<User>` 拒绝混入 `Map`
- `String?` 接受 `null`
- `List<User>?` 接受 `null`
- `List<User>?` 接受合法列表

## 3. Runtime

需要覆盖：

- `$ posts: List<Post> = [Post { ... }]` 成功
- `$ posts: List<Post> = [User { ... }]` 失败
- `posts[0] = Post { ... }` 成功
- `posts[0] = User { ... }` 失败
- `$Type Feed { items: List<Post> }` 构造成功
- `$Type Feed { items: List<Post> }` 混入错误元素失败
- `$Type Profile { email: String? }` 允许 `null`

## 实施建议

建议沿以下顺序推进：

1. 引入 `TypeExpr`
2. 先改 `$Type` 字段与变量 / 常量注解 AST
3. 补 parser 类型表达解析
4. 升级 runtime validator 到 `TypeExpr`
5. 升级 `TypeEnv`
6. 接入变量赋值与 list 索引赋值
7. 补文档与测试

## 结论

阶段 2 采用“结构化类型表达 + 有限容器约束”的路线：

- 通过 `TypeExpr` 统一类型模型
- 支持 `$Type` 字段 `List<T>` / `T?` / `List<T>?`
- 支持变量 / 常量注解 `T` / `List<T>`
- 只对 list 索引赋值增加持续校验
- 明确 `?` 允许 `null`
- 明确不支持 `List<User?>`

这会把 `$Type` 从“边界严格的名义类型”进一步提升为“在声明字段、变量和容器层都可组合的 runtime 自定义类型体系”。
