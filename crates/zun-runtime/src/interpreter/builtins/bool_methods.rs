// Bool 内置方法实现
use crate::error::Error;
use crate::interpreter::ZunValue;

/// Bool 方法分发
pub fn call(receiver: &ZunValue, method: &str, _args: &[ZunValue]) -> Result<ZunValue, Error> {
    let b = match receiver {
        ZunValue::Bool(b) => *b,
        _ => return Err(Error::Interpreter("expected Bool".to_string())),
    };

    match method {
        "to_str" => Ok(ZunValue::Str(b.to_string())),
        "to_int" => Err(Error::Interpreter(
            "type mismatch: Bool cannot convert to Int".to_string(),
        )),
        "to_float" => Err(Error::Interpreter(
            "type mismatch: Bool cannot convert to Float".to_string(),
        )),
        "to_bool" => Ok(ZunValue::Bool(b)),
        "type" => Ok(ZunValue::Str("Bool".to_string())),
        _ => Err(Error::Interpreter(format!(
            "Bool has no method '{}'",
            method
        ))),
    }
}
