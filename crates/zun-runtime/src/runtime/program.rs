use std::io::{self, Write};
use std::sync::Arc;

use crate::ast::Stmt;
use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::interpreter::env::{ConstEnv, Env, FnEnv, TypeEnv};
use crate::interpreter::exec::exec_with_writer;
use crate::parser;

use super::RuntimeContext;

#[derive(Debug, Default, Clone)]
pub struct ProgramState {
    pub env: Env,
    pub type_env: TypeEnv,
    pub const_env: ConstEnv,
    pub fns: FnEnv,
    env_snapshot: Option<Arc<Env>>,
    fallback_env: Option<Arc<Env>>,
    fallback_fns: Option<FnEnv>,
}

impl ProgramState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn capture_env_snapshot(&mut self) -> Arc<Env> {
        if let Some(snapshot) = &self.env_snapshot {
            return Arc::clone(snapshot);
        }

        let snapshot = Arc::new(self.visible_env_snapshot());
        self.env_snapshot = Some(Arc::clone(&snapshot));
        snapshot
    }

    pub fn set_env_fallback(&mut self, env: Arc<Env>) {
        self.env_snapshot = None;
        self.fallback_env = Some(env);
    }

    pub fn set_fn_fallback(&mut self, fns: FnEnv) {
        self.fallback_fns = Some(fns);
    }

    pub fn lookup_env(&self, name: &str) -> Option<&ZunValue> {
        self.env
            .get(name)
            .or_else(|| self.fallback_env.as_ref().and_then(|env| env.get(name)))
    }

    pub fn env_contains_key(&self, name: &str) -> bool {
        self.lookup_env(name).is_some()
    }

    pub fn get_env_mut(&mut self, name: &str) -> Option<&mut ZunValue> {
        if !self.env.contains_key(name) {
            let fallback_value = self
                .fallback_env
                .as_ref()
                .and_then(|env| env.get(name))
                .cloned()?;
            self.env_snapshot = None;
            self.env.insert(name.to_string(), fallback_value);
        }
        self.env.get_mut(name)
    }

    pub fn lookup_fn(&self, name: &str) -> Option<crate::interpreter::env::RuntimeFn> {
        self.fns.get(name).cloned().or_else(|| {
            self.fallback_fns
                .as_ref()
                .and_then(|fns| fns.get(name))
                .cloned()
        })
    }

    pub fn visible_env_snapshot(&self) -> Env {
        let mut snapshot = self
            .fallback_env
            .as_ref()
            .map(|env| env.as_ref().clone())
            .unwrap_or_default();
        snapshot.extend(
            self.env
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
        snapshot
    }

    pub fn insert_env(&mut self, name: String, value: ZunValue) -> Option<ZunValue> {
        self.env_snapshot = None;
        self.env.insert(name, value)
    }

    pub fn invalidate_env_snapshot(&mut self) {
        self.env_snapshot = None;
    }

    pub fn extend_env<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, ZunValue)>,
    {
        self.env_snapshot = None;
        self.env.extend(iter);
    }

    pub fn clear_env(&mut self) {
        self.env_snapshot = None;
        self.env.clear();
        self.fallback_env = None;
    }
}

pub fn execute_program_with_writer(
    statements: &[Stmt],
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    writer: &mut dyn Write,
) -> Result<bool, Error> {
    if let Some(global_cors) = statements.iter().find_map(|stmt| match stmt {
        Stmt::MainDecl(main) => Some(main.global_cors.clone()),
        _ => None,
    }) {
        context.set_global_cors(global_cors);
    }

    for stmt in statements {
        let (should_continue, result) = exec_with_writer(stmt, state, context, writer);
        result?;
        if !should_continue {
            return Ok(false);
        }
    }

    Ok(true)
}

pub fn execute_source_with_writer(
    source: &str,
    state: &mut ProgramState,
    context: &mut RuntimeContext,
    writer: &mut dyn Write,
) -> Result<bool, Error> {
    let statements = parser::parse(source)?;
    execute_program_with_writer(&statements, state, context, writer)
}

#[allow(dead_code)]
pub fn execute_program(
    statements: &[Stmt],
    state: &mut ProgramState,
    context: &mut RuntimeContext,
) -> Result<bool, Error> {
    execute_program_with_writer(statements, state, context, &mut io::stdout())
}
