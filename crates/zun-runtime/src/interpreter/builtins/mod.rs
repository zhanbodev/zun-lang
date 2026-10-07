// 内置方法分发入口
// 负责将方法调用路由到对应的类型实现

pub mod bool_methods;
pub mod file;
pub mod html;
pub mod json;
pub mod list;
pub mod map;
pub mod number;
pub mod sql_connection;
pub mod str_methods;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::RuntimeContext;

/// 判断方法是否需要可变访问
fn is_mutating(method: &str) -> bool {
    matches!(
        method,
        "push"
            | "pop"
            | "reverse"
            | "remove"
            | "sort"
            | "sort_desc"
            | "remove_at"
            | "insert"
            | "clear"
    )
}

/// 不可变方法分发
pub fn dispatch(
    receiver: &ZunValue,
    method: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    match receiver {
        ZunValue::List(_) => list::call(receiver, method, args),
        ZunValue::Map(_) => map::call(receiver, method, args),
        ZunValue::Str(_) => str_methods::call(receiver, method, args),
        ZunValue::Int(_) | ZunValue::Float(_) => number::call(receiver, method, args),
        ZunValue::Bool(_) => bool_methods::call(receiver, method, args),
        ZunValue::File { .. } => file::call(receiver, method, args, context),
        ZunValue::Json(_) => json::call(receiver, method, args),
        ZunValue::Html(_) => html::call(receiver, method, args, context),
        ZunValue::Response { .. } => Err(Error::Interpreter(format!(
            "Response has no method '{}'",
            method
        ))),
        ZunValue::Function { .. } => Err(Error::Interpreter(format!(
            "Function has no method '{}'",
            method
        ))),
        ZunValue::ModuleProxy { .. } => Err(Error::Interpreter(format!(
            "ModuleProxy has no method '{}', use module.function() to call module functions",
            method
        ))),
        ZunValue::TypedInstance { type_name, .. } => Err(Error::Interpreter(format!(
            "type '{}' exposes fields only; '{}' is not a built-in method",
            type_name, method
        ))),
        ZunValue::Connection { .. } => sql_connection::call(receiver, method, args, context),
        ZunValue::Null => Err(Error::Interpreter(format!(
            "cannot call method '{}' on null",
            method
        ))),
    }
}

/// 可变方法分发（仅 List 和 Map 需要）
pub fn dispatch_mut(
    receiver: &mut ZunValue,
    method: &str,
    args: &[ZunValue],
) -> Result<ZunValue, Error> {
    match receiver {
        ZunValue::List(_) => list::call_mut(receiver, method, args),
        ZunValue::Map(_) => map::call_mut(receiver, method, args),
        ZunValue::File { .. } => Err(Error::Interpreter(format!(
            "'{}' does not support mutable method '{}'",
            receiver.type_name(),
            method
        ))),
        ZunValue::Json(_) => Err(Error::Interpreter(format!(
            "'{}' does not support mutable method '{}'",
            receiver.type_name(),
            method
        ))),
        ZunValue::Response { .. } => Err(Error::Interpreter(format!(
            "'{}' does not support mutable method '{}'",
            receiver.type_name(),
            method
        ))),
        _ => Err(Error::Interpreter(format!(
            "'{}' does not support mutable method '{}'",
            receiver.type_name(),
            method
        ))),
    }
}

/// 判断方法是否为可变方法（会修改原对象）
pub fn is_method_mutating(method: &str) -> bool {
    is_mutating(method)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::runtime::intrinsics::ids;
    use crate::runtime::{RuntimeContext, RuntimeMode};

    #[test]
    fn dispatches_string_and_list_methods() {
        let context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
        let upper = dispatch(&ZunValue::Str("zun".to_string()), "upper", &[], &context)
            .expect("upper should work");
        let len = dispatch(
            &ZunValue::List(vec![ZunValue::Int(1), ZunValue::Int(2)]),
            "len",
            &[],
            &context,
        )
        .expect("len should work");

        assert_eq!(upper, ZunValue::Str("ZUN".to_string()));
        assert_eq!(len, ZunValue::Int(2));
    }

    #[test]
    fn dispatches_connection_methods() {
        let context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
        let conn = context
            .call_intrinsic(
                ids::SQL_SQLITE_CONNECT,
                &[ZunValue::Str(":memory:".to_string())],
            )
            .expect("sqlite connection should open");

        dispatch(
            &conn,
            "execute",
            &[
                ZunValue::Str("CREATE TABLE users (id INTEGER, name TEXT)".to_string()),
                ZunValue::List(vec![]),
            ],
            &context,
        )
        .expect("table creation should work");

        dispatch(
            &conn,
            "execute",
            &[
                ZunValue::Str("INSERT INTO users (id, name) VALUES (?, ?)".to_string()),
                ZunValue::List(vec![ZunValue::Int(1), ZunValue::Str("Ada".to_string())]),
            ],
            &context,
        )
        .expect("insert should work");

        let rows = dispatch(
            &conn,
            "query",
            &[
                ZunValue::Str("SELECT id, name FROM users WHERE id = ?".to_string()),
                ZunValue::List(vec![ZunValue::Int(1)]),
            ],
            &context,
        )
        .expect("query should work");

        assert_eq!(format!("{rows}"), "[{id: 1, name: Ada}]");
        dispatch(&conn, "close", &[], &context).expect("close should work");
    }
}
