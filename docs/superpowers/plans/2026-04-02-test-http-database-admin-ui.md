# Test HTTP Database Admin UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `test-http-database` into a runnable Zun admin sample with a single-page HTML shell, static assets, browser-driven API calls, full CRUD for `users` and `products`, and read-only `orders` views.

**Architecture:** Keep the existing DDD backend split for domain logic, but adapt the plan to the current worktree layout by preserving `test-http-database/app/router/*.zun` as the business API modules the user already moved. Add page-only routing under `test-http-database/app/http/`, serve assets from `test-http-database/app/public/`, and let `test-http-database/app/pages/admin.html` bootstrap a framework-free SPA that talks to `/api/...`.

**Tech Stack:** Zun HTTP routes, `$HTML().link(...)`, `$STATIC(...)`, `body[...]` request parsing, `$RES(...)`, PostgreSQL-backed sample queries, `cargo run -p zun-cli -- serve`, `curl`

---

## File Map

### Create

- `test-http-database/app/http/admin_pages.zun`
  - Owns `GET /admin`
- `test-http-database/app/pages/admin.html`
  - Single-page admin shell and DOM mount points
- `test-http-database/app/public/css/admin.css`
  - Admin layout, table, form, modal, and feedback styling
- `test-http-database/app/public/js/admin.js`
  - Frontend state, rendering, fetch wrappers, tab switching, and CRUD interactions

### Modify

- `test-http-database/main.zun`
  - Restore a valid `$main()` entry, mount page routes, and mount the existing business API router modules
- `test-http-database/app/router/product_router.zun`
  - Expand from read-only routes to product/category CRUD JSON endpoints
- `test-http-database/app/router/user_router.zun`
  - Expand from list-only route to user CRUD JSON endpoints
- `test-http-database/app/router/orders_router.zun`
  - Add an orders list endpoint for the admin read-only tab
- `test-http-database/domains/products/data/product_queries.zun`
  - Add create/update/delete/find helpers and reuse parameterized SQL
- `test-http-database/domains/products/services/product_service.zun`
  - Add product/category CRUD orchestration and response shaping
- `test-http-database/domains/users/data/user_queries.zun`
  - Add create/update/delete/find helpers and reuse parameterized SQL
- `test-http-database/domains/users/services/user_service.zun`
  - Add user CRUD orchestration and response shaping
- `test-http-database/domains/orders/data/order_queries.zun`
  - Add list-all-orders query for the admin read-only tab
- `test-http-database/domains/orders/services/order_service.zun`
  - Add list-all-orders use case
- `test-http-database/README.md`
  - Document `/admin`, `/assets`, and the added CRUD endpoints

### Verify

- `cargo run -p zun-cli -- serve test-http-database --routertab`
- `curl -sS http://127.0.0.1:8082/admin`
- `curl -sS http://127.0.0.1:8082/assets/js/admin.js`
- `curl -sS http://127.0.0.1:8082/api/products`
- `curl -sS -X POST http://127.0.0.1:8082/api/products -H 'Content-Type: application/json' --data '{"name":"Keyboard","price":99.5,"category_id":1}'`
- `curl -sS -X PUT http://127.0.0.1:8082/api/products/1 -H 'Content-Type: application/json' --data '{"name":"Updated","price":15.5,"category_id":1}'`
- `curl -sS -X DELETE http://127.0.0.1:8082/api/products/1`
- `curl -sS http://127.0.0.1:8082/api/users`
- `curl -sS -X POST http://127.0.0.1:8082/api/users -H 'Content-Type: application/json' --data '{"name":"Alice","email":"alice@example.com"}'`
- `curl -sS -X PUT http://127.0.0.1:8082/api/users/1 -H 'Content-Type: application/json' --data '{"name":"Alice Updated","email":"alice.updated@example.com"}'`
- `curl -sS -X DELETE http://127.0.0.1:8082/api/users/1`
- `curl -sS http://127.0.0.1:8082/api/orders`
- browser/manual check of `http://127.0.0.1:8082/admin`

### Task 1: Restore entrypoint and add page/static routing

**Files:**
- Create: `test-http-database/app/http/admin_pages.zun`
- Modify: `test-http-database/main.zun`
- Test: `cargo run -p zun-cli -- serve test-http-database --routertab`

- [ ] **Step 1: Write the failing route smoke checks**

Run:

```bash
zun test test-http-database/main.zun --route GET /admin
```

Expected:

```text
FAIL because `/admin` is not registered yet, and `main.zun` is currently incomplete.
```

- [ ] **Step 2: Create the page route module**

```zun
$GET("/admin") admin_page() -> HTML {
    $# $HTML().link("app.pages.admin");
}
```

Keep the file limited to page concerns only; do not define `$main()` here.

- [ ] **Step 3: Restore `main.zun` and mount all route groups**

Update `test-http-database/main.zun` to a valid startup-only entry:

```zun
$main() {
    $>> "[INFO] test-http-database starting";

    $STATIC("/assets", "app/public");
    $HTTP("/").link("app.http.admin_pages");
    $HTTP("/api").link("app.router.user_router");
    $HTTP("/api").link("app.router.product_router");
    $HTTP("/api").link("app.router.orders_router");
}
```

Do not move the existing `app/router/*.zun` files back into `domains/*/routers`; keep the user’s current API router layout intact.

- [ ] **Step 4: Add minimal page shell placeholder files**

Create these placeholders so `/admin` can resolve linked assets immediately:

```html
<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Zun Admin</title>
  <link rel="stylesheet" href="/assets/css/admin.css">
</head>
<body>
  <div id="app"></div>
  <script src="/assets/js/admin.js"></script>
</body>
</html>
```

```css
body { font-family: sans-serif; }
```

```javascript
document.getElementById("app").innerHTML = "<h1>Zun Admin</h1>";
```

- [ ] **Step 5: Run the smoke checks again**

Run:

```bash
zun test test-http-database/main.zun --route GET /admin
cargo run -p zun-cli -- serve test-http-database --routertab
curl -sS http://127.0.0.1:8082/assets/js/admin.js
```

Expected:

```text
`/admin` responds with HTML output, `/assets/js/admin.js` resolves over HTTP, and the route table shows `/admin` plus the existing `/api/...` endpoints.
```

- [ ] **Step 6: Commit**

```bash
git add test-http-database/main.zun test-http-database/app/http/admin_pages.zun test-http-database/app/pages/admin.html test-http-database/app/public/css/admin.css test-http-database/app/public/js/admin.js docs/superpowers/plans/2026-04-02-test-http-database-admin-ui.md
git commit -m "feat: add admin page shell for test-http-database"
```

### Task 2: Add product CRUD and orders list APIs for the admin

**Files:**
- Modify: `test-http-database/app/router/product_router.zun`
- Modify: `test-http-database/app/router/orders_router.zun`
- Modify: `test-http-database/domains/products/data/product_queries.zun`
- Modify: `test-http-database/domains/products/services/product_service.zun`
- Modify: `test-http-database/domains/orders/data/order_queries.zun`
- Modify: `test-http-database/domains/orders/services/order_service.zun`
- Test: `zun test test-http-database/main.zun --route GET /api/products`

- [ ] **Step 1: Write the failing API checks**

Run:

```bash
zun test test-http-database/main.zun --route POST /api/products --body '{"name":"Keyboard","price":99.5,"category_id":1}'
zun test test-http-database/main.zun --route PUT /api/products/1 --body '{"name":"Updated","price":15.5,"category_id":1}'
zun test test-http-database/main.zun --route DELETE /api/products/1
zun test test-http-database/main.zun --route GET /api/orders
```

Expected:

```text
FAIL because these routes do not exist yet.
```

- [ ] **Step 2: Add parameterized product data helpers**

Extend `test-http-database/domains/products/data/product_queries.zun` with helper functions shaped like:

```zun
$fn find_product_row_by_id(id) -> List<Map> {
    $# queries.safe_query(
        "SELECT id, name, price::double precision AS price, category_id FROM products WHERE id::bigint = $1",
        [id.to_int()]
    );
}

$fn insert_product_row(name, price, category_id) -> List<Map> {
    $# queries.safe_query(
        "INSERT INTO products (name, price, category_id) VALUES ($1, $2, $3) RETURNING id, name, price::double precision AS price, category_id",
        [name, price, category_id]
    );
}
```

Also add matching update and delete helpers, and an orders query:

```zun
$fn list_order_rows() -> List<Map> {
    $# queries.safe_query(
        "SELECT id, user_id, created_at::text AS created_at, status FROM orders ORDER BY id",
        []
    );
}
```

- [ ] **Step 3: Add minimal service orchestration**

Update `test-http-database/domains/products/services/product_service.zun` to expose:

```zun
$fn get_product_by_id(id) -> Product { ... }
$fn create_product(name, price, category_id) -> Product { ... }
$fn update_product(id, name, price, category_id) -> Product { ... }
$fn delete_product(id) -> Map { ... }
```

Return `Product` for reads/writes and a stable JSON-shaped map for delete:

```zun
$# {
    "deleted": true,
    "id": id.to_int()
};
```

Update `test-http-database/domains/orders/services/order_service.zun` with:

```zun
$fn list_orders() -> List<Order> { ... }
```

- [ ] **Step 4: Expose product CRUD and order list routes**

Update `test-http-database/app/router/product_router.zun` into this shape:

```zun
$GET("/categories") get_categories() -> List<Category> {
    $# product_service.list_categories();
}

$HTTP("/products") {
    $GET("") get_products() -> List<Product> {
        $# product_service.list_products();
    }

    $GET("/:id") get_product(id) -> Product {
        $# product_service.get_product_by_id(id);
    }

    $POST("") create_product() -> Product {
        $# product_service.create_product(body["name"], body["price"], body["category_id"]);
    }

    $PUT("/:id") update_product(id) -> Product {
        $# product_service.update_product(id, body["name"], body["price"], body["category_id"]);
    }

    $DELETE("/:id") delete_product(id) -> JSON {
        $# $JSON(product_service.delete_product(id));
    }
}
```

Update `test-http-database/app/router/orders_router.zun` with:

```zun
$GET("/orders") get_orders() -> List<Order> {
    $# order_service.list_orders();
}
```

- [ ] **Step 5: Run API verification**

Run:

```bash
zun test test-http-database/main.zun --route GET /api/products
zun test test-http-database/main.zun --route POST /api/products --body '{"name":"Keyboard","price":99.5,"category_id":1}'
zun test test-http-database/main.zun --route PUT /api/products/1 --body '{"name":"Updated","price":15.5,"category_id":1}'
zun test test-http-database/main.zun --route DELETE /api/products/1
zun test test-http-database/main.zun --route GET /api/orders
```

Expected:

```text
All five routes resolve and return JSON/typed responses instead of route-not-found errors.
```

- [ ] **Step 6: Commit**

```bash
git add test-http-database/app/router/product_router.zun test-http-database/app/router/orders_router.zun test-http-database/domains/products/data/product_queries.zun test-http-database/domains/products/services/product_service.zun test-http-database/domains/orders/data/order_queries.zun test-http-database/domains/orders/services/order_service.zun
git commit -m "feat: add product admin APIs"
```

### Task 3: Add user CRUD APIs for the admin

**Files:**
- Modify: `test-http-database/app/router/user_router.zun`
- Modify: `test-http-database/domains/users/data/user_queries.zun`
- Modify: `test-http-database/domains/users/services/user_service.zun`
- Test: `zun test test-http-database/main.zun --route POST /api/users --body '{"name":"Alice","email":"alice@example.com"}'`

- [ ] **Step 1: Write the failing API checks**

Run:

```bash
zun test test-http-database/main.zun --route GET /api/users/1
zun test test-http-database/main.zun --route POST /api/users --body '{"name":"Alice","email":"alice@example.com"}'
zun test test-http-database/main.zun --route PUT /api/users/1 --body '{"name":"Alice Updated","email":"alice.updated@example.com"}'
zun test test-http-database/main.zun --route DELETE /api/users/1
```

Expected:

```text
FAIL because only `GET /api/users` exists today.
```

- [ ] **Step 2: Add parameterized user data helpers**

Extend `test-http-database/domains/users/data/user_queries.zun` with:

```zun
$fn find_user_row_by_id(id) -> List<Map> {
    $# queries.safe_query(
        "SELECT id, name, email, created_at::text AS created_at FROM users WHERE id::bigint = $1",
        [id.to_int()]
    );
}

$fn insert_user_row(name, email) -> List<Map> {
    $# queries.safe_query(
        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email, created_at::text AS created_at",
        [name, email]
    );
}
```

Also add update and delete helpers with placeholder parameters, never string interpolation.

- [ ] **Step 3: Add user service orchestration**

Update `test-http-database/domains/users/services/user_service.zun` with:

```zun
$fn get_user_by_id(id) -> User { ... }
$fn create_user(name, email) -> User { ... }
$fn update_user(id, name, email) -> User { ... }
$fn delete_user(id) -> Map { ... }
```

Use the same not-found handling pattern as the order service for missing rows.

- [ ] **Step 4: Expose user CRUD routes**

Update `test-http-database/app/router/user_router.zun` into this shape:

```zun
$HTTP("/users") {
    $GET("") get_users() -> List<User> {
        $# user_service.list_users();
    }

    $GET("/:id") get_user(id) -> User {
        $# user_service.get_user_by_id(id);
    }

    $POST("") create_user() -> User {
        $# user_service.create_user(body["name"], body["email"]);
    }

    $PUT("/:id") update_user(id) -> User {
        $# user_service.update_user(id, body["name"], body["email"]);
    }

    $DELETE("/:id") delete_user(id) -> JSON {
        $# $JSON(user_service.delete_user(id));
    }
}
```

- [ ] **Step 5: Run API verification**

Run:

```bash
zun test test-http-database/main.zun --route GET /api/users
zun test test-http-database/main.zun --route GET /api/users/1
zun test test-http-database/main.zun --route POST /api/users --body '{"name":"Alice","email":"alice@example.com"}'
zun test test-http-database/main.zun --route PUT /api/users/1 --body '{"name":"Alice Updated","email":"alice.updated@example.com"}'
zun test test-http-database/main.zun --route DELETE /api/users/1
```

Expected:

```text
All five user endpoints resolve and return structured JSON/typed data.
```

- [ ] **Step 6: Commit**

```bash
git add test-http-database/app/router/user_router.zun test-http-database/domains/users/data/user_queries.zun test-http-database/domains/users/services/user_service.zun
git commit -m "feat: add user admin APIs"
```

### Task 4: Build the admin SPA and wire it to the APIs

**Files:**
- Modify: `test-http-database/app/pages/admin.html`
- Modify: `test-http-database/app/public/css/admin.css`
- Modify: `test-http-database/app/public/js/admin.js`
- Test: `curl -sS http://127.0.0.1:8082/admin`

- [ ] **Step 1: Replace the placeholder shell with the approved page structure**

Update `test-http-database/app/pages/admin.html` to include these stable mount points:

```html
<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Zun Admin</title>
  <link rel="stylesheet" href="/assets/css/admin.css">
</head>
<body>
  <div id="app">
    <header id="toolbar"></header>
    <main id="content"></main>
    <div id="modal"></div>
    <div id="message"></div>
  </div>
  <script src="/assets/js/admin.js"></script>
</body>
</html>
```

- [ ] **Step 2: Implement the frontend state and fetch wrappers**

Build `test-http-database/app/public/js/admin.js` around this state shape:

```javascript
const state = {
  currentTab: "products",
  listData: [],
  selectedItem: null,
  formMode: "create",
  loading: false,
  errorMessage: ""
};
```

Add request helpers:

```javascript
async function apiGet(path) { ... }
async function apiSend(path, method, payload) { ... }
```

The frontend must call:

```text
/api/users
/api/products
/api/categories
/api/orders
/api/orders/:id/detail
```

- [ ] **Step 3: Render products and users as full CRUD tabs**

Implement:

```javascript
function renderToolbar() { ... }
function renderList() { ... }
function renderForm(item) { ... }
function handleCreateSubmit() { ... }
function handleUpdateSubmit(id) { ... }
function handleDelete(id) { ... }
```

Requirements:

```text
- products tab: list, create, edit, delete
- users tab: list, create, edit, delete
- refresh the current list after each successful mutation
- show success/error messages in `#message`
```

- [ ] **Step 4: Render orders as a read-only operations tab**

Implement a third tab in `admin.js` that:

```text
- loads GET /api/orders
- lets the user click an order row
- loads GET /api/orders/:id/detail
- shows order, user, and item information
- does not expose create/update/delete controls
```

- [ ] **Step 5: Replace placeholder CSS with admin styling**

Update `test-http-database/app/public/css/admin.css` to cover:

```text
- page layout
- tab/navigation styling
- table styling
- form fields and action buttons
- selected row and message banners
- responsive behavior for mobile widths
```

- [ ] **Step 6: Run end-to-end verification**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
curl -sS http://127.0.0.1:8082/admin
curl -sS http://127.0.0.1:8082/assets/css/admin.css
curl -sS http://127.0.0.1:8082/assets/js/admin.js
curl -sS http://127.0.0.1:8082/api/products
curl -sS http://127.0.0.1:8082/api/users
curl -sS http://127.0.0.1:8082/api/orders
```

Then manually open `http://127.0.0.1:8082/admin` and verify:

```text
- default tab loads
- switching tabs reloads the correct data
- product create/edit/delete works
- user create/edit/delete works
- order detail panel loads
```

- [ ] **Step 7: Commit**

```bash
git add test-http-database/app/pages/admin.html test-http-database/app/public/css/admin.css test-http-database/app/public/js/admin.js
git commit -m "feat: add admin ui for test-http-database"
```

### Task 5: Update documentation and final verification

**Files:**
- Modify: `test-http-database/README.md`
- Test: `cargo run -p zun-cli -- serve test-http-database --routertab`

- [ ] **Step 1: Update the sample README**

Add these sections to `test-http-database/README.md`:

```text
- admin entry: /admin
- static assets: /assets/...
- product CRUD endpoints
- user CRUD endpoints
- orders list endpoint
- browser verification steps
```

- [ ] **Step 2: Run final verification**

Run:

```bash
cargo run -p zun-cli -- serve test-http-database --routertab
curl -sS http://127.0.0.1:8082/admin
curl -sS http://127.0.0.1:8082/api/products
curl -sS http://127.0.0.1:8082/api/users
curl -sS http://127.0.0.1:8082/api/orders
```

Expected:

```text
The sample starts cleanly, the admin shell resolves, and the three admin tabs have backing APIs.
```

- [ ] **Step 3: Commit**

```bash
git add test-http-database/README.md
git commit -m "docs: document admin ui sample"
```

## Self-Review

- Spec coverage: task 1 covers `/admin` + `/assets`, tasks 2-3 cover CRUD/read-only APIs, task 4 covers single-page admin behavior, task 5 covers docs and verification.
- Placeholder scan: all tasks name exact files and commands; no `TODO` or “similar to” references remain.
- Type consistency: `Product` and `User` writes return typed values; deletes return `Map` wrapped by `-> JSON` routes; orders remain read-only.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-02-test-http-database-admin-ui.md`. Two execution options:

1. Subagent-Driven (recommended) - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. Inline Execution - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
