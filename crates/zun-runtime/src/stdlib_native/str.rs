use std::sync::Arc;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::intrinsics::intrinsic_string_arg;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "trim".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(
                intrinsic_string_arg("str.trim", args, 0)?
                    .trim()
                    .to_string(),
            ))
        }),
    );
    exports.insert(
        "trim_start".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(
                intrinsic_string_arg("str.trim_start", args, 0)?
                    .trim_start()
                    .to_string(),
            ))
        }),
    );
    exports.insert(
        "trim_end".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(
                intrinsic_string_arg("str.trim_end", args, 0)?
                    .trim_end()
                    .to_string(),
            ))
        }),
    );
    exports.insert(
        "split".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.split", args, 0)?;
            let sep = intrinsic_string_arg("str.split", args, 1)?;
            Ok(ZunValue::List(
                s.split(sep)
                    .map(|part| ZunValue::Str(part.to_string()))
                    .collect(),
            ))
        }),
    );
    exports.insert(
        "join".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let list = match args.first() {
                Some(ZunValue::List(values)) => values,
                Some(other) => {
                    return Err(Error::Interpreter(format!(
                        "str.join: first arg must be List, got {}",
                        other.type_name()
                    )));
                }
                None => return Err(Error::Interpreter("str.join: missing first arg".into())),
            };
            let sep = intrinsic_string_arg("str.join", args, 1)?;
            let parts: Vec<String> = list.iter().map(ToString::to_string).collect();
            Ok(ZunValue::Str(parts.join(sep)))
        }),
    );
    exports.insert(
        "replace".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.replace", args, 0)?;
            let from = intrinsic_string_arg("str.replace", args, 1)?;
            let to = intrinsic_string_arg("str.replace", args, 2)?;
            Ok(ZunValue::Str(s.replace(from, to)))
        }),
    );
    exports.insert(
        "contains".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.contains", args, 0)?;
            let sub = intrinsic_string_arg("str.contains", args, 1)?;
            Ok(ZunValue::Bool(s.contains(sub)))
        }),
    );
    exports.insert(
        "starts_with".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.starts_with", args, 0)?;
            let prefix = intrinsic_string_arg("str.starts_with", args, 1)?;
            Ok(ZunValue::Bool(s.starts_with(prefix)))
        }),
    );
    exports.insert(
        "ends_with".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.ends_with", args, 0)?;
            let suffix = intrinsic_string_arg("str.ends_with", args, 1)?;
            Ok(ZunValue::Bool(s.ends_with(suffix)))
        }),
    );
    exports.insert(
        "to_upper".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(
                intrinsic_string_arg("str.to_upper", args, 0)?
                    .to_uppercase()
                    .to_string(),
            ))
        }),
    );
    exports.insert(
        "to_lower".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(
                intrinsic_string_arg("str.to_lower", args, 0)?
                    .to_lowercase()
                    .to_string(),
            ))
        }),
    );
    exports.insert(
        "len".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Int(
                intrinsic_string_arg("str.len", args, 0)?.chars().count() as i64,
            ))
        }),
    );
    exports.insert(
        "parse_int".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.parse_int", args, 0)?;
            s.trim()
                .parse::<i64>()
                .map(ZunValue::Int)
                .map_err(|err| Error::Interpreter(format!("str.parse_int: invalid integer: {err}")))
        }),
    );
    exports.insert(
        "parse_float".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let s = intrinsic_string_arg("str.parse_float", args, 0)?;
            s.trim()
                .parse::<f64>()
                .map(ZunValue::Float)
                .map_err(|err| Error::Interpreter(format!("str.parse_float: invalid float: {err}")))
        }),
    );
    context.register_native_module("std.str", exports);
}
