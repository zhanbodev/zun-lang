// String 内置方法实现
use crate::error::Error;
use crate::interpreter::ZunValue;

/// String 方法分发
pub fn call(receiver: &ZunValue, method: &str, args: &[ZunValue]) -> Result<ZunValue, Error> {
    let s = match receiver {
        ZunValue::Str(s) => s,
        _ => return Err(Error::Interpreter("expected String".to_string())),
    };

    match method {
        "len" | "length" => Ok(ZunValue::Int(s.len() as i64)),
        "upper" => Ok(ZunValue::Str(s.to_uppercase())),
        "lower" => Ok(ZunValue::Str(s.to_lowercase())),
        "trim" => Ok(ZunValue::Str(s.trim().to_string())),
        "contains" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'contains' requires 1 argument".to_string(),
                ));
            }
            let arg = match &args[0] {
                ZunValue::Str(a) => a.as_str(),
                _ => {
                    return Err(Error::Interpreter(
                        "contains requires a String argument".to_string(),
                    ));
                }
            };
            Ok(ZunValue::Bool(s.contains(arg)))
        }
        "starts_with" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'starts_with' requires 1 argument".to_string(),
                ));
            }
            let arg = match &args[0] {
                ZunValue::Str(a) => a.as_str(),
                _ => {
                    return Err(Error::Interpreter(
                        "starts_with requires a String argument".to_string(),
                    ));
                }
            };
            Ok(ZunValue::Bool(s.starts_with(arg)))
        }
        "ends_with" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'ends_with' requires 1 argument".to_string(),
                ));
            }
            let arg = match &args[0] {
                ZunValue::Str(a) => a.as_str(),
                _ => {
                    return Err(Error::Interpreter(
                        "ends_with requires a String argument".to_string(),
                    ));
                }
            };
            Ok(ZunValue::Bool(s.ends_with(arg)))
        }
        "replace" => {
            if args.len() != 2 {
                return Err(Error::Interpreter(
                    "method 'replace' requires 2 arguments".to_string(),
                ));
            }
            let old = match &args[0] {
                ZunValue::Str(a) => a.as_str(),
                _ => {
                    return Err(Error::Interpreter(
                        "replace requires String arguments".to_string(),
                    ));
                }
            };
            let new = match &args[1] {
                ZunValue::Str(a) => a.as_str(),
                _ => {
                    return Err(Error::Interpreter(
                        "replace requires String arguments".to_string(),
                    ));
                }
            };
            Ok(ZunValue::Str(s.replace(old, new)))
        }
        "split" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'split' requires 1 argument".to_string(),
                ));
            }
            let sep = match &args[0] {
                ZunValue::Str(a) => a.as_str(),
                _ => {
                    return Err(Error::Interpreter(
                        "split requires a String argument".to_string(),
                    ));
                }
            };
            let parts: Vec<ZunValue> = s.split(sep).map(|p| ZunValue::Str(p.to_string())).collect();
            Ok(ZunValue::List(parts))
        }
        "slice" => {
            if args.len() != 2 {
                return Err(Error::Interpreter(
                    "method 'slice' requires 2 arguments".to_string(),
                ));
            }
            let start = match &args[0] {
                ZunValue::Int(n) => *n as usize,
                _ => {
                    return Err(Error::Interpreter(
                        "slice requires Integer arguments".to_string(),
                    ));
                }
            };
            let end = match &args[1] {
                ZunValue::Int(n) => *n as usize,
                _ => {
                    return Err(Error::Interpreter(
                        "slice requires Integer arguments".to_string(),
                    ));
                }
            };
            let chars: Vec<char> = s.chars().collect();
            if start < chars.len() && end <= chars.len() && start < end {
                let slice: String = chars[start..end].iter().collect();
                Ok(ZunValue::Str(slice))
            } else {
                Err(Error::Interpreter("invalid slice indices".to_string()))
            }
        }
        "to_int" => {
            if let Ok(i) = s.parse::<i64>() {
                Ok(ZunValue::Int(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(ZunValue::Int(f as i64))
            } else {
                Err(Error::Interpreter(format!(
                    "cannot convert \"{}\" to Int",
                    s
                )))
            }
        }
        "to_float" => {
            if let Ok(f) = s.parse::<f64>() {
                Ok(ZunValue::Float(f))
            } else {
                Err(Error::Interpreter(format!(
                    "cannot convert \"{}\" to Float",
                    s
                )))
            }
        }
        "to_bool" => {
            if s == "true" {
                Ok(ZunValue::Bool(true))
            } else if s == "false" {
                Ok(ZunValue::Bool(false))
            } else {
                Err(Error::Interpreter(format!(
                    "cannot convert \"{}\" to Bool\nexpected \"true\" or \"false\"",
                    s
                )))
            }
        }
        "to_str" => Ok(receiver.clone()),
        "type" => Ok(ZunValue::Str("String".to_string())),
        _ => Err(Error::Interpreter(format!(
            "string has no method '{}'",
            method
        ))),
    }
}
