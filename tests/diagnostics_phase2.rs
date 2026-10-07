use std::path::PathBuf;

use zun::diagnostics::codes;
use zun::lexer::Lexer;
use zun::parse;
use zun::runtime::{ProgramState, RuntimeContext, RuntimeMode, execute_source_with_writer};

fn run_source_error(source: &str) -> zun::error::Error {
    let mut state = ProgramState::new();
    let mut context = RuntimeContext::new(RuntimeMode::Test, PathBuf::from("."));
    let mut output = Vec::new();

    execute_source_with_writer(source, &mut state, &mut context, &mut output)
        .expect_err("source should fail")
}

#[test]
fn lexer_error_reports_line_and_column() {
    let mut lexer = Lexer::new("$ a = 1;\n&");
    let diagnostic = lexer.lex_all().expect_err("lexer should fail");

    assert_eq!(diagnostic.code, codes::LEX_UNEXPECTED_CHAR);
    assert_eq!(diagnostic.line, Some(2));
    assert_eq!(diagnostic.column, Some(1));
}

#[test]
fn parser_error_reports_line_and_column() {
    let error = parse("$ a = 1;\n$ b = 2 + ;").expect_err("parser should fail");
    let diagnostic = error.diagnostic();

    assert!(matches!(
        diagnostic.code,
        codes::PARSE_GENERIC | codes::PARSE_EXPECTED_TOKEN
    ));
    assert_eq!(diagnostic.line, Some(2));
    assert!(!diagnostic.message.is_empty());
}

#[test]
fn runtime_division_by_zero_uses_structured_diagnostic() {
    let error = run_source_error("$ value = 1 / 0;");
    let diagnostic = error.diagnostic();

    assert_eq!(diagnostic.code, codes::RUNTIME_DIVISION_BY_ZERO);
    assert_eq!(diagnostic.message, "division by zero");
    assert!(error.to_string().contains("error[ZUN-R002]"));
}

#[test]
fn runtime_undefined_variable_uses_structured_diagnostic() {
    let error = run_source_error("$ value = missing;");
    let diagnostic = error.diagnostic();

    assert_eq!(diagnostic.code, codes::RUNTIME_UNDEFINED_VARIABLE);
    assert!(diagnostic.message.contains("missing"));
}

#[test]
fn module_import_failure_uses_structured_diagnostic() {
    let error = run_source_error("$mod missing.module;");
    let diagnostic = error.diagnostic();

    assert_eq!(diagnostic.code, codes::RUNTIME_MODULE_LOAD);
    assert!(diagnostic.message.contains("module"));
}
