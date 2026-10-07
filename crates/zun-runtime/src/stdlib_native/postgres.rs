use std::sync::Arc;

use crate::interpreter::ZunValue;
use crate::runtime::intrinsics::ids;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "connect".into(),
        Arc::new(|args: &[ZunValue], ctx: &RuntimeContext| {
            ctx.call_intrinsic(ids::SQL_POSTGRES_CONNECT, args)
        }),
    );

    context.register_native_module("std.postgres", exports);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    use super::register;
    use crate::interpreter::ZunValue;
    use crate::runtime::intrinsics::{IntrinsicRegistry, ids};
    use crate::runtime::{RuntimeContext, RuntimeMode};

    static CONNECT_ARGS: OnceLock<Mutex<Vec<ZunValue>>> = OnceLock::new();

    fn connect_args() -> &'static Mutex<Vec<ZunValue>> {
        CONNECT_ARGS.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    fn postgres_connect_stub(
        args: &[ZunValue],
        _context: &RuntimeContext,
    ) -> Result<ZunValue, crate::error::Error> {
        *connect_args().lock().expect("connect args lock") = args.to_vec();
        Ok(ZunValue::Str("postgres-connect".to_string()))
    }

    #[test]
    fn postgres_module_registers_connect() {
        connect_args().lock().expect("connect args lock").clear();

        let mut context = test_context();
        let mut intrinsics = IntrinsicRegistry::new();
        intrinsics.register(ids::SQL_POSTGRES_CONNECT, postgres_connect_stub);
        context.set_intrinsic_registry(intrinsics);

        register(&mut context);

        let module = context
            .native_module("std.postgres")
            .expect("postgres module should be registered");
        let connect = module
            .get("connect")
            .expect("postgres module should export connect");
        let result = connect.as_ref()(
            &[ZunValue::Str("postgres://localhost".to_string())],
            &context,
        )
        .expect("connect export should succeed");

        assert_eq!(result, ZunValue::Str("postgres-connect".to_string()));
        assert_eq!(
            *connect_args().lock().expect("connect args lock"),
            vec![ZunValue::Str("postgres://localhost".to_string())]
        );
    }
}
