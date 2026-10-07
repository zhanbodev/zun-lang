// List 内置方法实现
use std::cmp::Ordering;

use crate::error::Error;
use crate::interpreter::ZunValue;

/// List 方法分发（不可变）
pub fn call(receiver: &ZunValue, method: &str, args: &[ZunValue]) -> Result<ZunValue, Error> {
    let items = match receiver {
        ZunValue::List(items) => items,
        _ => return Err(Error::Interpreter("expected List".to_string())),
    };

    match method {
        "len" | "length" => Ok(ZunValue::Int(items.len() as i64)),
        "contains" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'contains' requires 1 argument".to_string(),
                ));
            }
            let contains = items.iter().any(|item| item == &args[0]);
            Ok(ZunValue::Bool(contains))
        }
        "join" => {
            let sep = if args.is_empty() {
                String::new()
            } else {
                match &args[0] {
                    ZunValue::Str(s) => s.clone(),
                    _ => {
                        return Err(Error::Interpreter(
                            "join requires a String argument".to_string(),
                        ));
                    }
                }
            };
            let result: String = items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
                .join(&sep);
            Ok(ZunValue::Str(result))
        }
        "index_of" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'index_of' requires 1 argument".to_string(),
                ));
            }
            let idx = items
                .iter()
                .position(|item| item == &args[0])
                .map(|i| i as i64)
                .unwrap_or(-1);
            Ok(ZunValue::Int(idx))
        }
        "last_index_of" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'last_index_of' requires 1 argument".to_string(),
                ));
            }
            let idx = items
                .iter()
                .rposition(|item| item == &args[0])
                .map(|i| i as i64)
                .unwrap_or(-1);
            Ok(ZunValue::Int(idx))
        }
        "slice" => {
            if args.len() < 2 {
                return Err(Error::Interpreter(
                    "method 'slice' requires 2 arguments (start, end)".to_string(),
                ));
            }
            let len = items.len() as i64;
            let resolve = |n: i64| -> usize {
                let idx = if n < 0 { (len + n).max(0) } else { n.min(len) };
                idx as usize
            };
            let start = match &args[0] {
                ZunValue::Int(n) => resolve(*n),
                other => {
                    return Err(Error::Interpreter(format!(
                        "slice: start must be Int, got {}",
                        other.type_name()
                    )));
                }
            };
            let end = match &args[1] {
                ZunValue::Int(n) => resolve(*n),
                other => {
                    return Err(Error::Interpreter(format!(
                        "slice: end must be Int, got {}",
                        other.type_name()
                    )));
                }
            };
            let end = end.max(start);
            Ok(ZunValue::List(items[start..end].to_vec()))
        }
        "first" => Ok(items.first().cloned().unwrap_or(ZunValue::Null)),
        "last" => Ok(items.last().cloned().unwrap_or(ZunValue::Null)),
        "is_empty" => Ok(ZunValue::Bool(items.is_empty())),
        "flatten" => {
            let mut result = Vec::new();
            for item in items {
                match item {
                    ZunValue::List(inner) => result.extend(inner.iter().cloned()),
                    other => result.push(other.clone()),
                }
            }
            Ok(ZunValue::List(result))
        }
        "unique" => {
            let mut seen = std::collections::HashSet::new();
            let result = items
                .iter()
                .filter(|v| seen.insert(format!("{:?}", v)))
                .cloned()
                .collect();
            Ok(ZunValue::List(result))
        }
        "count" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'count' requires 1 argument".to_string(),
                ));
            }
            let n = items.iter().filter(|item| *item == &args[0]).count();
            Ok(ZunValue::Int(n as i64))
        }
        "type" => Ok(ZunValue::Str("List".to_string())),
        _ => Err(Error::Interpreter(format!(
            "list has no method '{}'",
            method
        ))),
    }
}

/// List 方法分发（可变）
pub fn call_mut(
    receiver: &mut ZunValue,
    method: &str,
    args: &[ZunValue],
) -> Result<ZunValue, Error> {
    let items = match receiver {
        ZunValue::List(items) => items,
        _ => return Err(Error::Interpreter("expected List".to_string())),
    };

    match method {
        "push" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'push' requires 1 argument".to_string(),
                ));
            }
            items.push(args[0].clone());
            Ok(receiver.clone())
        }
        "pop" => {
            if items.is_empty() {
                return Err(Error::Interpreter("cannot pop from empty list".to_string()));
            }
            items
                .pop()
                .ok_or_else(|| Error::Interpreter("pop failed".to_string()))
        }
        "reverse" => {
            items.reverse();
            Ok(receiver.clone())
        }
        "sort" => {
            try_sort(items, false)?;
            Ok(receiver.clone())
        }
        "sort_desc" => {
            try_sort(items, true)?;
            Ok(receiver.clone())
        }
        "remove_at" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'remove_at' requires 1 argument".to_string(),
                ));
            }
            let idx = match &args[0] {
                ZunValue::Int(n) => *n,
                other => {
                    return Err(Error::Interpreter(format!(
                        "remove_at: index must be Int, got {}",
                        other.type_name()
                    )));
                }
            };
            if idx < 0 || idx as usize >= items.len() {
                return Err(Error::Interpreter(format!(
                    "remove_at: index {} out of bounds (len={})",
                    idx,
                    items.len()
                )));
            }
            items.remove(idx as usize);
            Ok(receiver.clone())
        }
        "insert" => {
            if args.len() < 2 {
                return Err(Error::Interpreter(
                    "method 'insert' requires 2 arguments (index, value)".to_string(),
                ));
            }
            let idx = match &args[0] {
                ZunValue::Int(n) => *n,
                other => {
                    return Err(Error::Interpreter(format!(
                        "insert: index must be Int, got {}",
                        other.type_name()
                    )));
                }
            };
            if idx < 0 || idx as usize > items.len() {
                return Err(Error::Interpreter(format!(
                    "insert: index {} out of bounds (len={})",
                    idx,
                    items.len()
                )));
            }
            items.insert(idx as usize, args[1].clone());
            Ok(receiver.clone())
        }
        "clear" => {
            items.clear();
            Ok(receiver.clone())
        }
        _ => Err(Error::Interpreter(format!(
            "list has no mutable method '{}'",
            method
        ))),
    }
}

fn try_sort(items: &mut Vec<ZunValue>, desc: bool) -> Result<(), Error> {
    // 验证所有元素可比较（全 Int/Float 混合，或全 Str）
    let all_numeric = items
        .iter()
        .all(|v| matches!(v, ZunValue::Int(_) | ZunValue::Float(_)));
    let all_str = items.iter().all(|v| matches!(v, ZunValue::Str(_)));

    if !items.is_empty() && !all_numeric && !all_str {
        return Err(Error::Interpreter(
            "sort: list must contain only numbers or only strings".to_string(),
        ));
    }

    items.sort_by(compare_values);
    if desc {
        items.reverse();
    }
    Ok(())
}

fn compare_values(a: &ZunValue, b: &ZunValue) -> Ordering {
    match (a, b) {
        (ZunValue::Int(x), ZunValue::Int(y)) => x.cmp(y),
        (ZunValue::Float(x), ZunValue::Float(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (ZunValue::Int(x), ZunValue::Float(y)) => {
            (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal)
        }
        (ZunValue::Float(x), ZunValue::Int(y)) => {
            x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal)
        }
        (ZunValue::Str(x), ZunValue::Str(y)) => x.cmp(y),
        _ => Ordering::Equal,
    }
}
