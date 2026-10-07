//! Zun error types

use std::fmt;

use crate::ast::Span;
use crate::diagnostics::codes;
use crate::diagnostics::{Diagnostic, render_diagnostic};

/// Runtime error types
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RuntimeError {
    TypeMismatch {
        name: String,
        expected: String,
        actual: String,
    },
    UndefinedVariable(String),
    UndefinedFunction(String),
    ConstReassign(String),
    FunctionNoReturnType {
        name: String,
    },
    DivisionByZero,
    ModuloByZero,
    Custom(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::TypeMismatch {
                name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "type mismatch: cannot assign {} to variable '{}' of type {}",
                    actual, name, expected
                )
            }
            RuntimeError::UndefinedVariable(name) => {
                write!(f, "variable '{}' not found", name)
            }
            RuntimeError::UndefinedFunction(name) => {
                write!(f, "function '{}' is not defined", name)
            }
            RuntimeError::ConstReassign(name) => {
                write!(f, "cannot reassign constant '{}'", name)
            }
            RuntimeError::FunctionNoReturnType { name } => {
                write!(
                    f,
                    "function '{}' has no return type declared but returns a value",
                    name
                )
            }
            RuntimeError::DivisionByZero => {
                write!(f, "division by zero")
            }
            RuntimeError::ModuloByZero => {
                write!(f, "modulo by zero")
            }
            RuntimeError::Custom(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Unified error type for Zun
#[derive(Debug, Clone)]
pub enum Error {
    /// Parse error
    Parse(ParseError),
    /// Structured diagnostic
    Diagnostic(Diagnostic),
    /// Runtime error
    Runtime(RuntimeError),
    /// Invalid expression
    InvalidExpression(Option<String>),
    /// Invalid assignment
    InvalidAssignment(Option<String>),
    /// Invalid statement
    InvalidStatement(Option<String>),
    /// Type mismatch error
    TypeMismatch(String),
    /// Lexer error
    Lexer(String),
    /// Interpreter error
    Interpreter(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", render_diagnostic(&self.diagnostic()))
    }
}

impl std::error::Error for Error {}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Parse(err)
    }
}

impl From<Diagnostic> for Error {
    fn from(err: Diagnostic) -> Self {
        Error::Diagnostic(err)
    }
}

impl From<RuntimeError> for Error {
    fn from(err: RuntimeError) -> Self {
        Error::Runtime(err)
    }
}

impl Error {
    pub fn diagnostic(&self) -> Diagnostic {
        match self {
            Error::Parse(err) => err.diagnostic(),
            Error::Diagnostic(err) => err.clone(),
            Error::Runtime(err) => Diagnostic::error(codes::RUNTIME_GENERIC, err.to_string()),
            Error::InvalidExpression(detail) => {
                let mut diagnostic =
                    Diagnostic::error(codes::RUNTIME_INVALID_EXPRESSION, "invalid expression");
                if let Some(detail) = detail {
                    diagnostic = diagnostic.with_note(detail.clone());
                }
                diagnostic
            }
            Error::InvalidAssignment(detail) => {
                let mut diagnostic =
                    Diagnostic::error(codes::RUNTIME_INVALID_ASSIGNMENT, "invalid assignment");
                if let Some(detail) = detail {
                    diagnostic = diagnostic.with_note(detail.clone());
                }
                diagnostic
            }
            Error::InvalidStatement(detail) => {
                let mut diagnostic =
                    Diagnostic::error(codes::PARSE_EXPECTED_TOKEN, "invalid statement");
                if let Some(detail) = detail {
                    diagnostic = diagnostic.with_note(detail.clone());
                }
                diagnostic
            }
            Error::TypeMismatch(msg) => {
                Diagnostic::error(codes::RUNTIME_TYPE_MISMATCH, msg.clone())
            }
            Error::Lexer(msg) => Diagnostic::error(codes::LEX_UNEXPECTED_CHAR, msg.clone()),
            Error::Interpreter(msg) => Diagnostic::error(codes::RUNTIME_GENERIC, msg.clone()),
        }
    }

    pub fn with_file(self, file: impl Into<String>) -> Self {
        Error::Diagnostic(self.diagnostic().with_file(file))
    }

    pub fn with_span(self, span: Span) -> Self {
        Error::Diagnostic(self.diagnostic().with_span(span))
    }

    pub fn with_location(self, line: usize, column: usize) -> Self {
        Error::Diagnostic(self.diagnostic().with_location(line, column))
    }
}

/// Parse error with context information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub found: Option<String>,
    pub expected: Option<String>,
}

impl ParseError {
    #[allow(dead_code)]
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            line: 1,
            column: 1,
            found: None,
            expected: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = line;
        self.column = column;
        self
    }

    #[allow(dead_code)]
    pub fn with_found(mut self, found: &str) -> Self {
        self.found = Some(found.to_string());
        self
    }

    #[allow(dead_code)]
    pub fn with_expected(mut self, expected: &str) -> Self {
        self.expected = Some(expected.to_string());
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", render_diagnostic(&self.diagnostic()))
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub fn diagnostic(&self) -> Diagnostic {
        let mut diagnostic = Diagnostic::error(codes::PARSE_GENERIC, self.message.clone())
            .with_location(self.line, self.column);
        if let Some(found) = &self.found {
            diagnostic = diagnostic.with_note(format!("found: {found}"));
        }
        if let Some(expected) = &self.expected {
            diagnostic = diagnostic.with_note(format!("expected: {expected}"));
        }
        diagnostic
    }
}
