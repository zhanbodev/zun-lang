// Zun 运行时值的类型安全表示
//
// 设计决策：
// - Char 内部归入 Str（与 Zun 文档一致："字符类型在内部也以字符串形式存储"）
// - Null 表示无返回值函数的结果（$fn greet() { ... } 无 $# 语句时）
// - Function 表示匿名函数作为一等公民存储在变量中的情况

use crate::ast::{FnParam, Stmt, TypeExpr};
use indexmap::IndexMap;
use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

/// Zun 运行时值的类型安全表示
#[derive(Clone)]
pub enum ZunValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    List(Vec<ZunValue>),
    Map(IndexMap<String, ZunValue>),
    Function {
        params: Vec<FnParam>,
        variadic_param: Option<String>,
        body: Vec<Stmt>,
        return_type: Option<TypeExpr>,
    },
    File {
        path: String,
        mode: Option<String>,     // "LINES" for line reading
        filename: Option<String>, // 上传时来自 multipart Header，本地文件为 None
    },
    Json(IndexMap<String, ZunValue>), // JSON object
    Html(Box<ZunValue>),              // HTML content
    Response {
        status: u16,
        body: Option<Box<ZunValue>>, // response body
    },
    ModuleProxy {
        path: String,
        state: Arc<super::ModuleNamespace>,
    },
    TypedInstance {
        type_name: String,
        fields: IndexMap<String, ZunValue>,
    },
    Connection {
        id: String,
        driver: String, // "sqlite" or "postgres"
    },
    Null,
}

impl PartialEq for ZunValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => {
                // 处理 NaN：Zun 语义中 NaN == NaN → false
                a == b
            }
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Null, Self::Null) => true,
            // List / Map 递归比较
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            // Function 不可比较
            (Self::Function { .. }, _) => false,
            // File 不可比较
            (Self::File { .. }, _) => false,
            // Json 不可比较
            (Self::Json(..), _) => false,
            // Html 不可比较
            (Self::Html(..), _) => false,
            // Response 不可比较
            (Self::Response { .. }, _) => false,
            // ModuleProxy 不可比较
            (Self::ModuleProxy { .. }, _) => false,
            // TypedInstance 暂不支持相等比较
            (Self::TypedInstance { .. }, _) => false,
            (Self::Connection { id: a, .. }, Self::Connection { id: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for ZunValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => write!(f, "{}", n),
            Self::Float(n) => {
                // Format float but always keep decimal point
                if n.fract().abs() < 1e-10 {
                    write!(f, "{:.1}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Self::Str(s) => write!(f, "{}", s),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Null => write!(f, "null"),
            Self::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Self::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Self::Function { params, .. } => {
                let names = params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "<fn({names})>")
            }
            Self::File { path, .. } => {
                write!(f, "File({})", path)
            }
            Self::Json(map) => {
                // Serialize as JSON string

                let mut s = String::from("{");
                let mut first = true;
                for (k, v) in map.iter() {
                    if !first {
                        s.push_str(", ");
                    }
                    first = false;
                    s.push_str(&format!("\"{}\": {}", k, value_to_json(v)));
                }
                s.push('}');
                write!(f, "{}", s)
            }
            Self::Html(val) => {
                // Display HTML content
                write!(f, "{}", val)
            }
            Self::Response { status, body } => {
                if let Some(b) = body {
                    write!(f, "Response({}, {})", status, b)
                } else {
                    write!(f, "Response({})", status)
                }
            }
            Self::ModuleProxy { path, .. } => {
                write!(f, "ModuleProxy({})", path)
            }
            Self::Connection { id, driver } => {
                write!(f, "Connection({}:{})", driver, id)
            }
            Self::TypedInstance { type_name, fields } => {
                write!(f, "{} {{", type_name)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl ZunValue {
    /// 运行时类型名，用于 .type() 方法返回值
    pub fn type_name(&self) -> Cow<'_, str> {
        match self {
            Self::Int(_) => Cow::Borrowed("Int"),
            Self::Float(_) => Cow::Borrowed("Float"),
            Self::Str(_) => Cow::Borrowed("String"),
            Self::Bool(_) => Cow::Borrowed("Bool"),
            Self::List(items) => {
                if let Some(first) = items.first() {
                    let first_type = first.type_name();
                    if items.iter().all(|item| item.type_name() == first_type) {
                        return Cow::Owned(format!("List<{}>", first_type));
                    }
                }
                Cow::Borrowed("List")
            }
            Self::Map(_) => Cow::Borrowed("Map"),
            Self::Function { .. } => Cow::Borrowed("Function"),
            Self::File { .. } => Cow::Borrowed("File"),
            Self::Json(_) => Cow::Borrowed("Json"),
            Self::Html(_) => Cow::Borrowed("Html"),
            Self::Response { .. } => Cow::Borrowed("Response"),
            Self::ModuleProxy { .. } => Cow::Borrowed("ModuleProxy"),
            Self::TypedInstance { type_name, .. } => Cow::Borrowed(type_name.as_str()),
            Self::Connection { .. } => Cow::Borrowed("Connection"),
            Self::Null => Cow::Borrowed("Null"),
        }
    }

    /// 判断值是否为真（用于布尔上下文）
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Int(n) => *n != 0,
            Self::Float(f) => *f != 0.0 && !f.is_nan(),
            Self::Str(s) => !s.is_empty(),
            Self::List(l) => !l.is_empty(),
            Self::Map(m) => !m.is_empty(),
            Self::Function { .. } => true,
            Self::File { .. } => true,
            Self::Json(m) => !m.is_empty(),
            Self::Html(v) => v.is_truthy(),
            Self::Response { .. } => true,
            Self::ModuleProxy { .. } => true,
            Self::TypedInstance { .. } => true,
            Self::Connection { .. } => true,
            Self::Null => false,
        }
    }
}

/// Convert ZunValue to JSON-compatible serde_json::Value
pub fn value_to_json(value: &ZunValue) -> serde_json::Value {
    use serde_json::json;
    match value {
        ZunValue::Int(n) => json!(*n),
        ZunValue::Float(f) => json!(*f),
        ZunValue::Str(s) => json!(s),
        ZunValue::Bool(b) => json!(*b),
        ZunValue::List(arr) => {
            json!(arr.iter().map(value_to_json).collect::<Vec<_>>())
        }
        ZunValue::Map(m) => {
            let obj = m
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect::<serde_json::Map<String, serde_json::Value>>();
            serde_json::Value::Object(obj)
        }
        ZunValue::Json(m) => {
            let obj = m
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect::<serde_json::Map<String, serde_json::Value>>();
            serde_json::Value::Object(obj)
        }
        ZunValue::TypedInstance { fields, .. } => {
            let mut obj = serde_json::Map::new();
            for (k, v) in fields {
                if !k.starts_with('_') {
                    obj.insert(k.clone(), value_to_json(v));
                }
            }
            serde_json::Value::Object(obj)
        }
        ZunValue::Html(v) => value_to_json(v),
        ZunValue::Null => serde_json::Value::Null,
        _ => serde_json::Value::Null,
    }
}

impl fmt::Debug for ZunValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(n) => write!(f, "Int({n})"),
            Self::Float(n) => write!(f, "Float({n})"),
            Self::Str(s) => write!(f, "Str({s:?})"),
            Self::Bool(b) => write!(f, "Bool({b})"),
            Self::Null => write!(f, "Null"),
            Self::List(v) => write!(f, "List({v:?})"),
            Self::Map(m) => write!(f, "Map({m:?})"),
            Self::Function { params, .. } => {
                let names = params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "Function({names})")
            }
            Self::File {
                path,
                mode,
                filename,
            } => write!(f, "File({path:?}, {mode:?}, {filename:?})"),
            Self::Json(m) => write!(f, "Json({m:?})"),
            Self::Html(v) => write!(f, "Html({v:?})"),
            Self::Response { status, .. } => write!(f, "Response({status})"),
            Self::ModuleProxy { path, .. } => write!(f, "ModuleProxy({path:?})"),
            Self::TypedInstance { type_name, fields } => {
                write!(f, "TypedInstance({type_name:?}, {fields:?})")
            }
            Self::Connection { id, driver } => write!(f, "Connection({driver:?}, {id:?})"),
        }
    }
}

#[cfg(test)]
mod connection_tests {
    use super::*;

    #[test]
    fn connection_value_has_correct_type_name() {
        let conn = ZunValue::Connection {
            id: "conn:sqlite:0".to_string(),
            driver: "sqlite".to_string(),
        };
        assert_eq!(conn.type_name(), "Connection");
    }

    #[test]
    fn connection_value_is_truthy() {
        let conn = ZunValue::Connection {
            id: "conn:sqlite:0".to_string(),
            driver: "sqlite".to_string(),
        };
        assert!(conn.is_truthy());
    }

    #[test]
    fn connection_display() {
        let conn = ZunValue::Connection {
            id: "conn:sqlite:0".to_string(),
            driver: "sqlite".to_string(),
        };
        assert_eq!(format!("{}", conn), "Connection(sqlite:conn:sqlite:0)");
    }
}

#[cfg(test)]
mod tests {
    use super::{ZunValue, value_to_json};
    use indexmap::IndexMap;

    #[test]
    fn truthiness_matches_runtime_expectations() {
        assert!(!ZunValue::Int(0).is_truthy());
        assert!(ZunValue::Int(1).is_truthy());
        assert!(!ZunValue::Str(String::new()).is_truthy());
        assert!(ZunValue::List(vec![ZunValue::Int(1)]).is_truthy());
    }

    #[test]
    fn json_conversion_keeps_map_shape() {
        let mut map = IndexMap::new();
        map.insert("name".to_string(), ZunValue::Str("zun".to_string()));
        let json = value_to_json(&ZunValue::Json(map));
        assert_eq!(json["name"], "zun");
    }

    #[test]
    fn json_conversion_hides_typed_instance_private_fields() {
        let mut fields = IndexMap::new();
        fields.insert("name".to_string(), ZunValue::Str("alice".to_string()));
        fields.insert("_password".to_string(), ZunValue::Str("secret".to_string()));
        let json = value_to_json(&ZunValue::TypedInstance {
            type_name: "User".to_string(),
            fields,
        });
        assert_eq!(json["name"], "alice");
        assert!(json.get("_password").is_none());
    }
}
