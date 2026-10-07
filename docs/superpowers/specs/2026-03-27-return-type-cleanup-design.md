# Return Type Cleanup Design

## Goal

统一 Zun 当前返回类型语义，去掉 `JSON<T>` 这类含混写法，并让 HTTP handler 与普通函数使用同一套返回类型校验规则。

本次设计解决三个已经暴露的问题：

- HTTP handler 目前没有执行返回类型校验
- `JSON<User>`、`JSON<List>` 这类写法会被 parser 接受，但 runtime 并没有按泛型语义处理
- 用户自定义类型名目前只要 runtime 里存在，就可能被跨模块直接引用，缺少显式导入约束

## New Rules

### 1. 返回类型只允许两类

#### 基本返回类型

允许这些内建返回类型名：

- `Int`
- `Float`
- `String`
- `Bool`
- `List`
- `Map`
- `JSON`
- `HTML`

这些类型名属于语言内建，用户不能自定义同名类型。

#### 用户自定义类型

允许直接写用户类型名，例如：

```zun
$mod data.user;

$fn get_user() -> User {
    $# User { id: 1, name: "Alice" };
}
```

用户类型必须来自 `$Type` 声明，并且当前模块中必须可见。

### 2. 禁止 `JSON<T>` 泛型返回类型写法

以下写法全部非法：

```zun
$fn a() -> JSON<User> { ... }
$fn b() -> JSON<List> { ... }
$GET("/users") list() -> JSON<User> { ... }
```

替代规则：

- 想表达 JSON 响应格式：写 `-> JSON`
- 想表达返回用户自定义类型：写 `-> User`
- 想表达返回列表：写 `-> List`

### 3. HTTP handler 与普通函数统一做返回类型校验

HTTP handler 不再绕过校验链，语义与 `$fn` 对齐。

例如：

```zun
$GET("/users/:id") get_user(id) -> User {
    $# users_service.find_user_by_id(id);
}
```

如果实际返回值不是 `User` 类型实例，则报错。

### 4. 用户类型名必须显式导入

如果某个文件要在返回类型、变量类型或其他类型位置写 `User`，则必须先显式导入定义该类型的模块：

```zun
$mod data.user;

$GET("/users/:id") get_user(id) -> User {
    ...
}
```

未导入时，视为类型不可见并报错。

## Parser Changes

- 函数声明和 HTTP handler 的 `-> Type` 解析只接受单个类型名
- 一旦在返回类型位置读到 `<`，直接报语法错误，不再继续构造 `JSON<T>`
- 这条规则同时适用于：
  - `$fn`
  - 匿名函数字面量（如果当前也支持 `-> Type`）
  - `$GET/$POST/...`

## Runtime Changes

### HTTP handler return validation

- 在 HTTP handler 执行完成并拿到 `$#` 返回值后，执行和普通函数同级别的返回类型检查
- 如果声明了返回类型但返回值不匹配，返回运行时错误

### Built-in return type validation

继续对这些返回类型做运行时检查：

- `Int`
- `Float`
- `String`
- `Bool`
- `List`
- `Map`
- `JSON`
- `HTML`

### User-defined type validation

如果返回类型是用户自定义类型，例如 `User`：

- 当前模块必须可见该类型
- 返回值必须是对应类型名的 `TypedInstance`
- `Null` 不自动放行，除非后续语言层明确引入可空返回语义

## Module Visibility Rule

类型可见性以当前模块显式导入为准，而不是“runtime 全局碰巧注册过”。

这意味着：

- `service` 文件里要写 `-> User`，必须 `$mod data.user;`
- `router` 文件里要写 `-> User`，同样必须 `$mod data.user;`
- 仅仅调用一个返回 `User` 的 service，不足以让当前文件自动获得 `User` 类型名可见性

## my-app Migration

`my-app` 按下面方式收敛：

- router:
  - `getUsers() -> JSON`
  - `getUserById(id) -> User`
  - 如果 router 直接写 `User`，必须 `$mod data.users_data;`
- service:
  - `find_all_users() -> List`
  - `find_user_by_id(id) -> User`，或在未命中语义下改成 `-> JSON` / 无注解

注意：如果“查不到用户”仍然要求返回 `{ "Null": "No data" }`，那 `find_user_by_id(id)` 就不能同时声明为 `-> User`。这一点需要按业务语义选择：

- 要么保留 `No data` JSON 结构，则函数返回类型应为 `JSON` 或省略
- 要么要求类型严格，则未命中应改成抛错 / 统一错误响应，而不是返回伪 JSON 结构

本次清理先以“语言规则正确”为先，不在 spec 中强行规定 `my-app` 业务未命中的最终返回结构。

## Tests

新增或更新这些测试：

- parser/spec:
  - `JSON<User>` 在函数返回类型中报错
  - `JSON<List>` 在 HTTP handler 返回类型中报错
- runtime/spec:
  - HTTP handler `-> Int` 返回 `Map/List/User` 时报错
  - HTTP handler `-> User` 返回错误类型时报错
  - 未导入 `User` 却写 `-> User` 时报错
- integration:
  - `my-app` 改用新返回类型写法后仍可正常工作

## Docs To Update

- `docs/spec/syntax.md`
- `docs/spec/semantics.md`
- `docs/reference/functions.md`
- `docs/reference/http.md`
- `docs/reference/types.md`
- `docs/guide/15-http-basics.md`
- `docs/CHANGELOG.md`

## Scope

本次只做返回类型语义收敛，不扩展新的泛型类型系统，也不引入可空返回类型语法。
