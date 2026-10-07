use std::sync::Arc;

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "v4".into(),
        Arc::new(|_args: &[ZunValue], _ctx: &RuntimeContext| {
            Ok(ZunValue::Str(uuid::Uuid::new_v4().to_string()))
        }),
    );
    exports.insert(
        "is_valid".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let input = match args.first() {
                Some(ZunValue::Str(s)) => s,
                Some(other) => {
                    return Err(Error::Interpreter(format!(
                        "uuid.is_valid: first arg must be String, got {}",
                        other.type_name()
                    )));
                }
                None => {
                    return Err(Error::Interpreter(
                        "uuid.is_valid: missing first arg".into(),
                    ));
                }
            };
            Ok(ZunValue::Bool(uuid::Uuid::parse_str(input).is_ok()))
        }),
    );

    context.register_native_module("std.uuid", exports);
}

#[cfg(test)]
mod tests {
    #[test]
    fn recognizes_valid_uuid_strings() {
        assert!(uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").is_ok());
    }
}
