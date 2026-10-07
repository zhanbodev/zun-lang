use std::collections::HashMap;
use std::io::{self, Read, Write};

use serde_json::{Value, json};
use zun_frontend::error::Error;

#[derive(Default)]
struct LspState {
    open_files: HashMap<String, String>,
}

fn main() {
    if let Err(err) = run_stdio() {
        let _ = writeln!(io::stderr(), "zun-lsp error: {err}");
    }
}

fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut state = LspState::default();

    while let Some(message) = read_message(&mut reader)? {
        if let Some(response) = handle_message(&mut state, &message) {
            write_message(&mut writer, &response)?;
        }
    }

    Ok(())
}

fn handle_message(state: &mut LspState, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str);
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    match method {
        Some("initialize") => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {
                        "textDocumentSync": 1
                    },
                    "serverInfo": {
                        "name": "zun-lsp",
                        "version": "0.1.0"
                    }
                }
            })
        }),
        Some("initialized") => None,
        Some("shutdown") => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null
            })
        }),
        Some("exit") => {
            std::process::exit(0);
        }
        Some("textDocument/didOpen") => publish_from_params(state, &params),
        Some("textDocument/didChange") => publish_from_params(state, &params),
        _ => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null
            })
        }),
    }
}

fn publish_from_params(state: &mut LspState, params: &Value) -> Option<Value> {
    let (uri, text) = extract_document(params)?;
    state.open_files.insert(uri.clone(), text.clone());
    let diagnostics = diagnostics_for_source(&uri, &text);

    Some(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics
        }
    }))
}

fn extract_document(params: &Value) -> Option<(String, String)> {
    if let Some(text_document) = params.get("textDocument") {
        let uri = text_document.get("uri")?.as_str()?.to_string();
        if let Some(text) = text_document.get("text").and_then(Value::as_str) {
            return Some((uri, text.to_string()));
        }
        if let Some(changes) = params.get("contentChanges").and_then(Value::as_array)
            && let Some(text) = changes
                .last()
                .and_then(|change| change.get("text"))
                .and_then(Value::as_str)
        {
            return Some((uri, text.to_string()));
        }
    }
    None
}

fn diagnostics_for_source(uri: &str, source: &str) -> Vec<Value> {
    match zun_frontend::parse(source) {
        Ok(_) => Vec::new(),
        Err(err) => vec![to_lsp_diagnostic(uri, err)],
    }
}

fn to_lsp_diagnostic(uri: &str, err: Error) -> Value {
    let diagnostic = err.with_file(uri.to_string()).diagnostic();
    let line = diagnostic.line.unwrap_or(1).saturating_sub(1) as u64;
    let column = diagnostic.column.unwrap_or(1).saturating_sub(1) as u64;
    let end_column = column.saturating_add(1);

    json!({
        "range": {
            "start": { "line": line, "character": column },
            "end": { "line": line, "character": end_column }
        },
        "severity": 1,
        "code": diagnostic.code,
        "source": "zun",
        "message": diagnostic.message,
        "relatedInformation": diagnostic.notes.iter().map(|note| {
            json!({
                "location": {
                    "uri": uri,
                    "range": {
                        "start": { "line": line, "character": column },
                        "end": { "line": line, "character": end_column }
                    }
                },
                "message": note
            })
        }).collect::<Vec<_>>()
    })
}

fn read_message(reader: &mut dyn Read) -> io::Result<Option<Value>> {
    let mut headers = Vec::new();
    let mut buffer = [0u8; 1];

    loop {
        match reader.read(&mut buffer)? {
            0 => return Ok(None),
            _ => {
                headers.push(buffer[0]);
                if headers.ends_with(b"\r\n\r\n") {
                    break;
                }
                if headers.ends_with(b"\n\n") {
                    break;
                }
            }
        }
    }

    let headers = String::from_utf8_lossy(&headers);
    let mut content_length = None;
    for line in headers.lines() {
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let Some(content_length) = content_length else {
        return Ok(None);
    };

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let message: Value = serde_json::from_slice(&body)?;
    Ok(Some(message))
}

fn write_message(writer: &mut dyn Write, message: &Value) -> io::Result<()> {
    let payload = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
    writer.write_all(&payload)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::diagnostics_for_source;

    #[test]
    fn returns_empty_diagnostics_for_valid_source() {
        let diagnostics = diagnostics_for_source("file:///valid.zun", "$ value = 1;");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn returns_syntax_diagnostic_for_invalid_source() {
        let diagnostics = diagnostics_for_source("file:///invalid.zun", "$ value = ;");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["source"], "zun");
        assert_eq!(diagnostics[0]["severity"], 1);
    }
}
