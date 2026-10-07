// HTTP Test Runner - handles zun test mode
use std::path::Path;
use std::process;

use serde_json::Value as JsonValue;

use zun::ast::TypeExpr;
use zun::interpreter::{HttpRoute, ZunValue, type_expr_name};
use zun::runtime::{
    HandlerInput, ProgramState, RuntimeMode, execute_http_route, execute_program_with_writer,
    load_context_and_program,
};

/// Test configuration (from CLI)
pub struct TestConfig {
    pub file: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub body: Option<String>,
}

/// Test result
#[derive(Debug)]
pub struct TestResult {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub body: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Run tests with the given configuration
pub fn run_test(config: TestConfig) {
    // Determine the file to test
    let test_file = config
        .file
        .clone()
        .unwrap_or_else(|| "main.zun".to_string());

    // Check if file exists
    if !std::path::Path::new(&test_file).exists() {
        eprintln!("error[ZUN-C001]: Test file not found: {}", test_file);
        process::exit(1);
    }

    println!("Running tests from: {}", test_file);
    println!();

    let (mut context, program) =
        match load_context_and_program(RuntimeMode::Test, Path::new(&test_file)) {
            Ok(loaded) => loaded,
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        };

    let mut state = ProgramState::new();
    match execute_program_with_writer(
        &program.statements,
        &mut state,
        &mut context,
        &mut std::io::stdout(),
    ) {
        Ok(true) => {}
        Ok(false) => process::exit(0),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    let routes = context.routes().to_vec();

    if routes.is_empty() {
        println!("No routes registered for testing.");
        process::exit(0);
    }

    // If --route specified, run specific test
    if let (Some(method), Some(path)) = (&config.method, &config.path) {
        let result = run_route_test(&routes, &context, method, path, config.body.as_deref());
        print_test_result(&result);
        if !result.success {
            process::exit(1);
        }
        return;
    }

    // Otherwise, run all registered routes
    println!("Testing {} route(s):", routes.len());
    println!();

    let mut all_passed = true;
    for route in &routes {
        let result = run_route_test(&routes, &context, &route.method, &route.path, None);
        print_test_result(&result);
        if !result.success {
            all_passed = false;
        }
    }

    if all_passed {
        println!();
        println!("All tests passed!");
    } else {
        println!();
        println!("Some tests failed!");
        process::exit(1);
    }
}

/// Run a test for a specific route
fn run_route_test(
    routes: &[HttpRoute],
    context: &zun::runtime::RuntimeContext,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> TestResult {
    // Find matching route
    let route = match routes.iter().find(|r| r.method == method && r.path == path) {
        Some(r) => r,
        None => {
            return TestResult {
                method: method.to_string(),
                path: path.to_string(),
                status: 404,
                body: String::new(),
                success: false,
                error: Some(format!("Route {} {} not found", method, path)),
            };
        }
    };

    let mut input = HandlerInput::new(path);
    if let Some(body_content) = body {
        input.body = Some(ZunValue::Str(body_content.to_string()));
    }

    let (should_continue, result, error) = execute_http_route(route, &input, context);

    // Handle error
    if let Some(err_msg) = error {
        return TestResult {
            method: method.to_string(),
            path: path.to_string(),
            status: 500,
            body: String::new(),
            success: false,
            error: Some(err_msg),
        };
    }

    if !should_continue {
        return TestResult {
            method: method.to_string(),
            path: path.to_string(),
            status: 500,
            body: String::new(),
            success: false,
            error: Some("Handler execution failed".to_string()),
        };
    }

    // Build response
    let return_type = route
        .return_type
        .clone()
        .unwrap_or_else(|| TypeExpr::Named("JSON".to_string()));
    let response_body = build_response(result, &return_type);

    TestResult {
        method: method.to_string(),
        path: path.to_string(),
        status: 200,
        body: response_body,
        success: true,
        error: None,
    }
}

/// Convert ZunValue to serde_json::Value
fn value_to_json(v: &ZunValue) -> JsonValue {
    match v {
        ZunValue::Null => JsonValue::Null,
        ZunValue::Bool(b) => JsonValue::Bool(*b),
        ZunValue::Int(i) => JsonValue::Number(serde_json::Number::from(*i)),
        ZunValue::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                JsonValue::Number(n)
            } else {
                JsonValue::Null
            }
        }
        ZunValue::Str(s) => JsonValue::String(s.clone()),
        ZunValue::List(list) => {
            let arr: Vec<JsonValue> = list.iter().map(value_to_json).collect();
            JsonValue::Array(arr)
        }
        ZunValue::Json(map) => {
            let mut obj: serde_json::Map<String, JsonValue> = serde_json::Map::new();
            for item in map.iter() {
                let k: &String = item.0;
                let v: &ZunValue = item.1;
                obj.insert(k.to_string(), value_to_json(v));
            }
            JsonValue::Object(obj)
        }
        ZunValue::TypedInstance { fields, .. } => {
            let mut obj: serde_json::Map<String, JsonValue> = serde_json::Map::new();
            for (k, v) in fields {
                if !k.starts_with('_') {
                    obj.insert(k.clone(), value_to_json(v));
                }
            }
            JsonValue::Object(obj)
        }
        _ => JsonValue::Null,
    }
}

/// Build HTTP response from handler result
fn build_response(result: Option<ZunValue>, return_type: &TypeExpr) -> String {
    let return_type_name = type_expr_name(return_type);
    match result {
        Some(val) => match return_type_name.as_str() {
            "String" => match val {
                ZunValue::Str(s) => s,
                _ => val.to_string(),
            },
            "JSON" | "json" => {
                let json_val = value_to_json(&val);
                serde_json::to_string(&json_val).unwrap_or_else(|_| "{}".to_string())
            }
            _ => val.to_string(),
        },
        None => String::new(),
    }
}

/// Print test result
fn print_test_result(result: &TestResult) {
    print!("  {} {} => ", result.method, result.path);

    if result.success {
        println!("{} {}", result.status, result.body);
    } else {
        println!("FAILED");
        if let Some(err) = &result.error {
            println!("    Error: {}", err);
        }
    }
}
