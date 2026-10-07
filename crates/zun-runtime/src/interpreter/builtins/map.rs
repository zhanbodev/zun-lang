// Map 内置方法实现
use crate::error::Error;
use crate::interpreter::ZunValue;

/// Map 方法分发 (不可变)
pub fn call(receiver: &ZunValue, method: &str, args: &[ZunValue]) -> Result<ZunValue, Error> {
    let map = match receiver {
        ZunValue::Map(map) => map,
        _ => return Err(Error::Interpreter("expected Map".to_string())),
    };

    match method {
        "len" | "length" => Ok(ZunValue::Int(map.len() as i64)),
        "keys" => {
            let keys: Vec<ZunValue> = map.keys().map(|k| ZunValue::Str(k.clone())).collect();
            Ok(ZunValue::List(keys))
        }
        "values" => {
            let values: Vec<ZunValue> = map.values().cloned().collect();
            Ok(ZunValue::List(values))
        }
        "contains_key" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'contains_key' requires 1 argument".to_string(),
                ));
            }
            let key = match &args[0] {
                ZunValue::Str(s) => s,
                _ => {
                    return Err(Error::Interpreter(
                        "contains_key requires a String argument".to_string(),
                    ));
                }
            };
            Ok(ZunValue::Bool(map.contains_key(key)))
        }
        "type" => Ok(ZunValue::Str("Map".to_string())),
        _ => Err(Error::Interpreter(format!(
            "map has no method '{}'",
            method
        ))),
    }
}

/// Map 方法分发 (可变)
pub fn call_mut(
    receiver: &mut ZunValue,
    method: &str,
    args: &[ZunValue],
) -> Result<ZunValue, Error> {
    let map = match receiver {
        ZunValue::Map(map) => map,
        _ => return Err(Error::Interpreter("expected Map".to_string())),
    };

    match method {
        "remove" => {
            if args.is_empty() {
                return Err(Error::Interpreter(
                    "method 'remove' requires 1 argument".to_string(),
                ));
            }
            let key = match &args[0] {
                ZunValue::Str(s) => s.clone(),
                _ => {
                    return Err(Error::Interpreter(
                        "remove requires a String argument".to_string(),
                    ));
                }
            };
            if map.swap_remove(&key).is_some() {
                Ok(receiver.clone())
            } else {
                Err(Error::Interpreter("key not found".to_string()))
            }
        }
        _ => call(receiver, method, args),
    }
}
