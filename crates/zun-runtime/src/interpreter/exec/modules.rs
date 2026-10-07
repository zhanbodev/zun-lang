use std::borrow::ToOwned;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ast::{ModDeclStmt, Span, Spanned, StaticStmt};
use crate::diagnostics::{Diagnostic, codes};
use crate::error::Error;
use crate::module::{ModuleResolver, ResolvedModule};
use crate::runtime::context::ModuleNamespaceCacheEntry;
use crate::runtime::{ProgramState, RuntimeContext, execute_program_with_writer};

use super::super::env::{Env, FnEnv};
use super::super::value::ZunValue;
use super::super::{ModuleNamespace, StaticRoute};
use super::{Flow, info_prefix};

pub(super) fn handle_mod_decl(
    stmt: &ModDeclStmt,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    let resolver = ModuleResolver::new(context.project_root().to_path_buf())
        .with_current_file(context.current_file().map(Path::new).map(Path::to_path_buf))
        .with_manifest(context.project_config().cloned());

    let modules = if stmt.wildcard {
        match resolve_wildcard_modules(&resolver, &stmt.path) {
            Ok(modules) => modules,
            Err(err) => return Flow::Err(module_error(stmt.span(), err.to_string())),
        }
    } else if let Some(resolved) = resolver.resolve_module(&stmt.path) {
        vec![resolved]
    } else if let Some(native_exports) = context.native_module(&stmt.path) {
        // Native module: build ModuleProxy from the registry and register directly.
        let namespace = stmt
            .path
            .rsplit('.')
            .next()
            .unwrap_or(&stmt.path)
            .to_string();
        if state.env.contains_key(&namespace) {
            return Flow::Err(module_error(
                stmt.span(),
                format!("module namespace conflict: '{}' already exists", namespace),
            ));
        }
        let module_value = ZunValue::ModuleProxy {
            path: stmt.path.clone(),
            state: Arc::new(ModuleNamespace {
                exports: FnEnv::new(),
                fns: FnEnv::new(),
                native_exports: native_exports.clone(),
                module_env: Arc::new(Env::new()),
                visible_user_types: Arc::new(HashSet::new()),
            }),
        };
        state.insert_env(namespace, module_value);
        return Flow::Normal;
    } else {
        return Flow::Err(module_error(
            stmt.span(),
            format!(
                "module not found: '{}' (tried: {})",
                stmt.path,
                resolver.describe_search_order(&stmt.path)
            ),
        ));
    };

    for resolved in modules {
        let namespace_name = {
            let stem = resolved
                .file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            // services/mod.zun → namespace "services" (parent dir name)
            if stem == "mod" {
                resolved
                    .file_path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or(stem)
                    .to_string()
            } else {
                stem.to_string()
            }
        };
        if namespace_name.is_empty() {
            return Flow::Err(module_error(
                stmt.span(),
                format!(
                    "invalid module file name: '{}'",
                    resolved.file_path.display()
                ),
            ));
        }

        let module_value = match build_module_namespace(&resolved, context, w) {
            Ok(module_value) => module_value,
            Err(err) => return Flow::Err(module_error(stmt.span(), err.to_string())),
        };

        if let Err(err) = register_module_namespace(state, &namespace_name, module_value, &resolved)
        {
            return Flow::Err(module_error(stmt.span(), err.to_string()));
        }
    }

    Flow::Normal
}

fn module_error(span: Span, message: impl Into<String>) -> Error {
    Error::Diagnostic(Diagnostic::error(codes::RUNTIME_MODULE_LOAD, message).with_span(span))
}

fn resolve_wildcard_modules(
    resolver: &ModuleResolver,
    module_path: &str,
) -> Result<Vec<ResolvedModule>, Error> {
    let Some(directory) = resolver.resolve_module_directory(module_path) else {
        return Err(Error::Interpreter(format!(
            "module directory not found: '{}.*' (tried: {})",
            module_path,
            resolver.describe_directory_search_order(module_path)
        )));
    };

    let mut modules = Vec::new();
    let read_dir = fs::read_dir(&directory.file_path).map_err(|err| {
        Error::Interpreter(format!(
            "cannot read module directory '{}': {}",
            directory.file_path.display(),
            err
        ))
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|err| {
            Error::Interpreter(format!(
                "cannot scan module directory '{}': {}",
                directory.file_path.display(),
                err
            ))
        })?;
        let file_path = entry.path();
        if !file_path.is_file() || file_path.extension().and_then(|ext| ext.to_str()) != Some("zun")
        {
            continue;
        }

        let file_stem = file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                Error::Interpreter(format!(
                    "invalid module file name: '{}'",
                    file_path.display()
                ))
            })?;

        let resolved_path = if module_path.is_empty() {
            file_stem.to_string()
        } else {
            format!("{module_path}.{file_stem}")
        };

        modules.push(ResolvedModule {
            module_path: resolved_path,
            file_path,
            namespace: directory.namespace.clone(),
        });
    }

    modules.sort_by(|left, right| left.file_path.cmp(&right.file_path));

    if modules.is_empty() {
        return Err(Error::Interpreter(format!(
            "module directory '{}' contains no importable modules",
            directory.file_path.display()
        )));
    }

    Ok(modules)
}

fn build_module_namespace(
    resolved: &ResolvedModule,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Result<ZunValue, Error> {
    let cache_key = resolved.file_path.to_string_lossy().into_owned();
    if let Some(cached) = context.cached_module_namespace(&cache_key).map_err(|err| {
        Error::Interpreter(format!(
            "module cache error for '{}': {}",
            resolved.module_path, err
        ))
    })? {
        match cached {
            ModuleNamespaceCacheEntry::Ready(cached) => {
                return Ok(ZunValue::ModuleProxy {
                    path: resolved.module_path.clone(),
                    state: cached,
                });
            }
            ModuleNamespaceCacheEntry::Loading { module_path } => {
                return Err(Error::Interpreter(format!(
                    "circular module import detected: '{}' re-imports '{}' while it is already loading",
                    current_module_hint(context),
                    module_path
                )));
            }
        }
    }

    context
        .mark_module_namespace_loading(cache_key.clone(), resolved.module_path.clone())
        .map_err(|err| {
            Error::Interpreter(format!(
                "module cache error for '{}': {}",
                resolved.module_path, err
            ))
        })?;

    let module_result = (|| -> Result<ZunValue, Error> {
        let content = fs::read_to_string(&resolved.file_path).map_err(|err| {
            Error::Interpreter(format!(
                "cannot read module '{}': {}",
                resolved.module_path, err
            ))
        })?;

        let module_stmts = crate::parser::parse(&content).map_err(|err| {
            Error::Interpreter(format!(
                "parse error in module '{}': {}",
                resolved.module_path, err
            ))
        })?;

        // 保存并切换 current_file，使模块内的相对 $mod 解析正确
        let prev_file = context.current_file().map(ToOwned::to_owned);
        context.set_current_file(Some(resolved.file_path.to_string_lossy().to_string()));

        // 完整执行模块文件，捕获执行后的状态
        let mut module_state = ProgramState::new();
        let exec_result = execute_program_with_writer(&module_stmts, &mut module_state, context, w);

        // 恢复 current_file
        context.set_current_file(prev_file);

        exec_result.map_err(|err| {
            Error::Interpreter(format!(
                "error in module '{}': {}",
                resolved.module_path, err
            ))
        })?;

        // 从执行后的 fns 提取导出（is_public == true 的函数）
        let fns = module_state.fns.clone();
        let mut exports = FnEnv::new();
        for (name, fn_decl) in &fns {
            if fn_decl.decl.is_public {
                exports.insert(name.clone(), fn_decl.clone());
            }
        }

        let state = Arc::new(ModuleNamespace {
            exports,
            fns,
            native_exports: crate::runtime::NativeFnMap::new(),
            module_env: Arc::new(module_state.env),
            visible_user_types: Arc::new(HashSet::new()),
        });
        context
            .cache_module_namespace(cache_key.clone(), Arc::clone(&state))
            .map_err(|err| {
                Error::Interpreter(format!(
                    "module cache error for '{}': {}",
                    resolved.module_path, err
                ))
            })?;

        Ok(ZunValue::ModuleProxy {
            path: resolved.module_path.clone(),
            state,
        })
    })();

    if module_result.is_err() {
        let _ = context.clear_module_namespace_cache_entry(&cache_key);
    }

    module_result
}

fn current_module_hint(context: &RuntimeContext) -> String {
    let Some(current_file) = context.current_file() else {
        return "<entrypoint>".to_string();
    };

    let current_path = Path::new(current_file);
    if let Ok(relative) = current_path.strip_prefix(context.project_root()) {
        let mut relative = relative.to_string_lossy().replace('\\', "/");
        if relative.ends_with(".zun") {
            relative.truncate(relative.len() - 4);
        }
        return relative.replace('/', ".");
    }

    current_file.to_string()
}

fn register_module_namespace(
    state: &mut ProgramState,
    namespace_name: &str,
    module_value: ZunValue,
    resolved: &ResolvedModule,
) -> Result<(), Error> {
    if let Some(existing) = state.env.get(namespace_name) {
        return Err(Error::Interpreter(format!(
            "module namespace conflict: '{}' from '{}' collides with existing value '{}'",
            namespace_name,
            resolved.file_path.display(),
            existing.type_name()
        )));
    }

    if state.fns.contains_key(namespace_name) {
        return Err(Error::Interpreter(format!(
            "module namespace conflict: '{}' from '{}' collides with existing function '{}'",
            namespace_name,
            resolved.file_path.display(),
            namespace_name
        )));
    }

    if state.const_env.contains_key(namespace_name) {
        return Err(Error::Interpreter(format!(
            "module namespace conflict: '{}' from '{}' collides with existing constant '{}'",
            namespace_name,
            resolved.file_path.display(),
            namespace_name
        )));
    }

    state.insert_env(namespace_name.to_string(), module_value);
    Ok(())
}

pub(super) fn handle_static_stmt(
    stmt: &StaticStmt,
    context: &mut RuntimeContext,
    w: &mut dyn Write,
) -> Flow {
    let base_dir = context
        .current_file()
        .and_then(|f| Path::new(f).parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let static_route = StaticRoute {
        url_prefix: stmt.url_prefix.clone(),
        module_path: stmt.module_path.clone(),
        base_dir: base_dir.display().to_string(),
    };

    context.register_static_route(static_route);
    writeln!(
        w,
        "{} Static route registered: {} -> {}",
        info_prefix(context),
        stmt.url_prefix,
        stmt.module_path
    )
    .ok();
    Flow::Normal
}
