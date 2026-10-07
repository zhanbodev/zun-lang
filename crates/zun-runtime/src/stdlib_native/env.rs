use std::sync::Arc;

use indexmap::IndexMap;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::intrinsics::{ids, intrinsic_string_arg};
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "get".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| ctx.call_intrinsic(ids::ENV_GET, args)),
    );
    exports.insert(
        "get_or".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let key = intrinsic_string_arg("env.get_or", args, 0)?;
            let default = intrinsic_string_arg("env.get_or", args, 1)?;
            Ok(ZunValue::Str(
                std::env::var(key).unwrap_or_else(|_| default.to_string()),
            ))
        }),
    );
    exports.insert(
        "has".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let key = intrinsic_string_arg("env.has", args, 0)?;
            Ok(ZunValue::Bool(std::env::var(key).is_ok()))
        }),
    );
    exports.insert(
        "all".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            if !args.is_empty() {
                return Err(Error::Interpreter("env.all: takes no arguments".into()));
            }
            let map: IndexMap<String, ZunValue> = std::env::vars()
                .map(|(k, v)| (k, ZunValue::Str(v)))
                .collect();
            Ok(ZunValue::Map(map))
        }),
    );
    exports.insert(
        "set".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            if ctx.is_serve_mode() {
                return Err(Error::Interpreter(
                    "env.set: not allowed in serve mode".into(),
                ));
            }
            let key = intrinsic_string_arg("env.set", args, 0)?;
            let value = intrinsic_string_arg("env.set", args, 1)?;
            // SAFETY: single-threaded interpreter context; no concurrent env access
            unsafe { std::env::set_var(key, value) };
            Ok(ZunValue::Null)
        }),
    );
    exports.insert(
        "remove".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            if ctx.is_serve_mode() {
                return Err(Error::Interpreter(
                    "env.remove: not allowed in serve mode".into(),
                ));
            }
            let key = intrinsic_string_arg("env.remove", args, 0)?;
            // SAFETY: single-threaded interpreter context; no concurrent env access
            unsafe { std::env::remove_var(key) };
            Ok(ZunValue::Null)
        }),
    );
    context.register_native_module("std.env", exports);
}
