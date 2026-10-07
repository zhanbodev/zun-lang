# Test HTTP Database Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure `test-http-database` into the approved Zun DDD sample layout with explicit `app`, `config`, and per-domain `data / services / routers` boundaries while keeping the HTTP + PostgreSQL sample runnable.

**Architecture:** Keep the project root as the source root, move database connection concerns into `config/database.zun`, centralize route assembly in `app/http/routes.zun`, and ensure each domain exposes its own router module. Preserve the existing query and service behavior, but tighten responsibility boundaries so `main.zun` only boots and assembles routes.

**Tech Stack:** Zun project system (`package.toml`), Zun HTTP routes and `.link()`, `std.postgres`, local PostgreSQL, `cargo run -p zun-cli -- serve`, `curl`

---

## File Structure

### Create

- `test-http-database/app/http/routes.zun`
- `test-http-database/config/app_config.zun`
- `test-http-database/config/database.zun`
- `test-http-database/domains/orders/routers/order_router.zun`

### Modify

- `test-http-database/main.zun`
- `test-http-database/README.md`
- `test-http-database/shared/db/queries.zun`
- `test-http-database/domains/users/data/user_queries.zun`
- `test-http-database/domains/users/services/user_service.zun`
- `test-http-database/domains/users/routers/user_router.zun`
- `test-http-database/domains/products/data/product_queries.zun`
- `test-http-database/domains/products/services/product_service.zun`
- `test-http-database/domains/products/routers/product_router.zun`
- `test-http-database/domains/orders/data/order_queries.zun`
- `test-http-database/domains/orders/services/order_service.zun`

### Delete

- `test-http-database/shared/db/connection.zun`

### Responsibilities

- `main.zun`: startup only; import the app-level routes module and keep no domain HTTP handlers
- `app/http/routes.zun`: assemble `/api` route groups by linking domain router modules
- `config/app_config.zun`: expose the project’s database connection string as a single stable source
- `config/database.zun`: create PostgreSQL connections from config
- `shared/db/queries.zun`: common query wrapper only
- `domains/*/routers/*.zun`: one router module per domain, no SQL or connection creation
- `domains/*/services/*.zun`: use-case orchestration and error conversion
- `domains/*/data/*.zun`: SQL and row-fetching only

### Task 1: Introduce App And Config Layers

**Files:**
- Create: `test-http-database/app/http/routes.zun`
- Create: `test-http-database/config/app_config.zun`
- Create: `test-http-database/config/database.zun`
- Modify: `test-http-database/main.zun`
- Modify: `test-http-database/shared/db/queries.zun`
- Delete: `test-http-database/shared/db/connection.zun`
- Test: `cargo run -p zun-cli -- serve test-http-database --routertab`

- [ ] **Step 1: Write the failing app/config skeleton**

```zun
# test-http-database/config/app_config.zun
$fn database_url() -> String {
    $# "postgresql://liangzhanbo@127.0.0.1:5432/postgres";
}
```

```zun
# test-http-database/config/database.zun
$mod std.postgres;
$mod config.app_config;

$fn connect_db() {
    $# postgres.connect(app_config.database_url());
}
```

```zun
# test-http-database/app/http/routes.zun
$HTTP("/api").link("domains.users.routers.user_router");
$HTTP("/api").link("domains.products.routers.product_router");
$HTTP("/api").link("domains.orders.routers.order_router");
```

```zun
# test-http-database/main.zun
$mod app.http.routes;

$main() {
    $>> "[INFO] test-http-database starting";
}
```

```zun
# test-http-database/shared/db/queries.zun
$mod config.database;

$fn safe_query(sql, params) -> List<Map> {
    $try {
        $ conn = database.connect_db();
        $ rows = conn.query(sql, params);
        $>> conn.close();
        $# rows;
    } $catch err {
        $throw "database unavailable";
    }
}
```

Expected: loading the project now fails because the orders router module does not exist yet.

- [ ] **Step 2: Run the server to capture the missing-router failure**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
```

Expected: FAIL with a module resolution error for `domains.orders.routers.order_router`.

- [ ] **Step 3: Add the minimal orders router shim to make route assembly loadable**

```zun
# test-http-database/domains/orders/routers/order_router.zun
$mod domains.orders.data.order_types;
$mod domains.orders.services.order_service;

$GET("/users/:id/orders") get_user_orders(id) -> List<Order> {
    $# order_service.list_orders_by_user_id(id);
}

$HTTP("/orders") {
    $GET("/:id") get_order(id) -> Order {
        $# order_service.get_order_by_id(id);
    }

    $GET("/:id/items") get_order_items(id) -> List<OrderItem> {
        $# order_service.list_order_items(id);
    }

    $GET("/:id/detail") get_order_detail(id) -> Map {
        $# order_service.get_order_detail(id);
    }
}
```

- [ ] **Step 4: Re-run the server to verify the thin-entry structure loads**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
```

Expected:

- the server starts
- `main.zun` contains only startup logic
- the router table includes users, products, and orders routes

- [ ] **Step 5: Commit the layering scaffold**

```bash
git add test-http-database/main.zun test-http-database/app/http/routes.zun test-http-database/config/app_config.zun test-http-database/config/database.zun test-http-database/shared/db/queries.zun test-http-database/domains/orders/router/order_router.zun
git rm test-http-database/shared/db/connection.zun
git commit -m "refactor: add app and config layers to test-http-database"
```

### Task 2: Route All Domain Imports Through The New Boundaries

**Files:**
- Modify: `test-http-database/domains/users/data/user_queries.zun`
- Modify: `test-http-database/domains/users/services/user_service.zun`
- Modify: `test-http-database/domains/users/routers/user_router.zun`
- Modify: `test-http-database/domains/products/data/product_queries.zun`
- Modify: `test-http-database/domains/products/services/product_service.zun`
- Modify: `test-http-database/domains/products/routers/product_router.zun`
- Modify: `test-http-database/domains/orders/data/order_queries.zun`
- Modify: `test-http-database/domains/orders/services/order_service.zun`
- Test: `cargo run -p zun-cli -- serve test-http-database --routertab`

- [ ] **Step 1: Update data modules so all database access flows through `shared.db.queries` or `config.database`**

Representative target imports:

```zun
$mod shared.db.queries;
```

and remove any remaining:

```zun
$mod shared.db.connection;
```

For example, keep `domains/users/data/user_queries.zun` in this shape:

```zun
$mod shared.db.queries;

$fn list_user_rows() -> List<Map> {
    $# queries.safe_query("SELECT id, name, email, created_at FROM users ORDER BY id", []);
}
```

- [ ] **Step 2: Keep service modules focused on orchestration only**

Use `domains/orders/services/order_service.zun` as the reference. The target shape is:

```zun
$mod domains.orders.data.order_queries;
$mod domains.orders.data.order_types;
$mod domains.users.data.user_types;

$fn get_order_detail(id) -> Map {
    $try {
        $ order = get_order_by_id(id);
        $ user_rows = order_queries.find_user_row(order.user_id);

        $if user_rows.len() == 0 {
            $throw "order user not found";
        }

        $# {
            "order": order,
            "user": User {
                id: user_rows[0]["id"],
                name: user_rows[0]["name"],
                email: user_rows[0]["email"],
                created_at: user_rows[0]["created_at"],
            },
            "items": list_order_items(id)
        };
    } $catch err {
        $if err == "order not found" {
            $throw err;
        }

        $if err == "order user not found" {
            $throw err;
        }

        $throw "database unavailable";
    }
}
```

- [ ] **Step 3: Keep router modules limited to HTTP entrypoints**

Representative target for `domains/users/routers/user_router.zun`:

```zun
$mod domains.users.data.user_types;
$mod domains.users.services.user_service;

$GET("/users") get_users() -> List<User> {
    $# user_service.list_users();
}
```

Representative target for `domains/products/routers/product_router.zun`:

```zun
$mod domains.products.data.category_types;
$mod domains.products.data.product_types;
$mod domains.products.services.product_service;

$GET("/categories") get_categories() -> List<Category> {
    $# product_service.list_categories();
}

$GET("/products") get_products() -> List<Product> {
    $# product_service.list_products();
}
```

- [ ] **Step 4: Run the server to verify all modules resolve after the import rewrite**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
```

Expected: PASS with no import errors and the same route table as Task 1.

- [ ] **Step 5: Commit the dependency cleanup**

```bash
git add test-http-database/domains/users/data/user_queries.zun test-http-database/domains/users/services/user_service.zun test-http-database/domains/users/router/user_router.zun test-http-database/domains/products/data/product_queries.zun test-http-database/domains/products/services/product_service.zun test-http-database/domains/products/router/product_router.zun test-http-database/domains/orders/data/order_queries.zun test-http-database/domains/orders/services/order_service.zun
git commit -m "refactor: align domain modules with new boundaries"
```

### Task 3: Verify Runtime Behavior Against The Existing HTTP Surface

**Files:**
- Modify: `test-http-database/README.md`
- Test: running server plus manual `curl`

- [ ] **Step 1: Update the README to document the new recommended structure**

Add a short structure section like:

```md
## Recommended Layout

- `main.zun`: startup only
- `app/http/routes.zun`: route assembly
- `config/`: runtime configuration and database connection
- `domains/*/data`: SQL and row fetching
- `domains/*/services`: use-case orchestration
- `domains/*/routers`: HTTP adapters
```

- [ ] **Step 2: Start the sample server**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
```

Expected: PASS, server listens on `127.0.0.1:8082` or the configured host/port and prints the route table.

- [ ] **Step 3: Verify the list endpoints still behave**

Run:

```bash
curl -sS http://127.0.0.1:8082/api/users
curl -sS http://127.0.0.1:8082/api/categories
curl -sS http://127.0.0.1:8082/api/products
```

Expected:

- each request returns JSON
- no database connection or module errors appear

- [ ] **Step 4: Verify the order endpoints still behave**

Run:

```bash
curl -sS http://127.0.0.1:8082/api/users/1/orders
curl -sS http://127.0.0.1:8082/api/orders/1
curl -sS http://127.0.0.1:8082/api/orders/1/items
curl -sS http://127.0.0.1:8082/api/orders/1/detail
curl -sS http://127.0.0.1:8082/api/orders/999
```

Expected:

- successful requests keep the old payload shape
- `/api/orders/999` still returns the existing not-found behavior

- [ ] **Step 5: Commit the sample documentation refresh**

```bash
git add test-http-database/README.md
git commit -m "docs: describe recommended sample layout"
```

### Task 4: Final Verification And Diff Review

**Files:**
- Modify: none expected
- Test: targeted status and diff review

- [ ] **Step 1: Review the changed tree to confirm the intended structure**

Run:

```bash
find test-http-database -maxdepth 4 -type f | sort
```

Expected: includes `app/http/routes.zun`, `config/app_config.zun`, `config/database.zun`, and `domains/orders/routers/order_router.zun`, and no longer includes `shared/db/connection.zun`.

- [ ] **Step 2: Review the git diff for scope control**

Run:

```bash
git diff --stat -- test-http-database docs/superpowers/plans/2026-04-01-test-http-database.md
```

Expected: only the planned sample-layout files and this plan are touched.

- [ ] **Step 3: Re-run the serve command one last time**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
```

Expected: PASS.

- [ ] **Step 4: Capture the final verification commands in the handoff summary**

Include:

```text
cargo run -p zun-cli -- serve test-http-database --routertab
curl -sS http://127.0.0.1:8082/api/users
curl -sS http://127.0.0.1:8082/api/orders/1/detail
```

- [ ] **Step 5: Commit the completed refactor**

```bash
git add test-http-database docs/superpowers/plans/2026-04-01-test-http-database.md
git commit -m "refactor: turn test-http-database into ddd sample"
```
