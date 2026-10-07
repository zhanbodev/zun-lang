# Auth News Sample Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `sample/auth-b2b-portal` into a realistic news website sample with public pages, an authenticated news admin console, stronger role/permission coverage, and Postgres-backed news content data.

**Architecture:** Keep the existing auth sample intact and add a thin news module on top of it. Public pages and admin pages remain static HTML/CSS/JS shells served by Zun routes, while Zun services and routers own all news reads/writes against Postgres. Role and permission checks stay flat and explicit in `package.toml` and `services/permissions.zun`.

**Tech Stack:** Zun HTTP routes, `$HTML().link(...)`, `$STATIC(...)`, `std.postgres`, `std.auth.guard`, cookie session, JWT bearer, CSRF, PostgreSQL, Rust integration tests in `tests/integration_suite.rs`

---

## File Structure

- Modify: `sample/auth-b2b-portal/package.toml`
  - add news/admin authz rules
- Modify: `sample/auth-b2b-portal/main.zun`
  - mount news page router and news API router
- Modify: `sample/auth-b2b-portal/services/permissions.zun`
  - extend role -> permission mapping to `editor`
- Create: `sample/auth-b2b-portal/services/news.zun`
  - news query/create/update/publish/unpublish logic
- Create: `sample/auth-b2b-portal/app/router/news_page_router.zun`
  - page shell routes for `/news`, `/news/:slug`, `/news-admin`
- Create: `sample/auth-b2b-portal/app/router/news_api_router.zun`
  - public and admin news APIs
- Create: `sample/auth-b2b-portal/app/pages/news.html`
  - public news index shell
- Create: `sample/auth-b2b-portal/app/pages/news_detail.html`
  - public news detail shell
- Create: `sample/auth-b2b-portal/app/pages/news_admin.html`
  - admin console shell
- Create: `sample/auth-b2b-portal/app/public/css/news.css`
  - public news styling
- Create: `sample/auth-b2b-portal/app/public/css/news_admin.css`
  - admin styling
- Create: `sample/auth-b2b-portal/app/public/js/news.js`
  - public list page fetch/render
- Create: `sample/auth-b2b-portal/app/public/js/news_detail.js`
  - public detail page fetch/render
- Create: `sample/auth-b2b-portal/app/public/js/news_admin.js`
  - admin CRUD/publish UI
- Modify: `sample/auth-b2b-portal/README.md`
  - document new routes, SQL, seed users, verification
- Modify: `sample/auth-b2b-portal/scripts/smoke.sh`
  - add news verification path
- Modify: `test-auth-plan.md`
  - add continuous SQL for `news_categories` and `news_articles`, plus `editor` seed user
- Modify: `tests/integration_suite.rs`
  - add sample structure tests and live auth/news flow tests

### Task 1: Extend Sample Data Model And Role Matrix

**Files:**
- Modify: `sample/auth-b2b-portal/services/permissions.zun`
- Modify: `test-auth-plan.md`
- Modify: `sample/auth-b2b-portal/README.md`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing test**

Add a new structure test in `tests/integration_suite.rs`:

```rust
#[test]
fn sample_auth_b2b_portal_news_plan_mentions_editor_and_news_tables() {
    let readme = fs::read_to_string(sample_project_path("auth-b2b-portal/README.md")).unwrap();
    let plan = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-auth-plan.md")).unwrap();

    assert!(readme.contains("editor"));
    assert!(plan.contains("news_categories"));
    assert!(plan.contains("news_articles"));
    assert!(plan.contains("news:create"));
    assert!(plan.contains("news:publish"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_news_plan_mentions_editor_and_news_tables -- --nocapture`

Expected: FAIL because the current README and `test-auth-plan.md` do not mention the `editor` role or news tables.

- [ ] **Step 3: Write minimal implementation**

Update the docs and SQL plan to include:

```sql
CREATE TABLE IF NOT EXISTS news_categories (
    category_id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS news_articles (
    article_id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    body TEXT NOT NULL,
    category_id TEXT NOT NULL REFERENCES news_categories(category_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'published')),
    author_user_id TEXT NOT NULL REFERENCES app_users(user_id),
    published_at BIGINT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);
```

Update the role mapping docs to state:

```text
admin  -> news:create, news:edit, news:publish, news:delete, project:create, audit:read
editor -> news:create, news:edit, news:publish
member -> project:read
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_news_plan_mentions_editor_and_news_tables -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add test-auth-plan.md sample/auth-b2b-portal/README.md tests/integration_suite.rs
git commit -m "docs: define news sample data model and roles"
```

### Task 2: Add News Service, Routers, And Auth Rules

**Files:**
- Modify: `sample/auth-b2b-portal/package.toml`
- Modify: `sample/auth-b2b-portal/main.zun`
- Modify: `sample/auth-b2b-portal/services/permissions.zun`
- Create: `sample/auth-b2b-portal/services/news.zun`
- Create: `sample/auth-b2b-portal/app/router/news_page_router.zun`
- Create: `sample/auth-b2b-portal/app/router/news_api_router.zun`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing test**

Add a new structure test:

```rust
#[test]
fn sample_auth_b2b_portal_declares_news_routes_and_auth_rules() {
    let project_dir = sample_project_path("auth-b2b-portal");
    let main_source = fs::read_to_string(project_dir.join("main.zun")).unwrap();
    let manifest = ProjectConfig::load_from_dir(&project_dir).unwrap();

    assert!(main_source.contains("news_page_router"));
    assert!(main_source.contains("news_api_router"));
    assert!(project_dir.join("services/news.zun").exists());
    assert!(project_dir.join("app/router/news_page_router.zun").exists());
    assert!(project_dir.join("app/router/news_api_router.zun").exists());
    assert!(manifest.server.auth.authorization.rules.iter().any(|rule| rule.path == "/news-admin"));
    assert!(manifest.server.auth.authorization.rules.iter().any(|rule| rule.path == "/api/admin/news"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_declares_news_routes_and_auth_rules -- --nocapture`

Expected: FAIL because none of the news files or auth rules exist yet.

- [ ] **Step 3: Write minimal implementation**

Add the router mounts in `main.zun`:

```zun
$HTTP("/").link("app.router.news_page_router");
$HTTP("/api").link("app.router.news_api_router");
```

Add auth rules in `package.toml`:

```toml
[[server.auth.authorization.rules]]
method = "GET"
path = "/news-admin"
require = "authenticated"
permissions_any = ["news:create", "news:edit", "news:publish"]

[[server.auth.authorization.rules]]
method = "GET"
path = "/api/admin/news"
require = "authenticated"
permissions_all = ["news:edit"]

[[server.auth.authorization.rules]]
method = "POST"
path = "/api/admin/news"
require = "authenticated"
permissions_all = ["news:create"]

[[server.auth.authorization.rules]]
method = "PUT"
path = "/api/admin/news/*"
require = "authenticated"
permissions_all = ["news:edit"]

[[server.auth.authorization.rules]]
method = "POST"
path = "/api/admin/news/*/publish"
require = "authenticated"
permissions_all = ["news:publish"]

[[server.auth.authorization.rules]]
method = "POST"
path = "/api/admin/news/*/unpublish"
require = "authenticated"
permissions_all = ["news:publish"]
```

In `services/permissions.zun`, extend the mapping to include `editor`.

In `services/news.zun`, implement these functions:

```zun
$fn list_public_articles() { ... }
$fn get_public_article(slug) { ... }
$fn list_admin_articles() { ... }
$fn create_article(payload, principal) { ... }
$fn update_article(article_id, payload) { ... }
$fn publish_article(article_id) { ... }
$fn unpublish_article(article_id) { ... }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_declares_news_routes_and_auth_rules -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add sample/auth-b2b-portal/package.toml sample/auth-b2b-portal/main.zun sample/auth-b2b-portal/services/permissions.zun sample/auth-b2b-portal/services/news.zun sample/auth-b2b-portal/app/router/news_page_router.zun sample/auth-b2b-portal/app/router/news_api_router.zun tests/integration_suite.rs
git commit -m "feat: add news routers and auth rules to auth sample"
```

### Task 3: Build Public News Pages

**Files:**
- Create: `sample/auth-b2b-portal/app/pages/news.html`
- Create: `sample/auth-b2b-portal/app/pages/news_detail.html`
- Create: `sample/auth-b2b-portal/app/public/css/news.css`
- Create: `sample/auth-b2b-portal/app/public/js/news.js`
- Create: `sample/auth-b2b-portal/app/public/js/news_detail.js`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing test**

Add a new sample asset test:

```rust
#[test]
fn sample_auth_b2b_portal_exposes_public_news_pages_and_assets() {
    let project_dir = sample_project_path("auth-b2b-portal");
    let html = fs::read_to_string(project_dir.join("app/pages/news.html")).unwrap();
    let detail = fs::read_to_string(project_dir.join("app/pages/news_detail.html")).unwrap();
    let js = fs::read_to_string(project_dir.join("app/public/js/news.js")).unwrap();
    let detail_js = fs::read_to_string(project_dir.join("app/public/js/news_detail.js")).unwrap();

    assert!(html.contains("/assets/css/news.css"));
    assert!(html.contains("/assets/js/news.js"));
    assert!(detail.contains("/assets/js/news_detail.js"));
    assert!(js.contains("/api/news"));
    assert!(detail_js.contains("/api/news/"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_exposes_public_news_pages_and_assets -- --nocapture`

Expected: FAIL because the files do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Page shells:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Zun News</title>
  <link rel="stylesheet" href="/assets/css/news.css">
</head>
<body>
  <main id="news-app"></main>
  <script src="/assets/js/news.js"></script>
</body>
</html>
```

Public JS must fetch:

```js
fetch("/api/news")
fetch(`/api/news/${slug}`)
```

Implement `/news` and `/news/:slug` page handlers in `news_page_router.zun`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_exposes_public_news_pages_and_assets -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add sample/auth-b2b-portal/app/pages/news.html sample/auth-b2b-portal/app/pages/news_detail.html sample/auth-b2b-portal/app/public/css/news.css sample/auth-b2b-portal/app/public/js/news.js sample/auth-b2b-portal/app/public/js/news_detail.js tests/integration_suite.rs
git commit -m "feat: add public news pages to auth sample"
```

### Task 4: Build News Admin Console

**Files:**
- Create: `sample/auth-b2b-portal/app/pages/news_admin.html`
- Create: `sample/auth-b2b-portal/app/public/css/news_admin.css`
- Create: `sample/auth-b2b-portal/app/public/js/news_admin.js`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing test**

Add a structure test:

```rust
#[test]
fn sample_auth_b2b_portal_exposes_news_admin_console_assets() {
    let project_dir = sample_project_path("auth-b2b-portal");
    let html = fs::read_to_string(project_dir.join("app/pages/news_admin.html")).unwrap();
    let js = fs::read_to_string(project_dir.join("app/public/js/news_admin.js")).unwrap();

    assert!(html.contains("/assets/css/news_admin.css"));
    assert!(html.contains("/assets/js/news_admin.js"));
    assert!(js.contains("/api/admin/news"));
    assert!(js.contains("/publish"));
    assert!(js.contains("X-CSRF-Token"));
    assert!(js.contains("transport === \"bearer\" ? \"omit\" : \"include\""));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_exposes_news_admin_console_assets -- --nocapture`

Expected: FAIL because the admin page and script do not exist yet.

- [ ] **Step 3: Write minimal implementation**

The admin page should provide:

```html
<main class="news-admin-shell">
  <section id="identity-panel"></section>
  <section id="article-list"></section>
  <section id="editor-panel"></section>
  <section id="response-panel"></section>
</main>
```

The admin JS should implement:

```js
fetch("/api/admin/news", { credentials: "include" });
fetch("/api/admin/news", { method: "POST", headers: { "X-CSRF-Token": csrfToken } });
fetch(`/api/admin/news/${id}/publish`, { method: "POST", headers: { Authorization: `Bearer ${accessToken}` } });
```

Keep the same transport split rule as `console.js`:

```js
credentials: transport === "bearer" ? "omit" : "include"
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_exposes_news_admin_console_assets -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add sample/auth-b2b-portal/app/pages/news_admin.html sample/auth-b2b-portal/app/public/css/news_admin.css sample/auth-b2b-portal/app/public/js/news_admin.js tests/integration_suite.rs
git commit -m "feat: add news admin console to auth sample"
```

### Task 5: Add Live Auth And News Regression Coverage

**Files:**
- Modify: `tests/integration_suite.rs`
- Modify: `sample/auth-b2b-portal/scripts/smoke.sh`
- Modify: `sample/auth-b2b-portal/README.md`

- [ ] **Step 1: Write the failing test**

Add an env-gated live test:

```rust
#[test]
fn sample_auth_b2b_portal_news_role_matrix_works_when_env_is_available() {
    let Some(url) = std::env::var("ZUN_TEST_POSTGRES_URL").ok() else {
        return;
    };
    let _guard = sample_auth_env_lock().lock().unwrap();
    unsafe {
        std::env::set_var("SESSION_DATABASE_URL", &url);
        std::env::set_var("APP_DATABASE_URL", &url);
        std::env::set_var("JWT_SECRET", "sample-auth-secret");
    }

    let outcome = run_program_at_path(&sample_project_path("auth-b2b-portal"), RuntimeMode::Serve);
    assert!(outcome.error.is_none());

    // login as member -> POST /api/admin/news => 403
    // login as editor -> create draft => 200/201
    // publish => 200
    // GET /api/news sees published slug
    // cookie POST without csrf => 403
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_suite sample_auth_b2b_portal_news_role_matrix_works_when_env_is_available -- --nocapture`

Expected: FAIL once the env is present because news routes and DB behavior are not complete yet.

- [ ] **Step 3: Write minimal implementation**

Extend `sample/auth-b2b-portal/scripts/smoke.sh` to add:

```bash
echo "[6] member create news should fail"
echo "[7] editor create news draft"
echo "[8] editor publish news"
echo "[9] public news list shows published slug"
```

Update `README.md` with:

```text
GET /news
GET /news/:slug
GET /news-admin
GET /api/news
GET /api/news/:slug
GET /api/admin/news
POST /api/admin/news
PUT /api/admin/news/:id
POST /api/admin/news/:id/publish
POST /api/admin/news/:id/unpublish
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test integration_suite sample_auth_b2b_portal_news_role_matrix_works_when_env_is_available -- --nocapture
bash sample/auth-b2b-portal/scripts/smoke.sh
```

Expected:

- the env-gated integration test passes
- smoke script can create and publish a news article
- public `/api/news` only exposes `published`

- [ ] **Step 5: Commit**

```bash
git add tests/integration_suite.rs sample/auth-b2b-portal/scripts/smoke.sh sample/auth-b2b-portal/README.md
git commit -m "test: cover news auth flows in auth sample"
```

## Self-Review

- Spec coverage:
  - public news pages: Task 3
  - news admin page: Task 4
  - role and permission expansion: Task 1 + Task 2
  - new DB tables: Task 1
  - authz rules: Task 2
  - live behavior verification: Task 5
- Placeholder scan:
  - no `TODO`, `TBD`, or “similar to above” references remain
- Type consistency:
  - route names, permission keys, and table names are consistent across tasks:
    - `news:create`
    - `news:edit`
    - `news:publish`
    - `news_categories`
    - `news_articles`

