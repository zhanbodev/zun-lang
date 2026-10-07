use std::borrow::Cow;

use crate::ast::{
    FStringLiteral, PrintStmt, PrintTarget, ReadMode, ReadStmt, Span, Stmt, StringLiteral,
};
use crate::diagnostics::{Diagnostic, codes};
use crate::error::Error;
use crate::token::{Token, Type};

use super::StmtParser;

impl<'a> StmtParser<'a> {
    pub fn parse_print_stmt(&mut self) -> Result<Option<Stmt>, Error> {
        let start = self.peek().pos;
        let print_literal = self.peek().literal.clone();
        self.advance();

        let expr_toks = self.collect_until_semi();
        if expr_toks.is_empty() {
            return Err(Error::InvalidExpression(None));
        }

        if print_literal == "$>>FILE"
            || (print_literal == "$>>"
                && expr_toks.len() >= 2
                && expr_toks[0].typ == Type::Ident
                && expr_toks[0].literal == "FILE"
                && expr_toks[1].typ == Type::LParen)
        {
            return Err(self.legacy_file_write_error(start));
        }

        if expr_toks.len() >= 2 && expr_toks[0].typ == Type::Ident && expr_toks[0].literal == "ERR"
        {
            let value = self.parse_stderr_target(&expr_toks)?;
            self.skip_semis();
            return Ok(Some(Stmt::Print(PrintStmt {
                span: Span::from_token(start),
                value,
                target: PrintTarget::Stderr,
            })));
        }

        let expr = self.parse_expr_tokens(&expr_toks)?;
        self.skip_semis();
        Ok(Some(Stmt::Print(PrintStmt {
            span: Span::from_token(start),
            value: expr,
            target: PrintTarget::Stdout,
        })))
    }

    pub fn parse_read_stmt(&mut self) -> Result<Option<Stmt>, Error> {
        let start = self.peek().pos;
        self.advance();

        let expr_toks = self.collect_until_semi();
        if expr_toks.is_empty() {
            return Err(Error::Parse(crate::error::ParseError {
                message: "invalid read statement, use $<<ENV(\"KEY\") or $<<LINE(\"prompt\")"
                    .to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: None,
            }));
        }
        if expr_toks[0].typ != Type::Ident {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected ENV or LINE after $<<".to_string(),
                line: 1,
                column: 1,
                found: Some(format!("{:?}", expr_toks[0].typ)),
                expected: Some("ENV or LINE".to_string()),
            }));
        }

        let mode_name = expr_toks[0].literal.clone();
        if mode_name == "FILE" {
            return Err(self.legacy_file_read_error(start));
        }
        if mode_name != "ENV" && mode_name != "LINE" {
            return Err(Error::Parse(crate::error::ParseError {
                message: format!("unknown read mode '{}', expected ENV or LINE", mode_name),
                line: 1,
                column: 1,
                found: Some(mode_name),
                expected: Some("ENV or LINE".to_string()),
            }));
        }

        if expr_toks.len() < 2 || expr_toks[1].typ != Type::LParen {
            return Err(Error::Parse(crate::error::ParseError {
                message: format!(
                    "{} requires parentheses: $<<{}(\"KEY\")",
                    mode_name, mode_name
                ),
                line: 1,
                column: 1,
                found: None,
                expected: Some("(".to_string()),
            }));
        }

        let args = collect_top_level_string_args(&expr_toks, 2)?;

        let mode = if mode_name == "ENV" {
            if args.len() != 1 {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "ENV requires a key: $<<ENV(\"KEY\")".to_string(),
                    line: 1,
                    column: 1,
                    found: if args.is_empty() {
                        None
                    } else {
                        Some("multiple arguments".to_string())
                    },
                    expected: Some("one string argument".to_string()),
                }));
            }
            if args[0].typ != Type::String {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "ENV key must be a String".to_string(),
                    line: 1,
                    column: 1,
                    found: Some(format!("{:?}", args[0].typ)),
                    expected: Some("String".to_string()),
                }));
            }
            ReadMode::Env
        } else {
            if args.len() > 1 {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "LINE accepts at most one argument".to_string(),
                    line: 1,
                    column: 1,
                    found: Some(format!("{} arguments", args.len())),
                    expected: Some("0 or 1 argument".to_string()),
                }));
            }
            if args.len() == 1 && args[0].typ != Type::String {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "LINE prompt must be a String".to_string(),
                    line: 1,
                    column: 1,
                    found: Some(format!("{:?}", args[0].typ)),
                    expected: Some("String".to_string()),
                }));
            }
            ReadMode::Line
        };

        let prompt = args.first().map(parse_string_like_expr).transpose()?;

        self.skip_semis();
        Ok(Some(Stmt::Read(ReadStmt {
            span: Span::from_token(start),
            mode,
            prompt,
        })))
    }

    fn parse_stderr_target(&self, expr_toks: &[Token]) -> Result<Box<crate::ast::Expr>, Error> {
        if expr_toks[1].typ != Type::LParen {
            return Err(Error::Parse(crate::error::ParseError {
                message: "ERR requires parentheses: $>>ERR(\"msg\")".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: None,
            }));
        }

        let mut found_arg: Option<(Token, bool)> = None;
        let mut paren_depth = 0;
        for tok in expr_toks.iter().skip(2) {
            match tok.typ {
                Type::LParen => paren_depth += 1,
                Type::RParen => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                }
                Type::String if paren_depth == 0 && found_arg.is_none() => {
                    found_arg = Some((tok.clone(), false));
                }
                Type::FString if paren_depth == 0 && found_arg.is_none() => {
                    found_arg = Some((tok.clone(), true));
                }
                _ => {}
            }
        }

        let (arg_tok, is_fstring) = found_arg.ok_or_else(|| {
            Error::Parse(crate::error::ParseError {
                message: "ERR requires a string argument: $>>ERR(\"msg\")".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: None,
            })
        })?;

        parse_string_like_expr_with_flag(&arg_tok, is_fstring)
    }

    fn legacy_file_write_error(&self, pos: usize) -> Error {
        let (line, column) = crate::parser::calc_line_col(self.src, pos);
        Error::Diagnostic(
            Diagnostic::error(
                codes::PARSE_LEGACY_FILE_SYNTAX,
                "legacy file syntax `$>>FILE(...)` has been removed",
            )
            .with_location(line, column)
            .with_span(Span::from_token(pos))
            .with_note("replace `$>>FILE(path, content)` with `std.fs.write(path, content)`")
            .with_note(
                "replace append/delete cases with `std.fs.append(...)` / `std.fs.delete(...)`",
            ),
        )
    }

    fn legacy_file_read_error(&self, pos: usize) -> Error {
        let (line, column) = crate::parser::calc_line_col(self.src, pos);
        Error::Diagnostic(
            Diagnostic::error(
                codes::PARSE_LEGACY_FILE_SYNTAX,
                "legacy file syntax `$<<FILE(...)` has been removed",
            )
            .with_location(line, column)
            .with_span(Span::from_token(pos))
            .with_note("replace `$<<FILE(path)` with `std.fs.read_text(path)`")
            .with_note("replace `$<<FILE(path, \"LINES\")` with `std.fs.read_lines(path)`"),
        )
    }
}

fn collect_top_level_string_args(
    expr_toks: &[Token],
    start_index: usize,
) -> Result<Vec<Token>, Error> {
    let mut args = Vec::new();
    let mut paren_depth = 1;
    let mut i = start_index;
    while i < expr_toks.len() {
        let tok = &expr_toks[i];
        match tok.typ {
            Type::LParen => paren_depth += 1,
            Type::RParen => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if paren_depth == 1 && (tok.typ == Type::String || tok.typ == Type::FString) {
            args.push(tok.clone());
        }
        i += 1;
    }

    if i >= expr_toks.len() || expr_toks[i].typ != Type::RParen {
        return Err(Error::Parse(crate::error::ParseError {
            message: "expected closing parenthesis".to_string(),
            line: 1,
            column: 1,
            found: None,
            expected: Some(")".to_string()),
        }));
    }

    Ok(args)
}

fn parse_string_like_expr(tok: &Token) -> Result<Box<crate::ast::Expr>, Error> {
    parse_string_like_expr_with_flag(tok, tok.typ == Type::FString)
}

fn parse_string_like_expr_with_flag(
    tok: &Token,
    is_fstring: bool,
) -> Result<Box<crate::ast::Expr>, Error> {
    if is_fstring {
        let segments =
            crate::parser::expr::parse_fstring_segments(&tok.literal, tok.pos).map_err(|e| {
                Error::Parse(crate::error::ParseError {
                    message: e.to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: None,
                })
            })?;
        Ok(Box::new(crate::ast::Expr::FString(FStringLiteral {
            span: Span::from_token(tok.pos),
            segments,
        })))
    } else {
        Ok(Box::new(crate::ast::Expr::StringLiteral(StringLiteral {
            span: Span::from_token(tok.pos),
            value: Cow::Owned(tok.literal.clone()),
        })))
    }
}
