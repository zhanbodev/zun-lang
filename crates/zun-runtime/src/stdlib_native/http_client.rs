use std::sync::Arc;
use std::time::Duration;

use indexmap::IndexMap;
use reqwest::Method;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::Error;
use crate::interpreter::ZunValue;
use crate::interpreter::value::value_to_json;
use crate::runtime::{NativeFnMap, RuntimeContext};

pub fn register(context: &mut RuntimeContext) {
    let mut exports = NativeFnMap::new();
    exports.insert(
        "get".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let url = string_arg("http.get", args, 0)?;
            let headers = optional_headers_arg("http.get", args, 1)?;
            execute_request(Method::GET, url, None, headers)
        }),
    );
    exports.insert(
        "post".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let url = string_arg("http.post", args, 0)?;
            let body = json_body_arg("http.post", args, 1)?;
            let headers = optional_headers_arg("http.post", args, 2)?;
            execute_request(Method::POST, url, Some(body), headers)
        }),
    );
    exports.insert(
        "put".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let url = string_arg("http.put", args, 0)?;
            let body = json_body_arg("http.put", args, 1)?;
            let headers = optional_headers_arg("http.put", args, 2)?;
            execute_request(Method::PUT, url, Some(body), headers)
        }),
    );
    exports.insert(
        "delete".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let url = string_arg("http.delete", args, 0)?;
            let headers = optional_headers_arg("http.delete", args, 1)?;
            execute_request(Method::DELETE, url, None, headers)
        }),
    );
    exports.insert(
        "request".into(),
        Arc::new(|args: &[ZunValue], _ctx: &RuntimeContext| {
            let method = string_arg("http.request", args, 0)?;
            let url = string_arg("http.request", args, 1)?;
            let body = json_body_arg("http.request", args, 2)?;
            let headers = headers_arg("http.request", args, 3)?;
            let method = Method::from_bytes(method.as_bytes()).map_err(|err| {
                Error::Interpreter(format!("http.request: invalid method: {err}"))
            })?;
            execute_request(method, url, Some(body), Some(headers))
        }),
    );

    context.register_native_module("std.http", exports);
}

fn execute_request(
    method: Method,
    url: &str,
    body: Option<serde_json::Value>,
    headers: Option<HeaderMap>,
) -> Result<ZunValue, Error> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| Error::Interpreter(format!("http.request: {err}")))?;

    let mut request = client.request(method, url);
    if let Some(headers) = headers {
        request = request.headers(headers);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request
        .send()
        .map_err(|err| Error::Interpreter(format!("http.request: {err}")))?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = response
        .text()
        .map_err(|err| Error::Interpreter(format!("http.request: {err}")))?;

    response_to_zun_map(status, body, &headers)
}

fn response_to_zun_map(status: u16, body: String, headers: &HeaderMap) -> Result<ZunValue, Error> {
    let mut map = IndexMap::new();
    map.insert("status".into(), ZunValue::Int(status as i64));
    map.insert("body".into(), ZunValue::Str(body));
    map.insert("headers".into(), headers_to_map(headers)?);
    Ok(ZunValue::Map(map))
}

fn headers_to_map(headers: &HeaderMap) -> Result<ZunValue, Error> {
    let mut out = IndexMap::new();
    for name in headers.keys() {
        let joined = headers
            .get_all(name)
            .iter()
            .map(|value| {
                value.to_str().map(|s| s.to_string()).map_err(|err| {
                    Error::Interpreter(format!(
                        "http.request: invalid response header value: {err}"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        out.insert(name.as_str().to_string(), ZunValue::Str(joined));
    }
    Ok(ZunValue::Map(out))
}

fn string_arg<'a>(id: &str, args: &'a [ZunValue], index: usize) -> Result<&'a str, Error> {
    match args.get(index) {
        Some(ZunValue::Str(s)) => Ok(s),
        Some(other) => Err(Error::Interpreter(format!(
            "{id}: argument {} must be String, got {}",
            index + 1,
            other.type_name()
        ))),
        None => Err(Error::Interpreter(format!(
            "{id}: missing argument {}",
            index + 1
        ))),
    }
}

fn json_body_arg(id: &str, args: &[ZunValue], index: usize) -> Result<serde_json::Value, Error> {
    match args.get(index) {
        Some(ZunValue::Map(_)) | Some(ZunValue::Json(_)) => {
            Ok(value_to_json(args.get(index).expect("argument exists")))
        }
        Some(other) => Err(Error::Interpreter(format!(
            "{id}: argument {} must be Map or Json, got {}",
            index + 1,
            other.type_name()
        ))),
        None => Err(Error::Interpreter(format!(
            "{id}: missing argument {}",
            index + 1
        ))),
    }
}

fn optional_headers_arg(
    id: &str,
    args: &[ZunValue],
    index: usize,
) -> Result<Option<HeaderMap>, Error> {
    match args.get(index) {
        Some(_) => headers_arg(id, args, index).map(Some),
        None => Ok(None),
    }
}

fn headers_arg(id: &str, args: &[ZunValue], index: usize) -> Result<HeaderMap, Error> {
    let map = match args.get(index) {
        Some(ZunValue::Map(map)) | Some(ZunValue::Json(map)) => map,
        Some(other) => {
            return Err(Error::Interpreter(format!(
                "{id}: argument {} must be Map, got {}",
                index + 1,
                other.type_name()
            )));
        }
        None => {
            return Err(Error::Interpreter(format!(
                "{id}: missing argument {}",
                index + 1
            )));
        }
    };

    let mut headers = HeaderMap::new();
    for (key, value) in map {
        let name = HeaderName::from_bytes(key.as_bytes()).map_err(|err| {
            Error::Interpreter(format!("{id}: invalid header name '{key}': {err}"))
        })?;
        let value = match value {
            ZunValue::Str(s) => HeaderValue::from_str(s).map_err(|err| {
                Error::Interpreter(format!("{id}: invalid header value for '{key}': {err}"))
            })?,
            other => {
                return Err(Error::Interpreter(format!(
                    "{id}: header '{}' must be String, got {}",
                    key,
                    other.type_name()
                )));
            }
        };
        headers.append(name, value);
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::{execute_request, headers_to_map};
    use crate::interpreter::ZunValue;
    use reqwest::Method;
    use reqwest::header::{HeaderMap, HeaderValue};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn folds_response_headers_into_string_map() {
        let mut headers = HeaderMap::new();
        headers.append("x-test", HeaderValue::from_static("a"));
        headers.append("x-test", HeaderValue::from_static("b"));
        let map = headers_to_map(&headers).unwrap();
        match map {
            ZunValue::Map(map) => {
                assert_eq!(map.get("x-test"), Some(&ZunValue::Str("a, b".to_string())));
            }
            other => panic!("expected Map, got {}", other.type_name()),
        }
    }

    #[test]
    fn get_returns_status_body_and_headers_against_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer).unwrap();
            let body = "hello";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        });

        let url = format!("http://{}", addr);
        let result = execute_request(Method::GET, &url, None, None).unwrap();
        handle.join().unwrap();

        match result {
            ZunValue::Map(map) => {
                assert_eq!(map.get("status"), Some(&ZunValue::Int(200)));
                assert_eq!(map.get("body"), Some(&ZunValue::Str("hello".to_string())));
                match map.get("headers") {
                    Some(ZunValue::Map(headers)) => {
                        assert_eq!(
                            headers.get("content-type"),
                            Some(&ZunValue::Str("text/plain".to_string()))
                        );
                    }
                    other => panic!("expected headers map, got {:?}", other),
                }
            }
            other => panic!("expected Map, got {}", other.type_name()),
        }
    }
}
