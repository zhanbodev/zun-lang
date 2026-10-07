// JSON 内置方法实现

use crate::error::Error;
use crate::interpreter::ZunValue;

/// JSON 方法分发（不可变）
pub fn call(receiver: &ZunValue, method: &str, _args: &[ZunValue]) -> Result<ZunValue, Error> {
    match method {
        "to_str" => {
            // 将 Json 转换为字符串表示
            Ok(ZunValue::Str(receiver.to_string()))
        }
        "type" => Ok(ZunValue::Str("Json".to_string())),
        _ => Err(Error::Interpreter(format!(
            "Json has no method '{}'",
            method
        ))),
    }
}
