use std::sync::Arc;

use crate::interpreter::ZunValue;
use crate::runtime::auth::{hash_password, verify_password};
use crate::runtime::intrinsics::intrinsic_string_arg;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "hash".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let password = intrinsic_string_arg("auth.password.hash", args, 0)?;
            Ok(ZunValue::Str(hash_password(password)?))
        }),
    );
    exports.insert(
        "verify".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let password = intrinsic_string_arg("auth.password.verify", args, 0)?;
            let hash = intrinsic_string_arg("auth.password.verify", args, 1)?;
            Ok(ZunValue::Bool(verify_password(password, hash)?))
        }),
    );
    context.register_native_module("std.auth.password", exports);
}
