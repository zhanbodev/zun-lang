// File 内置方法实现
use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::{RuntimeContext, intrinsics::ids};

/// File 方法分发（不可变）
pub fn call(
    receiver: &ZunValue,
    method: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    let (path, mode, filename) = match receiver {
        ZunValue::File {
            path,
            mode,
            filename,
        } => (path, mode.as_deref(), filename.as_deref()),
        _ => return Err(Error::Interpreter("expected File".to_string())),
    };

    let path_arg = [ZunValue::Str(path.clone())];

    match method {
        "exists" => context.call_intrinsic(ids::FS_EXISTS, &path_arg),
        "size" => context.call_intrinsic(ids::FS_SIZE, &path_arg),
        "is_dir" => context.call_intrinsic(ids::FS_IS_DIR, &path_arg),
        "read" => {
            if mode == Some("LINES") {
                context.call_intrinsic(ids::FS_READ_LINES, &path_arg)
            } else {
                context.call_intrinsic(ids::FS_READ_TEXT, &path_arg)
            }
        }
        "read_lines" => context.call_intrinsic(ids::FS_READ_LINES, &path_arg),
        "type" => Ok(ZunValue::Str("File".to_string())),
        "path" => Ok(ZunValue::Str(path.to_string())),
        "filename" => Ok(match filename {
            Some(f) => ZunValue::Str(f.to_string()),
            None => std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| ZunValue::Str(s.to_string()))
                .unwrap_or(ZunValue::Null),
        }),
        "content" => {
            let content_str = match args.first() {
                Some(ZunValue::Str(s)) => s.clone(),
                Some(v) => {
                    return Err(Error::Interpreter(format!(
                        "content() requires String argument, got {}",
                        v.type_name()
                    )));
                }
                None => {
                    return Err(Error::Interpreter(
                        "content() requires String argument".to_string(),
                    ));
                }
            };

            match mode {
                Some("A") | Some("a") | Some("append") => context.call_intrinsic(
                    ids::FS_APPEND_TEXT,
                    &[ZunValue::Str(path.clone()), ZunValue::Str(content_str)],
                ),
                _ => context.call_intrinsic(
                    ids::FS_WRITE_TEXT,
                    &[ZunValue::Str(path.clone()), ZunValue::Str(content_str)],
                ),
            }
        }
        _ => Err(Error::Interpreter(format!(
            "file has no method '{}'",
            method
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeContext, RuntimeMode};
    use std::path::PathBuf;

    fn test_context() -> RuntimeContext {
        RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."))
    }

    fn local_file(path: &str) -> ZunValue {
        ZunValue::File {
            path: path.to_string(),
            mode: None,
            filename: None,
        }
    }

    fn uploaded_file(path: &str, filename: &str) -> ZunValue {
        ZunValue::File {
            path: path.to_string(),
            mode: None,
            filename: Some(filename.to_string()),
        }
    }

    #[test]
    fn path_returns_file_path() {
        let file = local_file("./storage/avatar.png");
        let ctx = test_context();
        let result = call(&file, "path", &[], &ctx).unwrap();
        assert_eq!(result, ZunValue::Str("./storage/avatar.png".to_string()));
    }

    #[test]
    fn filename_returns_basename_for_local_file() {
        let file = local_file("/var/data/report.pdf");
        let ctx = test_context();
        let result = call(&file, "filename", &[], &ctx).unwrap();
        assert_eq!(result, ZunValue::Str("report.pdf".to_string()));
    }

    #[test]
    fn filename_returns_upload_original_name() {
        let file = uploaded_file("/tmp/zun-uploads/123-profile.jpg", "profile.jpg");
        let ctx = test_context();
        let result = call(&file, "filename", &[], &ctx).unwrap();
        assert_eq!(result, ZunValue::Str("profile.jpg".to_string()));
    }

    #[test]
    fn type_still_returns_file() {
        let file = local_file("./any.txt");
        let ctx = test_context();
        let result = call(&file, "type", &[], &ctx).unwrap();
        assert_eq!(result, ZunValue::Str("File".to_string()));
    }
}
