use crate::ast::{Span, StaticStmt, Stmt};
use crate::error::Error;
use crate::token::Type;

use super::StmtParser;

impl<'a> StmtParser<'a> {
    pub fn parse_static_decl(&mut self) -> Result<Stmt, Error> {
        self.advance();
        let mut tokens = Vec::new();
        while !self.at_end() && self.peek().typ != Type::Semicolon {
            tokens.push(self.peek().clone());
            self.advance();
        }
        if !self.at_end() && self.peek().typ == Type::Semicolon {
            self.advance();
        }
        parse_static_stmt(&tokens)
    }
}

/// Parse $STATIC("/url-prefix", "dir.module") or $STATIC("dir")
pub fn parse_static_stmt(tokens: &[crate::token::Token]) -> Result<Stmt, Error> {
    if tokens.is_empty() {
        return Err(Error::Parse(crate::error::ParseError {
            message: "$STATIC requires arguments".to_string(),
            line: 1,
            column: 1,
            expected: Some("arguments".to_string()),
            found: None,
        }));
    }

    let mut args = Vec::new();
    let mut collected = String::new();
    let mut paren_depth = 0;

    for tok in tokens {
        match tok.typ {
            Type::LParen => {
                if paren_depth != 0 {
                    collected.push_str(&tok.literal);
                }
                paren_depth += 1;
            }
            Type::RParen => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    if !collected.is_empty() {
                        args.push(collected.trim().to_string());
                    }
                    break;
                }
                collected.push_str(&tok.literal);
            }
            Type::Comma if paren_depth == 1 => {
                args.push(collected.trim().to_string());
                collected.clear();
            }
            _ => collected.push_str(&tok.literal),
        }
    }

    let (url_prefix, module_path) = match args.len() {
        1 => ("/static".to_string(), args[0].clone()),
        2 => {
            let prefix = args[0].trim_matches('"').to_string();
            let module = args[1].trim_matches('"').to_string();
            (prefix, module)
        }
        _ => {
            return Err(Error::Parse(crate::error::ParseError {
                message: "$STATIC requires 1 or 2 arguments: $STATIC(\"url-prefix\", \"module\") or $STATIC(\"module\")".to_string(),
                line: 1,
                column: 1,
                expected: Some("1 or 2 arguments".to_string()),
                found: Some(format!("{} arguments", args.len())),
            }));
        }
    };

    Ok(Stmt::Static(StaticStmt {
        span: Span::new(0, 0),
        url_prefix,
        module_path,
    }))
}
