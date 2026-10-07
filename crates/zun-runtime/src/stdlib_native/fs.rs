use std::sync::Arc;

use crate::runtime::intrinsics::ids;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "read_text".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_READ_TEXT, args)
            },
        ),
    );
    exports.insert(
        "read_lines".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_READ_LINES, args)
            },
        ),
    );
    exports.insert(
        "write_text".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_WRITE_TEXT, args)
            },
        ),
    );
    exports.insert(
        "write".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_WRITE_TEXT, args)
            },
        ),
    );
    exports.insert(
        "append_text".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_APPEND_TEXT, args)
            },
        ),
    );
    exports.insert(
        "append".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_APPEND_TEXT, args)
            },
        ),
    );
    exports.insert(
        "delete".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_DELETE, args)
            },
        ),
    );
    exports.insert(
        "exists".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_EXISTS, args)
            },
        ),
    );
    exports.insert(
        "size".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_SIZE, args)
            },
        ),
    );
    exports.insert(
        "is_dir".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_IS_DIR, args)
            },
        ),
    );
    exports.insert(
        "list".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_LIST, args)
            },
        ),
    );
    exports.insert(
        "mkdir".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_MKDIR, args)
            },
        ),
    );
    exports.insert(
        "mkdir_all".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_MKDIR_ALL, args)
            },
        ),
    );
    exports.insert(
        "rmdir".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_RMDIR, args)
            },
        ),
    );
    exports.insert(
        "copy".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_COPY, args)
            },
        ),
    );
    exports.insert(
        "rename".into(),
        Arc::new(
            |args: &[crate::interpreter::ZunValue], ctx: &RuntimeContext| {
                ctx.call_intrinsic(ids::FS_RENAME, args)
            },
        ),
    );
    context.register_native_module("std.fs", exports);
}
