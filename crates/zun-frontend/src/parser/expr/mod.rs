// Expression parser - handles all expression parsing.
mod literals;
mod precedence;
mod primary;

use crate::diagnostics::codes;
use crate::error::{Error, ParseError};
use crate::parser::calc_line_col;
use crate::token::Token;

pub use literals::parse_fstring_segments;

/// Parse a token slice into an expression.
pub fn parse_expr_tokens(toks: &[Token]) -> Result<Box<crate::ast::Expr>, Error> {
    parse_expr_tokens_with_src(toks, "")
}

/// Parse a token slice into an expression with source context.
pub fn parse_expr_tokens_with_src(
    toks: &[Token],
    src: &str,
) -> Result<Box<crate::ast::Expr>, Error> {
    if toks.is_empty() {
        return Err(Error::Parse(ParseError {
            message: "empty expression".to_string(),
            line: 1,
            column: 1,
            found: None,
            expected: Some("expression".to_string()),
        }));
    }

    let mut parser = ExprParser {
        tokens: toks,
        pos: 0,
        src,
    };
    let expr = parser.parse_expr()?;
    if parser.pos != toks.len() {
        let tok = parser.tokens.get(parser.pos);
        return Err(Error::Parse(ParseError {
            message: "unexpected token after expression".to_string(),
            line: 1,
            column: 1,
            found: tok.map(|t| format!("{:?}", t.typ)),
            expected: None,
        }));
    }
    Ok(expr)
}

pub struct ExprParser<'a> {
    pub tokens: &'a [Token],
    pub pos: usize,
    pub src: &'a str,
}

impl<'a> ExprParser<'a> {
    /// Create a parse error with context
    pub fn error(&self, message: &str) -> Error {
        let tok = self.tokens.get(self.pos).or_else(|| self.tokens.last());
        let (line, col) = if let Some(t) = tok {
            if self.src.is_empty() {
                (1, 1)
            } else {
                calc_line_col(self.src, t.pos)
            }
        } else {
            (1, 1)
        };

        let found = tok.map(|t| format!("{:?} {}", t.typ, t.literal));

        Error::Diagnostic(
            crate::diagnostics::Diagnostic::error(codes::PARSE_GENERIC, message)
                .with_location(line, col)
                .with_note(found.unwrap_or_else(|| "found: <eof>".to_string())),
        )
    }

    /// Create a parse error with expected token info
    pub fn error_expected(&self, message: &str, expected: &str) -> Error {
        let tok = self.tokens.get(self.pos).or_else(|| self.tokens.last());
        let (line, col) = if let Some(t) = tok {
            if self.src.is_empty() {
                (1, 1)
            } else {
                calc_line_col(self.src, t.pos)
            }
        } else {
            (1, 1)
        };

        let found = tok.map(|t| format!("{:?} {}", t.typ, t.literal));

        Error::Diagnostic(
            crate::diagnostics::Diagnostic::error(codes::PARSE_EXPECTED_TOKEN, message)
                .with_location(line, col)
                .with_note(format!(
                    "expected: {expected}; found: {}",
                    found.unwrap_or_else(|| "<eof>".to_string())
                )),
        )
    }

    /// Parse anonymous function literal: $fn(x, y) -> Int { body }
    pub fn parse_fn_literal(&mut self) -> Result<Box<crate::ast::Expr>, Error> {
        crate::parser::literal::parse_fn_literal(self)
    }
}
