use std::borrow::Cow;

use crate::ast::{
    BoolLiteral, CharLiteral, Expr, FStringLiteral, FnCallExpr, HdrReadExpr, HtmlConstructor,
    JsonConstructor, ListLiteral, MapLiteral, NullLiteral, NumberLiteral, ResConstructor, Span,
    StringLiteral, StructConstructor, VarLookup,
};
use crate::diagnostics::{Diagnostic, codes};
use crate::error::Error;
use crate::token::Type;

use super::{ExprParser, parse_fstring_segments};

impl<'a> ExprParser<'a> {
    pub fn parse_primary(&mut self) -> Result<Box<Expr>, Error> {
        if self.pos >= self.tokens.len() {
            return Err(self.error("unexpected end of input"));
        }
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;

        match tok.typ {
            Type::Number => Ok(Box::new(Expr::Number(NumberLiteral {
                span: Span::from_token(tok.pos),
                value: Cow::Owned(tok.literal),
            }))),
            Type::Char => Ok(Box::new(Expr::Char(CharLiteral {
                span: Span::from_token(tok.pos),
                value: Cow::Owned(tok.literal),
            }))),
            Type::Bool => {
                let value = tok.literal == "true";
                Ok(Box::new(Expr::Bool(BoolLiteral {
                    span: Span::from_token(tok.pos),
                    value,
                })))
            }
            Type::Null => Ok(Box::new(Expr::Null(NullLiteral {
                span: Span::from_token(tok.pos),
            }))),
            Type::String => Ok(Box::new(Expr::StringLiteral(StringLiteral {
                span: Span::from_token(tok.pos),
                value: Cow::Owned(tok.literal),
            }))),
            Type::FString => {
                let segments = parse_fstring_segments(&tok.literal, tok.pos)?;
                Ok(Box::new(Expr::FString(FStringLiteral {
                    span: Span::from_token(tok.pos),
                    segments,
                })))
            }
            Type::Fn => self.parse_fn_literal(),
            Type::Ident => {
                if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::LParen {
                    self.pos += 1;
                    let mut args: Vec<Expr> = Vec::new();
                    if self.pos < self.tokens.len() && self.tokens[self.pos].typ != Type::RParen {
                        loop {
                            let arg = self.parse_expr()?;
                            args.push(*arg);
                            if self.pos >= self.tokens.len()
                                || self.tokens[self.pos].typ != Type::Comma
                            {
                                break;
                            }
                            self.pos += 1;
                        }
                    }
                    if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
                        return Err(self
                            .error_expected("expected closing parenthesis in function call", ")"));
                    }
                    self.pos += 1;
                    Ok(Box::new(Expr::FnCall(FnCallExpr {
                        span: Span::from_token(tok.pos),
                        name: tok.literal,
                        args,
                    })))
                } else if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::LBrace
                {
                    self.parse_struct_constructor(tok.pos, tok.literal)
                } else {
                    Ok(Box::new(Expr::VarLookup(VarLookup {
                        span: Span::from_token(tok.pos),
                        name: Cow::Owned(tok.literal),
                    })))
                }
            }
            Type::LParen => {
                let expr = self.parse_expr()?;
                if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
                    return Err(self.error_expected("expected closing parenthesis", ")"));
                }
                self.pos += 1;
                Ok(expr)
            }
            Type::LBracket => self.parse_list_literal(tok.pos),
            Type::LBrace => self.parse_map_literal(tok.pos),
            Type::Print => self.parse_file_write_expr(tok),
            Type::Read => self.parse_read_expr(tok),
            Type::ConfigRead => self.parse_config_read_expr(tok.pos),
            Type::HdrRead => self.parse_hdr_read_expr(tok.pos),
            Type::Json => self.parse_json_constructor(tok.pos),
            Type::Html => self.parse_html_constructor(tok.pos),
            Type::Res => self.parse_res_constructor(tok.pos),
            _ => Err(self.error(&format!("unexpected token: {:?} {}", tok.typ, tok.literal))),
        }
    }

    fn parse_list_literal(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        let mut elements: Vec<Expr> = Vec::new();
        if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::RBracket {
            self.pos += 1;
            return Ok(Box::new(Expr::ListLiteral(ListLiteral {
                span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
                elements,
            })));
        }

        loop {
            let elem = self.parse_expr()?;
            elements.push(*elem);
            if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Comma {
                break;
            }
            self.pos += 1;
        }

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RBracket {
            return Err(self.error_expected("expected closing bracket in list literal", "]"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::ListLiteral(ListLiteral {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            elements,
        })))
    }

    fn parse_map_literal(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        let mut entries: Vec<(String, Expr)> = Vec::new();
        if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::RBrace {
            self.pos += 1;
            return Ok(Box::new(Expr::MapLiteral(MapLiteral {
                span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
                entries,
            })));
        }

        loop {
            if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::String {
                return Err(self.error_expected("expected string key in map literal", "string"));
            }
            let key = self.tokens[self.pos].literal.clone();
            self.pos += 1;

            if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Colon {
                return Err(self.error_expected("expected ':' in map literal", ":"));
            }
            self.pos += 1;

            let value = self.parse_expr()?;
            entries.push((key, *value));

            if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Comma {
                break;
            }
            self.pos += 1;
        }

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RBrace {
            return Err(self.error_expected("expected closing brace in map literal", "}"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::MapLiteral(MapLiteral {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            entries,
        })))
    }

    fn parse_file_write_expr(&mut self, tok: crate::token::Token) -> Result<Box<Expr>, Error> {
        let start = tok.pos;
        if tok.literal == "$>>"
            && self.pos + 1 < self.tokens.len()
            && self.tokens[self.pos].typ == Type::Ident
            && self.tokens[self.pos].literal == "FILE"
            && self.tokens[self.pos + 1].typ == Type::LParen
        {
            return Err(self.legacy_file_write_error(start));
        }

        if tok.literal != "$>>FILE" {
            return Err(self.error("unexpected token in expression"));
        }
        Err(self.legacy_file_write_error(start))
    }

    fn parse_read_expr(&mut self, tok: crate::token::Token) -> Result<Box<Expr>, Error> {
        let start = tok.pos;
        if tok.literal == "$<<FILE" {
            return Err(self.legacy_file_read_error(start));
        }

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Ident {
            return Err(self.error_expected("expected ENV or LINE after $<<", "identifier"));
        }
        let mode_name = self.tokens[self.pos].literal.clone();
        if mode_name == "FILE" {
            return Err(self.legacy_file_read_error(start));
        }
        if mode_name != "ENV" && mode_name != "LINE" {
            return Err(self.error_expected("expected ENV or LINE or FILE", &mode_name));
        }
        self.pos += 1;

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LParen {
            return Err(self.error_expected(&format!("expected '(' after {}", mode_name), "("));
        }
        self.pos += 1;

        let mut prompt = None;
        if self.pos < self.tokens.len()
            && (self.tokens[self.pos].typ == Type::String
                || self.tokens[self.pos].typ == Type::FString)
        {
            let arg_tok = self.tokens[self.pos].clone();
            self.pos += 1;
            if arg_tok.typ == Type::FString {
                let segments = parse_fstring_segments(&arg_tok.literal, arg_tok.pos)?;
                prompt = Some(Box::new(Expr::FString(FStringLiteral {
                    span: Span::from_token(arg_tok.pos),
                    segments,
                })));
            } else {
                prompt = Some(Box::new(Expr::StringLiteral(StringLiteral {
                    span: Span::from_token(arg_tok.pos),
                    value: Cow::Owned(arg_tok.literal),
                })));
            }
        }

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
            return Err(self.error_expected("expected ')'", ")"));
        }
        self.pos += 1;

        let mode = if mode_name == "ENV" {
            if prompt.is_none() {
                return Err(self.error_expected("ENV requires a key: $<<ENV(\"KEY\")", "string"));
            }
            crate::ast::ReadMode::Env
        } else {
            crate::ast::ReadMode::Line
        };

        Ok(Box::new(Expr::Read(crate::ast::ExprRead {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            mode,
            prompt,
        })))
    }

    fn legacy_file_write_error(&self, pos: usize) -> Error {
        let (line, column) = if self.src.is_empty() {
            (1, 1)
        } else {
            crate::parser::calc_line_col(self.src, pos)
        };
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
        let (line, column) = if self.src.is_empty() {
            (1, 1)
        } else {
            crate::parser::calc_line_col(self.src, pos)
        };
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

    fn parse_config_read_expr(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LParen {
            return Err(self.error_expected("expected '(' after CONFIG", "("));
        }
        self.pos += 1;

        let key = if self.pos < self.tokens.len()
            && (self.tokens[self.pos].typ == Type::String
                || self.tokens[self.pos].typ == Type::FString
                || self.tokens[self.pos].typ == Type::Ident)
        {
            let key = self.tokens[self.pos].literal.clone();
            self.pos += 1;
            key
        } else {
            return Err(self.error_expected("expected config key", "string or identifier"));
        };

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
            return Err(self.error_expected("expected ')'", ")"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::ConfigRead(crate::ast::ConfigReadExpr {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            key,
        })))
    }

    fn parse_hdr_read_expr(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LParen {
            return Err(self.error_expected("expected '(' after HDR", "("));
        }
        self.pos += 1;

        let header_name = if self.pos < self.tokens.len()
            && (self.tokens[self.pos].typ == Type::String
                || self.tokens[self.pos].typ == Type::FString
                || self.tokens[self.pos].typ == Type::Ident)
        {
            let name = self.tokens[self.pos].literal.clone();
            self.pos += 1;
            name
        } else {
            return Err(self.error_expected("expected header name", "string or identifier"));
        };

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
            return Err(self.error_expected("expected ')'", ")"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::HdrRead(HdrReadExpr {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            header_name,
        })))
    }

    fn parse_struct_constructor(
        &mut self,
        start: usize,
        type_name: String,
    ) -> Result<Box<Expr>, Error> {
        self.pos += 1; // consume '{'
        let mut fields: Vec<(String, Expr)> = Vec::new();

        while self.pos < self.tokens.len() && self.tokens[self.pos].typ != Type::RBrace {
            // field name (identifier)
            if self.tokens[self.pos].typ != Type::Ident {
                return Err(
                    self.error_expected("expected field name in struct constructor", "identifier")
                );
            }
            let field_name = self.tokens[self.pos].literal.clone();
            self.pos += 1;

            // colon
            if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Colon {
                return Err(
                    self.error_expected(&format!("expected ':' after field '{}'", field_name), ":")
                );
            }
            self.pos += 1;

            // value expression
            let val = self.parse_expr()?;
            fields.push((field_name, *val));

            // optional comma
            if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::Comma {
                self.pos += 1;
            }
        }

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RBrace {
            return Err(self.error_expected("expected '}' to close struct constructor", "}"));
        }
        self.pos += 1; // consume '}'

        Ok(Box::new(Expr::TypeInstance(StructConstructor {
            span: Span::from_token(start),
            type_name,
            fields,
        })))
    }

    fn parse_json_constructor(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LBrace {
            return Err(self.error_expected("expected '{' after $JSON", "{"));
        }
        self.pos += 1;

        let mut entries = Vec::new();
        while self.pos < self.tokens.len() && self.tokens[self.pos].typ != Type::RBrace {
            let key_tok = if self.pos < self.tokens.len()
                && (self.tokens[self.pos].typ == Type::String
                    || self.tokens[self.pos].typ == Type::FString)
            {
                self.tokens[self.pos].literal.clone()
            } else {
                return Err(self.error_expected("expected string key in $JSON", "string"));
            };
            self.pos += 1;

            if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Colon {
                return Err(self.error_expected("expected ':' after key", ":"));
            }
            self.pos += 1;

            let value = self.parse_expr()?;
            entries.push((key_tok, *value));

            if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::Comma {
                self.pos += 1;
            }
        }

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RBrace {
            return Err(self.error_expected("expected '}' in $JSON", "}"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::JsonConstructor(JsonConstructor {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            entries,
        })))
    }

    fn parse_html_constructor(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LParen {
            return Err(self.error_expected("expected '(' after $HTML", "("));
        }
        self.pos += 1;

        let content = if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::RParen {
            Box::new(Expr::StringLiteral(StringLiteral {
                span: Span::new(start, start + 1),
                value: Cow::Owned(String::new()),
            }))
        } else {
            self.parse_expr()?
        };

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
            return Err(self.error_expected("expected ')' in $HTML", ")"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::HtmlConstructor(HtmlConstructor {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            content,
        })))
    }

    fn parse_res_constructor(&mut self, start: usize) -> Result<Box<Expr>, Error> {
        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LParen {
            return Err(self.error_expected("expected '(' after $RES", "("));
        }
        self.pos += 1;

        let status = self.parse_expr()?;
        let body = if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::Comma {
            self.pos += 1;
            Some(self.parse_expr()?)
        } else {
            None
        };

        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
            return Err(self.error_expected("expected ')' in $RES", ")"));
        }
        self.pos += 1;

        Ok(Box::new(Expr::ResConstructor(ResConstructor {
            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
            status,
            body,
        })))
    }
}
