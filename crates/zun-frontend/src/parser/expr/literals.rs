use crate::ast::FStringSegment;
use crate::error::{Error, ParseError};
use crate::token::Type;

use super::parse_expr_tokens;

/// Parse f-string segments from the raw literal
/// Format: "Hello {name}, you have {count + 1} items"
pub fn parse_fstring_segments(
    literal: &str,
    start_pos: usize,
) -> Result<Vec<FStringSegment>, Error> {
    use crate::lexer::Lexer;

    let mut segments = Vec::new();
    let mut current_literal = String::new();
    let mut chars = literal.chars().peekable();
    let mut pos = start_pos;

    while let Some(ch) = chars.next() {
        pos += 1;
        match ch {
            '{' => {
                if !current_literal.is_empty() {
                    segments.push(FStringSegment::Literal(current_literal.clone()));
                    current_literal.clear();
                }

                let mut expr_str = String::new();
                let mut brace_count = 1;
                for c in chars.by_ref() {
                    pos += 1;
                    match c {
                        '{' => {
                            brace_count += 1;
                            expr_str.push(c);
                        }
                        '}' => {
                            brace_count -= 1;
                            if brace_count == 0 {
                                if expr_str.trim().is_empty() {
                                    return Err(Error::Parse(ParseError {
                                        message:
                                            "f-string syntax error: empty expression in \"{}\""
                                                .to_string(),
                                        line: 1,
                                        column: pos - 1,
                                        found: None,
                                        expected: None,
                                    }));
                                }

                                let mut lexer = Lexer::new(&expr_str);
                                let mut expr_tokens = lexer.lex_all().map_err(Error::from)?;
                                expr_tokens.retain(|t| t.typ != Type::Eof);
                                let expr = parse_expr_tokens(&expr_tokens).map_err(|e| {
                                    Error::Parse(ParseError {
                                        message: format!("f-string expression parse error: {}", e),
                                        line: 1,
                                        column: pos,
                                        found: None,
                                        expected: None,
                                    })
                                })?;
                                segments.push(FStringSegment::Expression(expr));
                                break;
                            } else {
                                expr_str.push(c);
                            }
                        }
                        _ => expr_str.push(c),
                    }
                }
                if brace_count > 0 {
                    return Err(Error::Parse(ParseError {
                        message: "f-string syntax error: unclosed \"{\"".to_string(),
                        line: 1,
                        column: pos,
                        found: None,
                        expected: None,
                    }));
                }
            }
            '}' => {
                return Err(Error::Parse(ParseError {
                    message: "f-string syntax error: unexpected \"}\"".to_string(),
                    line: 1,
                    column: pos,
                    found: None,
                    expected: None,
                }));
            }
            _ => current_literal.push(ch),
        }
    }

    if !current_literal.is_empty() {
        segments.push(FStringSegment::Literal(current_literal));
    }

    Ok(segments)
}
