# 16. HTTP 组织方式

当路由开始增多时，就需要把 HTTP 代码组织起来。

## `$HTTP { ... }`

```zun
$HTTP {
    $GET("/ping") ping() -> String {
        $# "pong";
    }

    $GET("/health") health() -> String {
        $# "ok";
    }
}
```

## 带前缀的 HTTP 块

```zun
$HTTP("/api/v1") {
    $GET("/users") list_users() -> JSON {
        $# $JSON { "ok": true };
    }
}
```

## `$main()` 的位置

`$main()` 适合放全局初始化逻辑：

```zun
@CORS("*")
$main() {
    $>> "[INFO] server starting";
}
```

什么时候需要：

- 需要全局 `@CORS`
- 需要启动日志或初始化动作
- 想把“启动阶段逻辑”和“路由定义”显式分开

什么时候可以省略：

- 只有少量路由
- 没有全局初始化逻辑

它和 `entry` 的关系是：

- `entry` 选入口文件
- `$main()` 定义该文件里的 serve 入口逻辑

## 组织 `@SET_HDR(...)`

```zun
@SET_HDR({
    "X-Frame-Options": "DENY",
    "Cache-Control": "no-cache"
})
$HTTP("/api") {
    $GET("/ping") ping() -> String {
        $# "pong";
    }
}
```

规则：

- 块级 header 会传给块内路由
- 同名 header 由更靠近路由的声明覆盖

## 组织 `@CORS(...)`

`@CORS(...)` 可以放在三个层级：

- `$main()` 前：全局默认策略
- `$HTTP(...)` 前：路由组策略
- 具体路由前：单条路由策略

优先级：

- 路由级覆盖块级
- 块级覆盖全局

## 链接外部路由模块

```zun
$HTTP("/v1/api").link("routers.api");
```

`.link()` 只用于挂载 HTTP 路由，不等于普通 `$mod` 导入。

## 静态资源和 HTML 文件

```zun
$STATIC("/static", "static");

$GET("/") index() -> HTML {
    $# $HTML().link("pages.index");
}
```

## 路由冲突怎么处理

当前仓库没有把“同路径同方法重复注册”的底层策略写成稳定主线规则，因此这一章只给出安全建议，不虚构实现细节。

推荐做法：

1. 同一个路径只在一个文件里定义一次
2. 先按业务域拆路由，再用 `.link()` 汇总
3. 启动时打开 `--routertab` 或最小化路由文件确认最终注册结果
4. 排查顺序先看入口文件、再看 `.link()`、最后看重复路径

如果你怀疑冲突，先把可疑路由缩成最小例子再排查。

## 一个完整例子

```zun
@CORS("*")
$main() {
    $>> "[INFO] server starting";
}

@SET_HDR({ "X-Frame-Options": "DENY" })
$HTTP("/api") {
    $GET("/ping") ping() -> JSON {
        $# $JSON { "ok": true };
    }
}

$HTTP("/site").link("routers.site");
$STATIC("/static", "static");
```

## 下一章

继续看 [17-testing-and-debugging.md](17-testing-and-debugging.md)。
