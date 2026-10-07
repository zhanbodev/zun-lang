use std::collections::HashSet;

use crate::ast::{CorsConfig, FnParam, HttpBlockStmt, HttpFnStmt, SetHdrEntry, Span, Stmt};
use crate::diagnostics::codes;
use crate::error::Error;
use crate::token::Type;

use super::StmtParser;

#[derive(Debug, Default)]
pub(crate) struct HttpAnnotations {
    pub cors: Option<CorsConfig>,
    pub headers: Vec<SetHdrEntry>,
}

impl<'a> StmtParser<'a> {
    pub fn parse_http_fn(&mut self) -> Result<Stmt, Error> {
        let annotations = self.parse_http_annotations()?;
        self.parse_http_fn_with_annotations(annotations)
    }

    pub(crate) fn parse_http_fn_with_annotations(
        &mut self,
        annotations: HttpAnnotations,
    ) -> Result<Stmt, Error> {
        if self.at_end()
            || !matches!(
                self.peek().typ,
                Type::HttpGet | Type::HttpPost | Type::HttpPut | Type::HttpDel | Type::HttpPatch
            )
        {
            return Err(self.error_for_annotation_position(&annotations));
        }

        let start = self.peek().pos;
        let method = match self.peek().typ {
            Type::HttpGet => "GET",
            Type::HttpPost => "POST",
            Type::HttpPut => "PUT",
            Type::HttpDel => "DELETE",
            Type::HttpPatch => "PATCH",
            _ => unreachable!(),
        };
        self.advance();

        self.expect(Type::LParen)?;
        if self.at_end() || self.peek().typ != Type::String {
            return Err(Error::Parse(crate::error::ParseError {
                message: format!("expected path string after ${}", method.to_lowercase()),
                line: 1,
                column: 1,
                found: None,
                expected: Some("path string".to_string()),
            }));
        }
        let path = self.advance().literal.clone();
        self.expect(Type::RParen)?;

        if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected function name after path".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("function name".to_string()),
            }));
        }
        let name = self.advance().literal.clone();

        self.expect(Type::LParen)?;

        let mut params: Vec<FnParam> = Vec::new();
        let mut variadic_param: Option<String> = None;
        if !self.at_end() && self.peek().typ != Type::RParen {
            loop {
                if self.peek().typ == Type::Spread {
                    self.advance();
                    if self.at_end() || self.peek().typ != Type::Ident {
                        return Err(Error::InvalidStatement(None));
                    }
                    variadic_param = Some(self.advance().literal.clone());
                    break;
                }
                if self.at_end() || self.peek().typ != Type::Ident {
                    return Err(Error::InvalidStatement(None));
                }
                let name = self.advance().literal.clone();
                let type_annotation = if !self.at_end() && self.peek().typ == Type::Colon {
                    self.advance();
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                params.push(FnParam {
                    name,
                    type_annotation,
                });
                if self.at_end() || self.peek().typ != Type::Comma {
                    break;
                }
                self.advance();
            }
        }

        self.expect(Type::RParen)?;

        let return_type = if !self.at_end() && self.peek().typ == Type::Arrow {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Stmt::HttpFn(HttpFnStmt {
            span: Span::from_token(start),
            method: method.to_string(),
            path,
            name,
            params,
            variadic_param,
            return_type,
            cors: annotations.cors,
            headers: annotations.headers,
            body,
        }))
    }

    pub fn parse_http_block(&mut self) -> Result<Stmt, Error> {
        let annotations = self.parse_http_annotations()?;
        self.parse_http_block_with_annotations(annotations)
    }

    pub(crate) fn parse_http_block_with_annotations(
        &mut self,
        annotations: HttpAnnotations,
    ) -> Result<Stmt, Error> {
        if self.at_end() || self.peek().typ != Type::HttpBlock {
            return Err(self.error_for_annotation_position(&annotations));
        }

        let start = self.peek().pos;
        self.advance();

        let mut prefix: Option<String> = None;
        if !self.at_end() && self.peek().typ == Type::LParen {
            self.advance();
            let mut path = String::new();
            while !self.at_end() && self.peek().typ != Type::RParen {
                path.push_str(&self.peek().literal);
                self.advance();
            }
            if self.at_end() || self.peek().typ != Type::RParen {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "expected ')' after $HTTP(path".to_string(),
                    line: 1,
                    column: 1,
                    expected: Some("')'".to_string()),
                    found: None,
                }));
            }
            self.advance();
            prefix = Some(path);
        }

        if !self.at_end() && self.peek().typ == Type::Dot {
            let mut links: Vec<String> = Vec::new();
            while !self.at_end() && self.peek().typ == Type::Dot {
                self.advance();
                if self.at_end() || self.peek().literal != "link" {
                    return Err(Error::Parse(crate::error::ParseError {
                        message: "expected 'link' after '.'".to_string(),
                        line: 1,
                        column: 1,
                        expected: Some("'link'".to_string()),
                        found: None,
                    }));
                }
                self.advance();
                if self.at_end() || self.peek().typ != Type::LParen {
                    return Err(Error::Parse(crate::error::ParseError {
                        message: "expected '(' after .link".to_string(),
                        line: 1,
                        column: 1,
                        expected: Some("'('".to_string()),
                        found: None,
                    }));
                }
                self.advance();

                let mut module_path = String::new();
                while !self.at_end() && self.peek().typ != Type::RParen {
                    module_path.push_str(&self.peek().literal);
                    self.advance();
                }
                if self.at_end() || self.peek().typ != Type::RParen {
                    return Err(Error::Parse(crate::error::ParseError {
                        message: "expected ')' after .link(module".to_string(),
                        line: 1,
                        column: 1,
                        expected: Some("')'".to_string()),
                        found: None,
                    }));
                }
                self.advance();
                links.push(module_path);
            }

            if !links.is_empty() {
                if !self.at_end() && self.peek().typ == Type::Semicolon {
                    self.advance();
                }
                return Ok(Stmt::HttpBlock(HttpBlockStmt {
                    span: Span::new(start, start + 1),
                    prefix,
                    link: links.into_iter().next(),
                    cors: annotations.cors,
                    headers: annotations.headers,
                    routes: Vec::new(),
                }));
            }
        }

        if prefix.is_some() && (self.at_end() || self.peek().typ != Type::LBrace) {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected '{' or '.link' after $HTTP(path)".to_string(),
                line: 1,
                column: 1,
                expected: Some("'{' or '.link'".to_string()),
                found: None,
            }));
        }

        if self.at_end() || self.peek().typ != Type::LBrace {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected '{' after $HTTP".to_string(),
                line: 1,
                column: 1,
                expected: Some("'{'".to_string()),
                found: None,
            }));
        }
        self.advance();

        let mut tokens = Vec::new();
        let mut brace_depth = 1;
        while !self.at_end() {
            let tok = self.peek().clone();
            if tok.typ == Type::LBrace {
                brace_depth += 1;
            } else if tok.typ == Type::RBrace {
                brace_depth -= 1;
                if brace_depth == 0 {
                    self.advance();
                    break;
                }
            }
            tokens.push(tok);
            self.advance();
        }

        let mut routes = Vec::new();
        let mut pos = 0;
        while pos < tokens.len() {
            let tok = &tokens[pos];
            if tok.typ == Type::At {
                let annotation_name = tokens
                    .get(pos + 1)
                    .filter(|token| token.typ == Type::Ident)
                    .map(|token| token.literal.as_str());
                return Err(self.parse_unknown_annotation_error(tok.pos, annotation_name));
            }
            if matches!(
                tok.typ,
                Type::AtSetHdr
                    | Type::AtCors
                    | Type::HttpGet
                    | Type::HttpPost
                    | Type::HttpPut
                    | Type::HttpDel
                    | Type::HttpPatch
            ) {
                let mut sub_parser = StmtParser::new(&tokens[pos..], self.src);
                match sub_parser.parse_http_fn() {
                    Ok(Stmt::HttpFn(route)) => routes.push(route),
                    Ok(_) => {
                        pos += 1;
                        continue;
                    }
                    Err(e) => return Err(e),
                }
                pos += sub_parser.pos;
            } else {
                pos += 1;
            }
        }

        Ok(Stmt::HttpBlock(HttpBlockStmt {
            span: Span::new(start, start + 1),
            prefix,
            link: None,
            cors: annotations.cors,
            headers: annotations.headers,
            routes,
        }))
    }

    pub(crate) fn parse_http_annotations(&mut self) -> Result<HttpAnnotations, Error> {
        let mut annotations = HttpAnnotations::default();

        loop {
            if self.at_end() {
                break;
            }

            match self.peek().typ {
                Type::AtSetHdr => annotations.headers.extend(self.parse_set_hdr_annotation()?),
                Type::AtCors => {
                    let annotation_start = self.peek().pos;
                    if annotations.cors.is_some() {
                        return Err(self.error_at_pos(
                            annotation_start,
                            codes::PARSE_CORS_DUPLICATE,
                            "duplicate @CORS annotation",
                        ));
                    }
                    annotations.cors = Some(self.parse_cors_annotation()?);
                }
                _ => break,
            }
        }

        Ok(annotations)
    }

    fn parse_set_hdr_annotation(&mut self) -> Result<Vec<SetHdrEntry>, Error> {
        let annotation_start = self.peek().pos;
        self.advance();

        if self.at_end() || self.peek().typ != Type::LParen {
            return Err(self.error_at_current(
                codes::PARSE_SET_HDR_INVALID_SYNTAX,
                "expected '(' after @SET_HDR",
            ));
        }
        self.advance();

        if self.at_end() || self.peek().typ != Type::LBrace {
            return Err(self.error_at_current(
                codes::PARSE_SET_HDR_INVALID_SYNTAX,
                "@SET_HDR expects a config object like @SET_HDR({ \"X-Test\": \"1\" })",
            ));
        }
        self.advance();

        let mut headers = Vec::new();

        while !self.at_end() && self.peek().typ != Type::RBrace {
            if self.peek().typ != Type::String {
                return Err(self.error_at_current(
                    codes::PARSE_SET_HDR_INVALID_SYNTAX,
                    "expected string header name in @SET_HDR",
                ));
            }

            let header_name = self.advance().literal.clone();

            if self.at_end() || self.peek().typ != Type::Colon {
                return Err(self.error_at_current(
                    codes::PARSE_SET_HDR_INVALID_SYNTAX,
                    "expected ':' after @SET_HDR header name",
                ));
            }
            self.advance();

            if self.at_end() || self.peek().typ != Type::String {
                return Err(self.error_at_current(
                    codes::PARSE_SET_HDR_INVALID_SYNTAX,
                    format!("expected string header value for '{header_name}' in @SET_HDR"),
                ));
            }
            let header_value = self.advance().literal.clone();
            headers.push(SetHdrEntry {
                name: header_name,
                value: header_value,
            });

            if !self.at_end() && self.peek().typ == Type::Comma {
                self.advance();
            } else {
                break;
            }
        }

        if self.at_end() || self.peek().typ != Type::RBrace {
            return Err(self.error_at_current(
                codes::PARSE_SET_HDR_INVALID_SYNTAX,
                "expected '}' in @SET_HDR object",
            ));
        }
        self.advance();

        if self.at_end() || self.peek().typ != Type::RParen {
            return Err(self.error_at_current(
                codes::PARSE_SET_HDR_INVALID_SYNTAX,
                "expected ')' to close @SET_HDR",
            ));
        }
        self.advance();

        if headers.is_empty() {
            return Err(self.error_at_pos(
                annotation_start,
                codes::PARSE_SET_HDR_INVALID_SYNTAX,
                "@SET_HDR requires at least one header entry",
            ));
        }

        Ok(headers)
    }

    fn parse_cors_annotation(&mut self) -> Result<CorsConfig, Error> {
        self.advance();
        if self.at_end() || self.peek().typ != Type::LParen {
            return Err(
                self.error_at_current(codes::PARSE_CORS_INVALID_SYNTAX, "expected '(' after @CORS")
            );
        }
        self.advance();

        let cors = match self.peek().typ {
            Type::String if self.peek().literal == "*" => {
                self.advance();
                CorsConfig {
                    allow_all: true,
                    origins: Vec::new(),
                    methods: Vec::new(),
                    headers: Vec::new(),
                    max_age: None,
                    credentials: false,
                }
            }
            Type::LBrace => self.parse_cors_object()?,
            _ => {
                return Err(self.error_at_current(
                    codes::PARSE_CORS_INVALID_SYNTAX,
                    "expected \"*\" or a config object after @CORS(",
                ));
            }
        };

        if self.at_end() || self.peek().typ != Type::RParen {
            return Err(self.error_at_current(
                codes::PARSE_CORS_INVALID_SYNTAX,
                "expected ')' after @CORS arguments",
            ));
        }
        self.advance();

        Ok(cors)
    }

    fn parse_cors_object(&mut self) -> Result<CorsConfig, Error> {
        if self.at_end() || self.peek().typ != Type::LBrace {
            return Err(self.error_at_current(
                codes::PARSE_CORS_INVALID_SYNTAX,
                "expected '{' in @CORS object",
            ));
        }
        self.advance();

        let mut origins = Vec::new();
        let mut methods = Vec::new();
        let mut headers = Vec::new();
        let mut max_age = None;
        let mut credentials = false;
        let mut seen_fields = HashSet::new();

        while !self.at_end() && self.peek().typ != Type::RBrace {
            if self.peek().typ != Type::Ident {
                return Err(self.error_at_current(
                    codes::PARSE_CORS_INVALID_SYNTAX,
                    "expected @CORS field name",
                ));
            }
            let field_pos = self.peek().pos;
            let field_name = self.advance().literal.clone();

            if !seen_fields.insert(field_name.clone()) {
                return Err(self.error_at_pos(
                    field_pos,
                    codes::PARSE_CORS_INVALID_SYNTAX,
                    format!("duplicate @CORS field '{field_name}'"),
                ));
            }

            if self.at_end() || self.peek().typ != Type::Colon {
                return Err(self.error_at_current(
                    codes::PARSE_CORS_INVALID_SYNTAX,
                    format!("expected ':' after @CORS field '{field_name}'"),
                ));
            }
            self.advance();

            match field_name.as_str() {
                "origins" => origins = self.parse_cors_string_list()?,
                "methods" => methods = self.parse_cors_string_list()?,
                "headers" => headers = self.parse_cors_string_list()?,
                "max_age" => {
                    let mut sign = 1_i64;
                    if !self.at_end() && self.peek().typ == Type::Minus {
                        sign = -1;
                        self.advance();
                    }
                    if self.at_end() || self.peek().typ != Type::Number {
                        return Err(self.error_at_current(
                            codes::PARSE_CORS_INVALID_SYNTAX,
                            "expected integer number for @CORS max_age",
                        ));
                    }
                    let value_pos = self.peek().pos;
                    let literal = self.advance().literal.clone();
                    if literal.contains('.') {
                        return Err(self.error_at_pos(
                            value_pos,
                            codes::PARSE_CORS_INVALID_SYNTAX,
                            "expected integer for @CORS max_age",
                        ));
                    }
                    let parsed = literal.parse::<i64>().map_err(|_| {
                        self.error_at_pos(
                            value_pos,
                            codes::PARSE_CORS_INVALID_SYNTAX,
                            "expected valid i64 for @CORS max_age",
                        )
                    })?;
                    max_age = Some(sign * parsed);
                }
                "credentials" => {
                    if self.at_end() || self.peek().typ != Type::Bool {
                        return Err(self.error_at_current(
                            codes::PARSE_CORS_INVALID_SYNTAX,
                            "expected boolean for @CORS credentials",
                        ));
                    }
                    credentials = self.advance().literal == "true";
                }
                _ => {
                    return Err(self.error_at_pos(
                        field_pos,
                        codes::PARSE_CORS_INVALID_SYNTAX,
                        format!("unknown @CORS field '{field_name}'"),
                    ));
                }
            }

            if !self.at_end() && self.peek().typ == Type::Comma {
                self.advance();
            } else {
                break;
            }
        }

        if self.at_end() || self.peek().typ != Type::RBrace {
            return Err(self.error_at_current(
                codes::PARSE_CORS_INVALID_SYNTAX,
                "expected '}' in @CORS object",
            ));
        }
        self.advance();

        Ok(CorsConfig {
            allow_all: false,
            origins,
            methods,
            headers,
            max_age,
            credentials,
        })
    }

    fn parse_cors_string_list(&mut self) -> Result<Vec<String>, Error> {
        if self.at_end() || self.peek().typ != Type::LBracket {
            return Err(self.error_at_current(
                codes::PARSE_CORS_INVALID_SYNTAX,
                "expected '[' for @CORS string list",
            ));
        }
        self.advance();

        let mut values = Vec::new();
        while !self.at_end() && self.peek().typ != Type::RBracket {
            if self.peek().typ != Type::String {
                return Err(self.error_at_current(
                    codes::PARSE_CORS_INVALID_SYNTAX,
                    "expected string value in @CORS list",
                ));
            }
            values.push(self.advance().literal.clone());
            if !self.at_end() && self.peek().typ == Type::Comma {
                self.advance();
            } else {
                break;
            }
        }

        if self.at_end() || self.peek().typ != Type::RBracket {
            return Err(self.error_at_current(
                codes::PARSE_CORS_INVALID_SYNTAX,
                "expected ']' after @CORS list",
            ));
        }
        self.advance();

        Ok(values)
    }
}
