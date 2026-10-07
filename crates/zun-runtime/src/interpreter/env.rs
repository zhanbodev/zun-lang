// Environment management - types for variables and functions.
use super::value::ZunValue;
use crate::ast::{FnDeclStmt, TypeExpr};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Represents the type of a value (for type checking)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Dynamic, // No type annotation, can change freely
    Int,
    Float,
    String,
    Bool,
    List,
    Map,
    File,
    Json,
    Response,
}

/// Get the ValueType from a ZunValue
pub fn get_value_type(val: &ZunValue) -> ValueType {
    match val {
        ZunValue::Int(_) => ValueType::Int,
        ZunValue::Float(_) => ValueType::Float,
        ZunValue::Str(_) => ValueType::String,
        ZunValue::Bool(_) => ValueType::Bool,
        ZunValue::List(_) => ValueType::List,
        ZunValue::Map(_) => ValueType::Map,
        ZunValue::Function { .. } => ValueType::Dynamic,
        ZunValue::File { .. } => ValueType::File,
        ZunValue::Json(_) => ValueType::Json,
        ZunValue::Html(_) => ValueType::Dynamic,
        ZunValue::Response { .. } => ValueType::Response,
        ZunValue::ModuleProxy { .. } => ValueType::Dynamic,
        ZunValue::TypedInstance { .. } => ValueType::Dynamic,
        ZunValue::Connection { .. } => ValueType::Dynamic,
        ZunValue::Null => ValueType::Dynamic,
    }
}

/// Variable environment: name -> ZunValue
pub type Env = HashMap<String, ZunValue>;

/// Type environment: tracks declared types for variables with type annotations
/// This is used to enforce type checking on assignment
/// Key: variable name, Value: declared type (if any)
pub type TypeEnv = HashMap<String, TypeExpr>;

/// Constant environment: tracks which variables are constants
/// Key: variable name, Value: true if constant
pub type ConstEnv = HashMap<String, bool>;

#[derive(Debug, Clone)]
pub struct RuntimeFn {
    pub decl: FnDeclStmt,
    pub source_file: Option<String>,
    pub module_env: Arc<Env>,
}

/// Function environment with copy-on-write semantics so nested execution
/// can cheaply share function tables until a declaration mutates them.
#[derive(Debug, Clone, Default)]
pub struct FnEnv(Arc<HashMap<String, RuntimeFn>>);

impl FnEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: String, runtime_fn: RuntimeFn) -> Option<RuntimeFn> {
        Arc::make_mut(&mut self.0).insert(name, runtime_fn)
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, RuntimeFn)>,
    {
        Arc::make_mut(&mut self.0).extend(iter);
    }

    pub fn clear(&mut self) {
        Arc::make_mut(&mut self.0).clear();
    }
}

impl Deref for FnEnv {
    type Target = HashMap<String, RuntimeFn>;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<'a> IntoIterator for &'a FnEnv {
    type Item = (&'a String, &'a RuntimeFn);
    type IntoIter = std::collections::hash_map::Iter<'a, String, RuntimeFn>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for FnEnv {
    type Item = (String, RuntimeFn);
    type IntoIter = std::collections::hash_map::IntoIter<String, RuntimeFn>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.0).into_iter()
    }
}

/// Counter for generating unique anonymous function names
static FN_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generate a unique name for anonymous function
pub fn generate_fn_name() -> String {
    let n = FN_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("__anon_fn_{}__", n)
}

/// Format a parsed type expression for diagnostics.
pub fn type_expr_name(type_expr: &TypeExpr) -> String {
    match type_expr {
        TypeExpr::Named(name) => name.clone(),
        TypeExpr::List(inner) => format!("List<{}>", type_expr_name(inner)),
        TypeExpr::Optional(inner) => format!("{}?", type_expr_name(inner)),
    }
}

/// Get the type name for display
pub fn type_name(vt: &ValueType) -> &'static str {
    match vt {
        ValueType::Dynamic => "Dynamic",
        ValueType::Int => "Int",
        ValueType::Float => "Float",
        ValueType::String => "String",
        ValueType::Bool => "Bool",
        ValueType::List => "List",
        ValueType::Map => "Map",
        ValueType::File => "File",
        ValueType::Json => "Json",
        ValueType::Response => "Response",
    }
}
