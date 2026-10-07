use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Stmt;
use crate::diagnostics::{Diagnostic, codes};
use crate::error::Error;
use crate::module::{
    ProjectConfig, resolve_main_file as resolve_manifest_main_file,
    resolve_project_root as resolve_manifest_project_root,
};
use crate::parser;
use crate::runtime::auth::validate_runtime_auth_config;

use super::{RuntimeContext, RuntimeMode};

#[derive(Debug, Clone)]
pub struct LoadedProgram {
    pub path: PathBuf,
    pub source: String,
    pub statements: Vec<Stmt>,
}

pub fn resolve_main_file(path: &Path) -> PathBuf {
    let project_root = resolve_project_root(path);
    let manifest = ProjectConfig::load_from_dir(&project_root);
    resolve_manifest_main_file(path, &project_root, manifest.as_ref())
}

pub fn resolve_project_root(path: &Path) -> PathBuf {
    resolve_manifest_project_root(path)
}

pub fn load_program_from_path(path: &Path) -> Result<LoadedProgram, Error> {
    let resolved_path = path.to_path_buf();
    let source = fs::read_to_string(&resolved_path).map_err(|err| {
        Error::from(
            Diagnostic::error(
                codes::PROJECT_LOAD,
                format!("cannot read file '{}': {}", resolved_path.display(), err),
            )
            .with_file(resolved_path.display().to_string()),
        )
    })?;
    let statements =
        parser::parse(&source).map_err(|err| err.with_file(resolved_path.display().to_string()))?;

    Ok(LoadedProgram {
        path: resolved_path,
        source,
        statements,
    })
}

pub fn load_context_and_program(
    mode: RuntimeMode,
    path: &Path,
) -> Result<(RuntimeContext, LoadedProgram), Error> {
    let (project_root, manifest, main_file) = match mode {
        RuntimeMode::Run => {
            let main_file = if path.is_dir() {
                path.join("main.zun")
            } else {
                path.to_path_buf()
            };
            let project_root = if path.is_dir() {
                path.to_path_buf()
            } else {
                path.parent().unwrap_or(Path::new(".")).to_path_buf()
            };
            (project_root, None, main_file)
        }
        _ => {
            let project_root = resolve_project_root(path);
            let manifest = ProjectConfig::load_from_dir(&project_root);
            let main_file = resolve_manifest_main_file(path, &project_root, manifest.as_ref());
            (project_root, manifest, main_file)
        }
    };
    if !main_file.exists() {
        return Err(Error::from(
            Diagnostic::error(
                codes::PROJECT_LOAD,
                format!("main file not found: {}", main_file.display()),
            )
            .with_file(main_file.display().to_string()),
        ));
    }

    let mut context = RuntimeContext::new(mode, project_root.clone());
    crate::stdlib_native::register_stdlib_native_modules(&mut context);
    context.set_project_config(manifest);
    validate_server_config(context.project_config())?;
    validate_runtime_auth_config(context.runtime_auth_config())
        .map_err(|err| Error::Interpreter(err.to_string()))?;
    context.set_current_file(Some(main_file.to_string_lossy().to_string()));

    let program = load_program_from_path(&main_file)?;
    let global_cors = program.statements.iter().find_map(|stmt| match stmt {
        Stmt::MainDecl(main) => main.global_cors.clone(),
        _ => None,
    });
    context.set_global_cors(global_cors);
    Ok((context, program))
}

fn validate_server_config(config: Option<&ProjectConfig>) -> Result<(), Error> {
    let Some(config) = config else {
        return Ok(());
    };

    match config.server.host.as_str() {
        "127.0.0.1" | "0.0.0.0" => Ok(()),
        other => Err(Error::from(
            Diagnostic::error(codes::PROJECT_LOAD, "invalid server configuration").with_note(
                format!("server.host must be \"127.0.0.1\" or \"0.0.0.0\", got \"{other}\""),
            ),
        )),
    }
}
