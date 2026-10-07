<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-wordmark-dark.svg">
    <img src="assets/logo-wordmark-light.svg" alt="Zun" width="360">
  </picture>
</p>

<p align="center">
  <strong>Tiny as a hummingbird. Fast as one.</strong><br>
  A small, complete language for HTTP services — one file, one command, under 10 MB of memory.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README-CN.md">中文</a>
</p>

---

```zun
$GET("/hello/:name") hello(name) -> JSON {
    $# { "message": "Hello, " + name + "!" };
}
```

```bash
zun serve
# ➜  Local:   http://127.0.0.1:8080
```

That is a complete web service. No framework, no server boilerplate, no
dependency-injection container.

## Why Zun

A hummingbird weighs about as much as a coin, yet it hovers, flies backwards,
and beats its wings 50 times a second. Zun aims for the same balance:

| | |
|---|---|
| 🪶 **Tiny** | One binary. A single `main.zun` is a whole service. HTTP routes are syntax, not a library. |
| 🍃 **Light** | **9.8 MB** resident memory under load, about 4× smaller than FastAPI and 6× smaller than Node + Hono. Built on Rust, so there are no garbage-collector pauses. |
| 🧰 **Complete** | Batteries included: an HTTP server and client, SQLite and PostgreSQL, sessions, JWT, password hashing, CSRF, JSON, the filesystem, a test runner, a REPL, and an LSP. |
| ⚡ **Fast** | About **42k QPS** with a **1.84 ms p99** in our benchmark, roughly 6× the throughput of FastAPI. |

Zun is aimed at a narrow, high-value slice of backend work: API gateways,
edge functions, sidecars, and internal tools, where you want the feel of a
scripting language with the footprint of Rust.

## Quick Start

You need a stable Rust toolchain.

```bash
git clone https://github.com/zhanbodev/zun-lang.git
cd zun-lang
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

Create `main.zun`:

```zun
$>> "Hello, Zun!";
```

Then run it:

```bash
zun run main.zun
```

## A Taste of the Language

Every keyword starts with `$`, so language constructs stand out from your own
names. You only need to add types where they help.

```zun
// Variables and constants
$ name = "Zun";
$ version: Int = 1;
$@ MOTTO = "tiny, light, complete, fast";

// Functions
$fn greet(who: Str) -> Str {
    $# "Hello, " + who + "!";
}

// Records
$Type Bird {
    name: Str
    weight: Float
}

$ birds = [
    Bird { name: "Bee hummingbird", weight: 1.6 },
    Bird { name: "Ruby-throated", weight: 3.4 }
];

// Control flow
$for bird in birds {
    $ grams = bird.weight;
    $if grams < 2.0 {
        $>> bird.name;
    }
}

// Errors
$try {
    $throw "out of nectar";
} $catch err {
    $>> "caught: " + err;
}

$>> greet(name);
```

| Symbol | Meaning |
|---|---|
| `$ x = …` | declare a variable |
| `$@ X = …` | declare a constant |
| `$fn` | define a function |
| `$#` | return |
| `$>>` | print to stdout |
| `$mod` | import a module |
| `$Type` | define a record type |

## Your First HTTP Service

```text
hello-api/
├── package.toml
├── main.zun
└── app/router/birds.zun
```

`main.zun` mounts a router module under a path prefix:

```zun
$main() {
    $HTTP("/api").link("app.router.birds");
}
```

`app/router/birds.zun` holds the routes. Path parameters arrive as handler
arguments, and a JSON request body is available as `body`:

```zun
$GET("/hello/:name") hello(name) -> JSON {
    $# { "message": "Hello, " + name + "!" };
}

$POST("/birds") create() -> JSON {
    $ name = body["name"];
    $if name == "" {
        $# $RES(400, { "error": "name is required" });
    }
    $# $RES(201, { "created": name });
}
```

```bash
zun serve --routertab
curl localhost:8080/api/hello/Zun          # {"message":"Hello, Zun!"}
curl -X POST localhost:8080/api/birds \
     -H 'content-type: application/json' \
     -d '{"name":"Bee"}'                    # 201 {"created":"Bee"}
```

`package.toml` is the single source of truth for a project:

```toml
[project]
name    = "hello-api"
version = "0.1.0"
entry   = "main.zun"

[server]
host                 = "0.0.0.0"
port                 = 8080
upload_max_size      = 50   # total request body limit, MB
upload_max_file_size = 10   # per-file limit, MB
```

## Command Line

| Command | What it does |
|---|---|
| `zun` | Start the REPL |
| `zun run <file.zun>` | Run a script |
| `zun serve [path]` | Start the HTTP server for a project. `--routertab` prints the route table |
| `zun test [file]` | Run tests (default `main.zun`). `--route METHOD /path` and `--body JSON` test a single route |

Zun only executes `.zun` files.

## Standard Library

| Area | Modules |
|---|---|
| Core | `std.str` · `std.math` · `std.json` · `std.time` · `std.uuid` · `std.env` · `std.fs` |
| Network | `std.http` (outbound HTTP client). The server side is built into the language |
| Data | `std.sqlite` · `std.postgres` |
| Auth | `std.auth.session` · `std.auth.jwt` · `std.auth.password` · `std.auth.guard` · `std.auth.csrf` |

See the full API in [docs/reference/stdlib-api.md](docs/reference/stdlib-api.md).

## Performance

All three services implement identical `/hello` and `/greet/:name` endpoints,
measured on the same machine with the same client.
Reproduce the results with [`benches/http-comparison/`](benches/http-comparison/).

| Target | RSS | Cold start | `/hello` QPS | p99 |
|:--|--:|--:|--:|--:|
| **Zun** | **9.8 MB** | 1011 ms | **42,431** | **1.84 ms** |
| FastAPI + uvicorn | 43.3 MB | 3108 ms | 6,814 | 16.57 ms |
| Hono + Node.js | 63.0 MB | **76 ms** | 39,025 | 1.96 ms |

To be honest about what these numbers show:

- **Memory is the headline.** One box fits about 4× more Zun services than
  FastAPI, and about 6× more than Hono.
- **Throughput is tied with Hono.** Both servers hit the ceiling of the Python
  load generator, not their own limits.
- **Cold start is our open problem.** It takes about 1 s today. That doesn't
  matter for long-running servers, but it does for FaaS workloads.

## Workspace Layout

| Path | Contents |
|---|---|
| `crates/zun-frontend/` | Lexer, parser, AST, diagnostics |
| `crates/zun-runtime/` | Interpreter, module resolver, standard library |
| `crates/zun-cli/` | `zun` CLI: REPL, `run`, `serve`, `test` |
| `crates/zun-lsp/` | Language Server Protocol implementation |
| `stdlib/` | Standard library modules written in Zun |
| `benches/` | Microbenchmarks and the HTTP comparison suite |
| `docs/` | Guide, reference, and specs |
| `tests/` | Integration tests and spec fixtures |
| `assets/` | Logo files |

## Documentation

- [Guide](docs/guide/README.md): learn Zun step by step
- [Syntax](docs/reference/syntax.md) · [Types](docs/reference/types.md) · [HTTP](docs/reference/http.md) · [SQL](docs/reference/sql-database.md)
- [Project system](docs/reference/project-system.md) · [Standard library](docs/reference/stdlib-api.md) · [Error codes](docs/reference/errors.md)
- [Contributor guide](docs/contributing/dev-guide.md)

## Status

Zun is young. Expect breaking changes before 1.0. These are the known sharp
edges:

- Cold start is about 1 s.
- Databases are limited to SQLite and PostgreSQL. MySQL and Redis are not yet supported.
- There is no package registry. Projects compose through local modules and `package.toml`.
- Observability is minimal. OpenTelemetry support is planned.

## Contributing

Before you send a change, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).
Report security issues through [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
