use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use zun::interpreter::ZunValue;
use zun::module::{ModuleNamespace, ModuleResolver, ProjectConfig};
use zun::runtime::{
    ProgramState, RuntimeMode, execute_program_with_writer, load_context_and_program,
};

fn unique_project_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be valid")
        .as_nanos();
    std::env::temp_dir().join(format!("zun-{prefix}-{unique}"))
}

#[test]
fn manifest_schema_parses_sections() {
    let manifest = r#"
[project]
name = "phase4"
version = "0.1.0"
entry = "app/main.zun"

[server]
host = "127.0.0.1"
port = 9090

[env]
APP_ENV = "dev"
DB_URL = "sqlite://demo.db"

[dependencies]
acme = "0.1.0"
"#;

    let config = ProjectConfig::parse_toml(manifest).expect("manifest should parse");
    assert_eq!(config.name, "phase4");
    assert_eq!(config.version, "0.1.0");
    assert_eq!(config.entry, "app/main.zun");
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 9090);
    assert_eq!(config.get("APP_ENV"), Some("dev"));
    assert_eq!(config.get("DB_URL"), Some("sqlite://demo.db"));
    assert_eq!(config.dependencies.get("acme"), Some(&"0.1.0".to_string()));
}

#[test]
fn manifest_schema_keeps_legacy_top_level_project_fields_compatible() {
    let manifest = r#"
name = "phase4-legacy"
version = "0.1.0"
entry = "app/main.zun"

[server]
host = "127.0.0.1"
port = 9090
"#;

    let config = ProjectConfig::parse_toml(manifest).expect("manifest should parse");
    assert_eq!(config.name, "phase4-legacy");
    assert_eq!(config.version, "0.1.0");
    assert_eq!(config.entry, "app/main.zun");
    assert_eq!(config.server.host, "127.0.0.1");
}

#[test]
fn manifest_schema_defaults_host_to_localhost_loopback() {
    let manifest = r#"
[project]
name = "phase4-default-host"
version = "0.1.0"
entry = "main.zun"
"#;

    let config = ProjectConfig::parse_toml(manifest).expect("manifest should parse");
    assert_eq!(config.server.host, "127.0.0.1");
}

#[test]
fn load_context_rejects_unsupported_server_host() {
    let project_dir = unique_project_dir("phase4-bad-host");
    fs::create_dir_all(&project_dir).expect("project dir");
    fs::write(
        project_dir.join("package.toml"),
        r#"
[project]
name = "phase4-bad-host"
version = "0.1.0"
entry = "main.zun"

[server]
host = "localhost"
port = 8080
"#,
    )
    .expect("manifest");
    fs::write(project_dir.join("main.zun"), "$main() {}\n").expect("main");

    let outcome = load_context_and_program(RuntimeMode::Serve, &project_dir)
        .expect_err("unsupported host should fail during load");

    assert!(outcome.to_string().contains("server.host"));
    fs::remove_dir_all(&project_dir).expect("cleanup");
}

#[test]
fn module_resolution_priority_is_relative_then_project_then_modules() {
    let project_dir = unique_project_dir("phase4-priority");
    fs::create_dir_all(project_dir.join("features/api/shared")).expect("relative dir");
    fs::create_dir_all(project_dir.join("shared")).expect("project dir");
    fs::create_dir_all(project_dir.join("modules/shared")).expect("modules dir");
    fs::write(
        project_dir.join("package.toml"),
        "name = \"phase4\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");

    let current_file = project_dir.join("features/api/main.zun");
    fs::write(&current_file, "$ value = 1;").expect("current file");
    fs::write(
        project_dir.join("features/api/shared/tool.zun"),
        "$fn source() -> String { $# \"relative\"; }",
    )
    .expect("relative module");
    fs::write(
        project_dir.join("shared/tool.zun"),
        "$fn source() -> String { $# \"project\"; }",
    )
    .expect("project module");
    fs::write(
        project_dir.join("modules/shared/tool.zun"),
        "$fn source() -> String { $# \"modules\"; }",
    )
    .expect("modules module");

    let resolver = ModuleResolver::new(project_dir.clone())
        .with_current_file(Some(current_file.clone()))
        .with_manifest(ProjectConfig::load_from_dir(&project_dir));

    let resolved = resolver
        .resolve_module("shared.tool")
        .expect("relative module should resolve");
    assert_eq!(
        resolved.file_path,
        project_dir.join("features/api/shared/tool.zun")
    );
    assert_eq!(resolved.namespace, ModuleNamespace::Relative);

    fs::remove_file(project_dir.join("features/api/shared/tool.zun")).expect("remove relative");
    let resolved = resolver
        .resolve_module("shared.tool")
        .expect("project module should resolve");
    assert_eq!(resolved.file_path, project_dir.join("shared/tool.zun"));
    assert_eq!(resolved.namespace, ModuleNamespace::Project);

    fs::remove_file(project_dir.join("shared/tool.zun")).expect("remove project");
    let resolved = resolver
        .resolve_module("shared.tool")
        .expect("modules fallback should resolve");
    assert_eq!(
        resolved.file_path,
        project_dir.join("modules/shared/tool.zun")
    );
    assert_eq!(resolved.namespace, ModuleNamespace::Modules);

    fs::remove_dir_all(&project_dir).expect("cleanup");
}

#[test]
fn relative_imports_work_from_current_file_context() {
    let project_dir = unique_project_dir("phase4-relative");
    fs::create_dir_all(project_dir.join("app/shared")).expect("app/shared dir");
    fs::write(
        project_dir.join("package.toml"),
        "name = \"phase4\"\nversion = \"0.1.0\"\nentry = \"app/main.zun\"\n",
    )
    .expect("manifest");
    fs::write(
        project_dir.join("app/main.zun"),
        "$mod shared.helper;\n$ result = helper.answer();\n",
    )
    .expect("main");
    fs::write(
        project_dir.join("app/shared/helper.zun"),
        "$fn answer() -> Int { $# 42; }\n",
    )
    .expect("helper");

    let (mut context, program) =
        load_context_and_program(RuntimeMode::Run, &project_dir.join("app/main.zun"))
            .expect("program should load");
    let mut state = ProgramState::new();
    let mut output = Vec::new();

    let should_continue =
        execute_program_with_writer(&program.statements, &mut state, &mut context, &mut output)
            .expect("program should execute");

    assert!(should_continue);
    assert_eq!(state.env.get("result"), Some(&ZunValue::Int(42)));

    fs::remove_dir_all(&project_dir).expect("cleanup");
}

#[test]
fn std_namespace_resolves_from_stdlib() {
    let project_dir = unique_project_dir("phase4-stdlib");
    fs::create_dir_all(project_dir.join("stdlib")).expect("stdlib dir");
    fs::create_dir_all(project_dir.join("std")).expect("shadow dir");
    fs::write(
        project_dir.join("package.toml"),
        "name = \"phase4\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    fs::write(project_dir.join("main.zun"), "$ value = 1;").expect("main");
    fs::write(
        project_dir.join("stdlib/io.zun"),
        "$fn answer() -> Int { $# 7; }\n",
    )
    .expect("stdlib module");
    fs::write(
        project_dir.join("std/io.zun"),
        "$fn answer() -> Int { $# 99; }\n",
    )
    .expect("shadow module");

    let resolver = ModuleResolver::new(project_dir.clone())
        .with_current_file(Some(project_dir.join("main.zun")))
        .with_manifest(ProjectConfig::load_from_dir(&project_dir));

    let resolved = resolver
        .resolve_module("std.io")
        .expect("stdlib module should resolve");
    assert_eq!(resolved.namespace, ModuleNamespace::Stdlib);
    assert_eq!(resolved.file_path, project_dir.join("stdlib/io.zun"));

    fs::remove_dir_all(&project_dir).expect("cleanup");
}
