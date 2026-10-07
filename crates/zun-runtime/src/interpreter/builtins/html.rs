// HTML 内置方法
// $HTML().link() 方法实现

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::runtime::RuntimeContext;
use std::path::PathBuf;

/// 获取当前文件的基础目录
fn get_base_dir(context: &RuntimeContext) -> PathBuf {
    if let Some(file) = context.current_file()
        && let Some(parent) = PathBuf::from(&file).parent()
        && !parent.as_os_str().is_empty()
    {
        return parent.to_path_buf();
    }
    PathBuf::from(".")
}

/// 调用 HTML 类型的内置方法
pub fn call(
    receiver: &ZunValue,
    method: &str,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    match method {
        "link" => html_link(receiver, args, context),
        "to_str" => {
            // 将 HTML 转换为字符串
            let content = match receiver {
                ZunValue::Html(inner) => match *inner.clone() {
                    ZunValue::Str(s) => s,
                    _ => "".to_string(),
                },
                _ => "".to_string(),
            };
            Ok(ZunValue::Str(content))
        }
        "type" => Ok(ZunValue::Str("Html".to_string())),
        _ => Err(Error::Interpreter(format!(
            "Html has no method '{}'",
            method
        ))),
    }
}

/// $HTML().link("module.path") - 链接外部 HTML/CSS/JS/XML 文件
fn html_link(
    receiver: &ZunValue,
    args: &[ZunValue],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    // 获取 link 参数
    let module_path = match args.first() {
        Some(ZunValue::Str(s)) => s.clone(),
        Some(v) => {
            return Err(Error::Interpreter(format!(
                "$HTML().link() requires string argument, got {}",
                v.type_name()
            )));
        }
        None => {
            return Err(Error::Interpreter(
                "$HTML().link() requires a module path argument".to_string(),
            ));
        }
    };

    // 检查是否已经有非空内联内容
    let existing_content = match receiver {
        ZunValue::Html(inner) => match **inner {
            ZunValue::Str(ref s) if !s.is_empty() => Some(s.clone()),
            _ => None,
        },
        _ => None,
    };

    // 如果已有内联内容，直接返回（支持链式调用）
    if let Some(content) = existing_content {
        return Ok(ZunValue::Html(Box::new(ZunValue::Str(content))));
    }

    // 验证 module_path 格式：必须是 "module.name" 格式，不能是路径（除非是通配符）
    if !module_path.contains('*') && (module_path.contains('/') || module_path.contains('\\')) {
        return Err(Error::Interpreter(
            "$HTML().link() requires module path format (e.g., 'pages.index'), not file path"
                .to_string(),
        ));
    }

    // 支持的文件扩展名
    let valid_extensions = ["html", "htm", "css", "js", "xml"];

    // 处理通配符 "pages.*"
    if module_path.contains('*') {
        return handle_wildcard(&module_path, &valid_extensions, context);
    }

    // 普通模式：尝试加载单个文件
    let base_dir = get_base_dir(context);
    let module_parts: Vec<&str> = module_path.split('.').collect();
    let mut tried_paths = Vec::new();

    for ext in valid_extensions {
        // 构建可能的文件路径
        let mut path_parts = module_parts.clone();
        let file_name = format!("{}.{}", path_parts.last().unwrap_or(&""), ext);
        path_parts.pop();
        path_parts.push(&file_name);

        let relative_path = path_parts.join("/");
        let possible_path = base_dir.join(&relative_path);

        if possible_path.exists()
            && let Ok(content) = std::fs::read_to_string(&possible_path)
        {
            return Ok(ZunValue::Html(Box::new(ZunValue::Str(content))));
        }

        tried_paths.push(possible_path.display().to_string());
    }

    // 文件未找到
    Err(Error::Interpreter(format!(
        "$HTML().link(\"{}\") file not found. Tried: {}",
        module_path,
        tried_paths.join(", ")
    )))
}

/// 处理通配符匹配 "pages.*"
fn handle_wildcard(
    module_path: &str,
    valid_extensions: &[&str],
    context: &RuntimeContext,
) -> Result<ZunValue, Error> {
    // 解析通配符路径
    let parts: Vec<&str> = module_path.split('.').collect();
    let mut dir_path = Vec::new();

    for part in parts.iter() {
        if part.contains('*') {
            break;
        }
        dir_path.push(*part);
    }

    let base_dir = get_base_dir(context);
    let dir = if dir_path.is_empty() {
        base_dir.clone()
    } else {
        base_dir.join(dir_path.join("/"))
    };

    if !dir.exists() || !dir.is_dir() {
        return Err(Error::Interpreter(format!(
            "$HTML().link(\"{}\") directory not found: {}",
            module_path,
            dir.display()
        )));
    }

    // 收集所有匹配的文件
    let mut matches: Vec<(String, String)> = Vec::new();

    collect_matching_files(&dir, "", valid_extensions, &mut matches);

    if matches.is_empty() {
        return Err(Error::Interpreter(format!(
            "$HTML().link(\"{}\") no matching files found in {}",
            module_path,
            dir.display()
        )));
    }

    // 按文件名排序
    matches.sort_by(|a, b| a.0.cmp(&b.0));

    // 合并所有文件内容（用换行分隔）
    let mut combined_content = String::new();
    for (filename, content) in &matches {
        if !combined_content.is_empty() {
            combined_content.push_str("\n<!-- ===== ");
            combined_content.push_str(filename);
            combined_content.push_str(" ===== -->\n");
        }
        combined_content.push_str(content);
    }

    Ok(ZunValue::Html(Box::new(ZunValue::Str(combined_content))))
}

/// 递归收集匹配的文件
fn collect_matching_files(
    dir: &PathBuf,
    prefix: &str,
    valid_extensions: &[&str],
    matches: &mut Vec<(String, String)>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                // 递归处理子目录
                let new_prefix = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{}.{}", prefix, name)
                };
                collect_matching_files(&path, &new_prefix, valid_extensions, matches);
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if valid_extensions.contains(&ext_str.as_str()) {
                    let file_key = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{}.{}", prefix, name)
                    };
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        matches.push((file_key, content));
                    }
                }
            }
        }
    }
}
