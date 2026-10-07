use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use zun::interpreter::ZunValue;
use zun::parse;
use zun::runtime::{
    IntrinsicRegistry, ProgramState, RuntimeContext, RuntimeMode, RuntimePolicy,
    execute_program_with_writer, execute_source_with_writer, intrinsics::ids,
    load_context_and_program,
};
use zun_runtime::stdlib_native;

fn run_program(source: &str) -> (ProgramState, RuntimeContext, String) {
    let statements = parse(source).expect("source should parse");
    let mut state = ProgramState::new();
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    stdlib_native::register_stdlib_native_modules(&mut context);
    let mut output = Vec::new();

    let should_continue =
        execute_program_with_writer(&statements, &mut state, &mut context, &mut output)
            .expect("program should execute successfully");
    assert!(
        should_continue,
        "program should not exit during baseline tests"
    );

    let output = String::from_utf8(output).expect("stdout should be utf-8");
    (state, context, output)
}

fn run_program_error(source: &str) -> String {
    let statements = parse(source).expect("source should parse");
    let mut state = ProgramState::new();
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    stdlib_native::register_stdlib_native_modules(&mut context);
    let mut output = Vec::new();

    execute_program_with_writer(&statements, &mut state, &mut context, &mut output)
        .expect_err("program should fail")
        .to_string()
}

#[test]
fn runs_minimal_script_fixture() {
    let source = include_str!("fixtures/runtime/hello.zun");
    let (_state, _context, output) = run_program(source);

    assert_eq!(output, "Hello, Zun!\n");
}

#[test]
fn tracks_variable_declaration_and_assignment() {
    let source = "$ counter = 1; counter = 2;";
    let (state, _context, _output) = run_program(source);

    assert_eq!(state.env.get("counter"), Some(&ZunValue::Int(2)));
}

#[test]
fn variable_declaration_cannot_shadow_existing_constant() {
    let error = run_program_error("$@ MAX = 300; $ MAX = 30;");

    assert!(error.contains("constant 'MAX' is already defined"));
}

#[test]
fn supports_function_definition_and_call() {
    let source = r#"
$fn add(a, b) -> Int {
    $# a + b;
}

$ result = add(1, 2);
"#;
    let (state, _context, _output) = run_program(source);

    assert_eq!(state.env.get("result"), Some(&ZunValue::Int(3)));
}

#[test]
fn evaluates_expression_precedence() {
    let source = "$ value = 1 + 2 * 3;";
    let (state, _context, _output) = run_program(source);

    assert_eq!(state.env.get("value"), Some(&ZunValue::Int(7)));
}

#[test]
fn registers_http_route_from_fixture() {
    let source = include_str!("fixtures/http/health_route.zun");
    let (_state, context, _output) = run_program(source);

    let routes = context.routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].method, "GET");
    assert_eq!(routes[0].path, "/health");
    assert_eq!(routes[0].name, "health");
}

#[test]
fn runtime_context_does_not_leak_routes_between_executions() {
    let source = include_str!("fixtures/http/health_route.zun");

    let (_state_a, context_a, _output_a) = run_program(source);
    let (_state_b, context_b, _output_b) = run_program("$ name = \"isolated\";");

    assert_eq!(context_a.routes().len(), 1);
    assert!(context_b.routes().is_empty());
}

#[test]
fn execute_source_with_writer_uses_explicit_context() {
    let mut state = ProgramState::new();
    let mut context = RuntimeContext::new(RuntimeMode::Repl, PathBuf::from("."));
    let mut output = Vec::new();

    let should_continue = execute_source_with_writer(
        "$ msg = \"hello\"; $>> msg;",
        &mut state,
        &mut context,
        &mut output,
    )
    .expect("source should execute");

    assert!(should_continue);
    assert_eq!(
        state.env.get("msg"),
        Some(&ZunValue::Str("hello".to_string()))
    );
    assert_eq!(String::from_utf8(output).expect("utf-8"), "hello\n");
}

#[test]
fn main_decl_prints_use_the_explicit_writer() {
    let source = r#"
$main() {
    $>> "[INFO] startup";
}
"#;
    let statements = parse(source).expect("source should parse");
    let mut state = ProgramState::new();
    let mut context = RuntimeContext::new(RuntimeMode::Serve, PathBuf::from("."));
    context.set_current_file(Some("main.zun".to_string()));
    let mut output = Vec::new();

    let should_continue =
        execute_program_with_writer(&statements, &mut state, &mut context, &mut output)
            .expect("program should execute successfully");

    assert!(should_continue);
    assert_eq!(
        String::from_utf8(output).expect("utf-8"),
        "[INFO] startup\n"
    );
}

#[test]
fn run_mode_ignores_package_manifest_and_uses_script_path_directly() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let project_dir = std::env::temp_dir().join(format!("zun-phase1-{unique}"));
    fs::create_dir_all(project_dir.join("scripts")).expect("temp project dir should be created");

    let package = r#"
[project]
name = "phase1"
version = "0.1.0"
entry = "app.zun"

[server]
host = "127.0.0.1"
port = 8080

[server.auth]
enabled = true

[server.auth.session.store]
driver = "postgres"

[server.auth.session.store.postgres]
url_env = "SESSION_DATABASE_URL"
table = "auth_sessions"
refresh_table = "auth_refresh_tokens"
"#;
    fs::write(project_dir.join("package.toml"), package).expect("package.toml should be written");
    fs::write(project_dir.join("app.zun"), "$ value = 42;").expect("entry file should be written");
    let script_path = project_dir.join("scripts/hash_password.zun");
    fs::write(&script_path, "$ value = 7;").expect("script file should be written");

    let (context, program) =
        load_context_and_program(RuntimeMode::Run, &script_path).expect("program should load");

    assert_eq!(program.path, script_path);
    assert_eq!(
        context.current_file(),
        Some(program.path.to_string_lossy().as_ref())
    );
    assert_eq!(
        context.project_root(),
        project_dir.join("scripts").as_path()
    );
    assert!(context.project_config().is_none());

    fs::remove_dir_all(&project_dir).expect("temp project dir should be removed");
}

#[test]
fn runtime_context_exposes_intrinsic_registry_for_file_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("zun-intrinsic-file-{unique}.txt"));
    let context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));

    context
        .call_intrinsic(
            ids::FS_WRITE_TEXT,
            &[
                ZunValue::Str(path.display().to_string()),
                ZunValue::Str("hello".to_string()),
            ],
        )
        .expect("write intrinsic should succeed");

    let content = context
        .call_intrinsic(
            ids::FS_READ_TEXT,
            &[ZunValue::Str(path.display().to_string())],
        )
        .expect("read intrinsic should succeed");
    assert_eq!(content, ZunValue::Str("hello".to_string()));

    fs::remove_file(path).expect("temp file should be removed");
}

#[test]
fn std_fs_write_append_delete_round_trip() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("zun-stdout-fs-{unique}.txt"));
    let source = format!(
        "$mod std.fs;\n\
         fs.write(\"{}\", \"hello\");\n\
         fs.append(\"{}\", \"\\nworld\");\n\
         $ text = fs.read_text(\"{}\");\n\
         $ exists_before = fs.exists(\"{}\");\n\
         $ size_before = fs.size(\"{}\");\n\
         fs.delete(\"{}\");\n\
         $ exists_after = fs.exists(\"{}\");\n",
        path.display(),
        path.display(),
        path.display(),
        path.display(),
        path.display(),
        path.display(),
        path.display()
    );

    let (state, _context, _output) = run_program(&source);
    assert_eq!(
        state.env.get("text"),
        Some(&ZunValue::Str("hello\nworld".to_string()))
    );
    assert_eq!(state.env.get("exists_before"), Some(&ZunValue::Bool(true)));
    assert_eq!(state.env.get("exists_after"), Some(&ZunValue::Bool(false)));
    assert_eq!(state.env.get("size_before"), Some(&ZunValue::Int(11)));
}

#[test]
fn std_fs_read_lines_and_is_dir_work() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("zun-stdout-fs-dir-{unique}"));
    fs::create_dir_all(&dir).expect("dir should exist");
    let file = dir.join("lines.txt");
    fs::write(&file, "a\nb\n").expect("fixture file");

    let source = format!(
        "$mod std.fs;\n\
         $ lines = fs.read_lines(\"{}\");\n\
         $ dir_flag = fs.is_dir(\"{}\");\n",
        file.display(),
        dir.display()
    );

    let (state, _context, _output) = run_program(&source);
    assert_eq!(
        state.env.get("lines"),
        Some(&ZunValue::List(vec![
            ZunValue::Str("a".to_string()),
            ZunValue::Str("b".to_string())
        ]))
    );
    assert_eq!(state.env.get("dir_flag"), Some(&ZunValue::Bool(true)));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn runtime_policy_can_block_intrinsic_calls() {
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    let mut policy = RuntimePolicy::allow_all();
    policy.deny(ids::ENV_GET);
    context.set_runtime_policy(policy);

    let error = context
        .call_intrinsic(ids::ENV_GET, &[ZunValue::Str("HOME".to_string())])
        .expect_err("denied intrinsic should fail");
    assert!(error.to_string().contains("denied by runtime policy"));
}

#[test]
fn runtime_context_can_surface_unregistered_intrinsic_error() {
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    let mut registry = IntrinsicRegistry::new();
    registry.unregister(ids::CONFIG_GET);
    context.set_intrinsic_registry(registry);

    let error = context
        .call_intrinsic(ids::CONFIG_GET, &[ZunValue::Str("APP_ENV".to_string())])
        .expect_err("missing intrinsic should fail");
    assert!(error.to_string().contains("is not registered"));
}

#[test]
fn std_fs_metadata_methods_keep_working_after_intrinsic_refactor() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("zun-file-methods-{unique}.txt"));
    let source = format!(
        "$mod std.fs;\n\
         fs.write(\"{}\", \"hello\\nworld\");\n\
         $ exists = fs.exists(\"{}\");\n\
         $ size = fs.size(\"{}\");\n\
         $ lines = fs.read_lines(\"{}\");\n",
        path.display(),
        path.display(),
        path.display(),
        path.display()
    );

    let (state, _context, _output) = run_program(&source);
    assert_eq!(state.env.get("exists"), Some(&ZunValue::Bool(true)));
    assert_eq!(state.env.get("size"), Some(&ZunValue::Int(11)));
    assert_eq!(
        state.env.get("lines"),
        Some(&ZunValue::List(vec![
            ZunValue::Str("hello".to_string()),
            ZunValue::Str("world".to_string())
        ]))
    );

    fs::remove_file(path).expect("temp file should be removed");
}
