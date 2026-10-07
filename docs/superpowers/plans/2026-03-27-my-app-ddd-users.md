# my-app DDD Users Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a DDD-style `my-app` example with a dedicated `data/` layer, fake user records in `services/`, and HTTP routes linked through `$HTTP(...).link(...)`, while exercising the latest `$Type` instance and hidden-field behavior.

**Architecture:** Keep `my-app/main.zun` as a thin HTTP entrypoint. Move the user data model into `my-app/data/users_data.zun`, keep fake-record query logic in `my-app/services/users_services.zun`, and keep HTTP-only glue in `my-app/routers/users_routers.zun`. Validate behavior through a focused Rust integration test that boots the app and makes real HTTP requests.

**Tech Stack:** Zun modules and HTTP routes, Rust integration tests in `tests/integration_suite.rs`, existing Axum-backed serve mode

---

### Task 1: Add the Failing Integration Test for my-app

**Files:**
- Modify: `tests/integration_suite.rs`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the failing test**

Add a focused integration test near the end of `tests/integration_suite.rs`:

```rust
#[test]
fn my_app_users_routes_follow_ddd_layout_and_hide_private_fields() {
    let outcome = support::run_program_at_path(
        std::path::Path::new("my-app"),
        RuntimeMode::Serve,
    );
    assert!(outcome.error.is_none(), "my-app should boot");

    let base_url = start_live_http_server_from_context(outcome.context);

    let list_response = http_get(&base_url, "/v1/api/users");
    assert_eq!(list_response.status, 200);
    assert!(list_response.body.contains("\"id\":1"));
    assert!(list_response.body.contains("\"role\":\"admin\""));
    assert!(!list_response.body.contains("_password"));
    assert!(!list_response.body.contains("_salt"));

    let one_response = http_get(&base_url, "/v1/api/users/1");
    assert_eq!(one_response.status, 200);
    assert!(one_response.body.contains("\"name\":\"John\""));
    assert!(!one_response.body.contains("_password"));

    let missing_response = http_get(&base_url, "/v1/api/users/999");
    assert_eq!(missing_response.status, 200);
    assert_eq!(missing_response.body, r#"{"Null":"No data"}"#);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`

Expected: FAIL because `my-app` still uses the old service/type layout and does not expose the planned `/users` and `/users/:id` behavior.

- [ ] **Step 3: Commit the red test**

```bash
git add tests/integration_suite.rs
git commit -m "test: add my-app ddd users integration coverage"
```

### Task 2: Create the Dedicated Data Layer

**Files:**
- Create: `my-app/data/users_data.zun`
- Modify: `my-app/services/users_services.zun`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Write the data model file**

Create `my-app/data/users_data.zun` with:

```zun
$Type User {
    id: Int
    name: String
    email: String
    role: String
    _password: String
    _salt: String
}
```

- [ ] **Step 2: Wire the service module to import the new type**

Replace the top of `my-app/services/users_services.zun` so it imports the type instead of declaring it inline:

```zun
$mod data.users_data;
```

The service should refer to the imported type as `users_data.User`.

- [ ] **Step 3: Run the focused test to verify it still fails for the right reason**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`

Expected: FAIL because routes and service behavior are still incomplete, not because the module fails to parse.

- [ ] **Step 4: Commit the data-layer split**

```bash
git add my-app/data/users_data.zun my-app/services/users_services.zun
git commit -m "refactor: move my-app user type into data layer"
```

### Task 3: Implement the Service-Layer Fake Data and Queries

**Files:**
- Modify: `my-app/services/users_services.zun`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Replace the old service body with fake records and query functions**

Rewrite `my-app/services/users_services.zun` to:

```zun
$mod data.users_data;

$fn all_users() {
    $# [
        users_data.User {
            id: 1,
            name: "John",
            email: "john@example.com",
            role: "admin",
            _password: "hash_john",
            _salt: "salt_john",
        },
        users_data.User {
            id: 2,
            name: "Alice",
            email: "alice@example.com",
            role: "editor",
            _password: "hash_alice",
            _salt: "salt_alice",
        },
        users_data.User {
            id: 3,
            name: "Bob",
            email: "bob@example.com",
            role: "viewer",
            _password: "hash_bob",
            _salt: "salt_bob",
        }
    ];
}

$fn find_all_users() -> JSON<List> {
    $# all_users();
}

$fn find_user_by_id(id) -> JSON<User> {
    $ users = all_users();

    $for user in users {
        $if user.id == id {
            $# user;
        }
    }

    $# { "Null": "No data" };
}
```

- [ ] **Step 2: Run the focused test to verify the service behavior is still blocked only by routing**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`

Expected: FAIL because the routers still expose the old endpoint shape.

- [ ] **Step 3: Commit the service-layer implementation**

```bash
git add my-app/services/users_services.zun
git commit -m "feat: add my-app user service fake queries"
```

### Task 4: Implement the Router Layer and HTTP Link Shape

**Files:**
- Modify: `my-app/routers/users_routers.zun`
- Modify: `my-app/main.zun`
- Test: `tests/integration_suite.rs`

- [ ] **Step 1: Replace the router file with HTTP-only route glue**

Rewrite `my-app/routers/users_routers.zun` to:

```zun
$mod services.users_services;

$GET("/users") getUsers() -> JSON<List> {
    $# users_services.find_all_users();
}

$GET("/users/:id") getUserById(id) -> JSON<User> {
    $# users_services.find_user_by_id(id);
}
```

- [ ] **Step 2: Keep the app entrypoint minimal**

Ensure `my-app/main.zun` is:

```zun
$main() {
    $HTTP("/v1/api").link("routers.users_routers");
}
```

- [ ] **Step 3: Run the focused test to verify it passes**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`

Expected: PASS

- [ ] **Step 4: Commit the router wiring**

```bash
git add my-app/main.zun my-app/router/users_routers.zun
git commit -m "feat: expose my-app user ddd routes"
```

### Task 5: Run Full Verification

**Files:**
- Verify only: `my-app/data/users_data.zun`
- Verify only: `my-app/services/users_services.zun`
- Verify only: `my-app/routers/users_routers.zun`
- Verify only: `my-app/main.zun`
- Verify only: `tests/integration_suite.rs`

- [ ] **Step 1: Run targeted integration coverage**

Run: `cargo test --test integration_suite my_app_users_routes_follow_ddd_layout_and_hide_private_fields -- --nocapture`

Expected: PASS

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`

Expected: PASS with all existing suites green.

- [ ] **Step 3: Review the final my-app layout**

Check:

```bash
find my-app -maxdepth 3 -print | sort
```

Expected to include:

```text
my-app
my-app/data
my-app/data/users_data.zun
my-app/main.zun
my-app/routers
my-app/routers/users_routers.zun
my-app/services
my-app/services/users_services.zun
```

- [ ] **Step 4: Commit the verified example**

```bash
git add my-app tests/integration_suite.rs
git commit -m "feat: add ddd-style my-app users example"
```

## Self-Review

- Spec coverage: the plan covers the `data/`, `services/`, `routers/`, and `main.zun` split, both HTTP routes, the hidden-field JSON behavior, and the `200 + {"Null":"No data"}` missing-user contract.
- Placeholder scan: no `TODO`, `TBD`, or implicit “write tests later” steps remain.
- Type consistency: all planned names use the same module and function identifiers throughout:
  - `data.users_data`
  - `users_services.find_all_users()`
  - `users_services.find_user_by_id(id)`
  - routes `/users` and `/users/:id`
