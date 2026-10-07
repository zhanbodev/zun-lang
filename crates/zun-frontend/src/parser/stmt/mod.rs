mod control_flow;
mod declarations;
mod functions;
mod http;
mod io;
mod modules;

use crate::ast::{AssignStmt, BinaryExpr, Expr, Span, Stmt, TypeExpr, VarDeclStmt};
use crate::diagnostics::{Diagnostic, codes};
use crate::error::Error;
use crate::parser::{calc_line_col, parse_expr_tokens, parse_expr_tokens_with_src};
use crate::token::{Token, Type};
use http::HttpAnnotations;

/// Top-level statement parser — token-stream based, supports blocks.
pub struct StmtParser<'a> {
    pub tokens: &'a [Token],
    pub pos: usize,
    #[allow(dead_code)]
    pub src: &'a str,
}

impl<'a> StmtParser<'a> {
    pub fn new(tokens: &'a [Token], src: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            src,
        }
    }

    pub fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub fn peek_next(&self) -> Option<&Token> {
        if self.pos + 1 < self.tokens.len() {
            Some(&self.tokens[self.pos + 1])
        } else {
            None
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].typ == Type::Eof
    }

    pub fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    pub fn skip_semis(&mut self) {
        while !self.at_end() && self.peek().typ == Type::Semicolon {
            self.pos += 1;
        }
    }

    /// Expect a token of the given type, consume it, or return an error.
    pub fn expect(&mut self, typ: Type) -> Result<&Token, Error> {
        if self.at_end() || self.peek().typ != typ {
            return Err(self.error_expected(format!("expected token '{typ}'"), typ.to_string()));
        }
        Ok(self.advance())
    }

    pub(crate) fn error_at_current(&self, code: &'static str, message: impl Into<String>) -> Error {
        let pos = if self.at_end() {
            self.tokens.last().map(|token| token.pos).unwrap_or(0)
        } else {
            self.peek().pos
        };
        self.error_at_pos(pos, code, message)
    }

    pub(crate) fn error_at_pos(
        &self,
        pos: usize,
        code: &'static str,
        message: impl Into<String>,
    ) -> Error {
        let (line, column) = calc_line_col(self.src, pos);
        Error::Diagnostic(
            Diagnostic::error(code, message)
                .with_location(line, column)
                .with_span(crate::ast::Span::from_token(pos)),
        )
    }

    pub(crate) fn parse_expr_tokens(&self, toks: &[Token]) -> Result<Box<Expr>, Error> {
        parse_expr_tokens_with_src(toks, self.src)
    }

    pub(crate) fn error_expected(&self, message: impl Into<String>, expected: String) -> Error {
        let pos = if self.at_end() {
            self.tokens.last().map(|token| token.pos).unwrap_or(0)
        } else {
            self.peek().pos
        };
        let (line, column) = calc_line_col(self.src, pos);
        let found = if self.at_end() {
            "<eof>".to_string()
        } else {
            format!("{:?} {}", self.peek().typ, self.peek().literal)
        };
        Error::Diagnostic(
            Diagnostic::error(codes::PARSE_EXPECTED_TOKEN, message)
                .with_location(line, column)
                .with_note(format!("expected: {expected}; found: {found}")),
        )
    }

    /// Parse a sequence of statements until EOF (or a closing `}`).
    pub fn parse_stmts(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut stmts = Vec::new();
        loop {
            self.skip_semis();
            if self.at_end() || self.peek().typ == Type::RBrace {
                break;
            }
            let stmt = self.parse_one_stmt()?;
            if let Some(s) = stmt {
                stmts.push(s);
            }
        }
        Ok(stmts)
    }

    /// Parse a `{ ... }` block and return the contained statements.
    pub fn parse_block(&mut self) -> Result<Vec<Stmt>, Error> {
        self.expect(Type::LBrace)?;
        let stmts = self.parse_stmts()?;
        self.expect(Type::RBrace)?;
        Ok(stmts)
    }

    /// Collect tokens up to the next `;` or `}` (not inside braces).
    pub fn collect_until_semi(&mut self) -> Vec<Token> {
        let mut toks = Vec::new();
        let mut brace_depth = 0;
        while !self.at_end() {
            let typ = self.peek().typ.clone();
            if brace_depth == 0
                && (typ == Type::Semicolon || typ == Type::RBrace || typ == Type::Eof)
            {
                break;
            }
            if typ == Type::LBrace {
                brace_depth += 1;
            } else if typ == Type::RBrace {
                brace_depth -= 1;
            }
            toks.push(self.advance().clone());
        }
        toks
    }

    /// Parse a single top-level statement.
    pub fn parse_one_stmt(&mut self) -> Result<Option<Stmt>, Error> {
        let typ = self.peek().typ.clone();

        if typ == Type::If {
            return self.parse_if_stmt().map(Some);
        }
        if typ == Type::While {
            return self.parse_while_stmt().map(Some);
        }
        if typ == Type::Loop {
            return self.parse_loop_stmt().map(Some);
        }
        if typ == Type::For {
            return self.parse_for_stmt().map(Some);
        }
        if matches!(typ, Type::Fn | Type::PrivateFn)
            && let Some(next_pos) = self.peek_next()
            && next_pos.typ == Type::Ident
        {
            return self.parse_fn_decl().map(Some);
        }
        if matches!(
            typ,
            Type::HttpGet | Type::HttpPost | Type::HttpPut | Type::HttpDel | Type::HttpPatch
        ) {
            return self.parse_http_fn().map(Some);
        }
        if matches!(typ, Type::AtSetHdr | Type::AtCors) {
            let annotations = self.parse_http_annotations()?;
            if self.at_end() {
                return Err(self.error_for_annotation_position(&annotations));
            }

            return match self.peek().typ {
                Type::HttpGet
                | Type::HttpPost
                | Type::HttpPut
                | Type::HttpDel
                | Type::HttpPatch => self.parse_http_fn_with_annotations(annotations).map(Some),
                Type::HttpBlock => self
                    .parse_http_block_with_annotations(annotations)
                    .map(Some),
                Type::MainDecl if annotations.headers.is_empty() => self
                    .parse_main_decl_with_global_cors(annotations.cors)
                    .map(Some),
                _ => Err(self.error_for_annotation_position(&annotations)),
            };
        }
        if typ == Type::At {
            return Err(self.parse_unknown_annotation_error(
                self.peek().pos,
                self.peek_next()
                    .filter(|token| token.typ == Type::Ident)
                    .map(|token| token.literal.as_str()),
            ));
        }
        if typ == Type::HttpBlock {
            return self.parse_http_block().map(Some);
        }
        if typ == Type::Static {
            return self.parse_static_decl().map(Some);
        }
        if typ == Type::Try {
            return self.parse_try_stmt().map(Some);
        }
        if typ == Type::Throw {
            return self.parse_throw_stmt().map(Some);
        }
        if typ == Type::Return {
            return self.parse_return_stmt().map(Some);
        }
        if typ == Type::ModDecl {
            return self.parse_mod_decl().map(Some);
        }
        if typ == Type::MainDecl {
            return self.parse_main_decl().map(Some);
        }
        if typ == Type::TypeDecl {
            return self.parse_type_decl().map(Some);
        }
        if typ == Type::Break {
            return Ok(Some(self.parse_break_stmt()));
        }
        if typ == Type::Continue {
            return Ok(Some(self.parse_continue_stmt()));
        }
        if typ == Type::Ident && matches!(self.peek().literal.as_str(), "exit" | "exit()" | "quit")
        {
            return Ok(Some(self.parse_exit_stmt()));
        }
        if typ == Type::Print {
            return self.parse_print_stmt();
        }
        if typ == Type::Read {
            return self.parse_read_stmt();
        }
        if typ == Type::VarDecl {
            return self.parse_var_decl().map(Some);
        }
        if typ == Type::ConstDecl {
            return self.parse_const_decl().map(Some);
        }

        self.parse_expr_or_assignment_stmt()
    }

    pub(crate) fn parse_unknown_annotation_error(
        &self,
        at_pos: usize,
        annotation_name: Option<&str>,
    ) -> Error {
        let annotation_name = annotation_name
            .map(|name| format!("@{name}"))
            .unwrap_or_else(|| "@".to_string());
        self.error_at_pos(
            at_pos,
            codes::PARSE_SET_HDR_INVALID_SYNTAX,
            format!("unknown annotation '{annotation_name}'"),
        )
    }

    pub(crate) fn error_for_annotation_position(&self, annotations: &HttpAnnotations) -> Error {
        if !annotations.headers.is_empty() {
            self.error_at_current(
                codes::PARSE_SET_HDR_INVALID_POSITION,
                "@SET_HDR annotation must appear immediately before an HTTP route or HTTP block",
            )
        } else {
            self.error_at_current(
                codes::PARSE_CORS_INVALID_POSITION,
                "@CORS annotation is only allowed before $main(), $HTTP blocks, or HTTP routes",
            )
        }
    }
}

/// Find a token of a specific type in a slice.
fn find_token(toks: &[Token], t: Type) -> Option<usize> {
    toks.iter().position(|tok| tok.typ == t)
}

/// Find a compound assignment token and return its index and the corresponding binary operator.
fn find_compound_assign(toks: &[Token]) -> Option<(usize, Type)> {
    for (i, tok) in toks.iter().enumerate() {
        let binary_op = match tok.typ {
            Type::PlusAssign => Some(Type::Plus),
            Type::MinusAssign => Some(Type::Minus),
            Type::MulAssign => Some(Type::Mul),
            Type::DivAssign => Some(Type::Div),
            Type::ModAssign => Some(Type::Mod),
            _ => None,
        };
        if let Some(op) = binary_op {
            return Some((i, op));
        }
    }
    None
}

/// Split a token slice by Semicolon into parts.
fn split_by_semi(toks: &[Token]) -> Vec<Vec<Token>> {
    let mut parts: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    for tok in toks {
        if tok.typ == Type::Semicolon {
            parts.push(current.clone());
            current.clear();
        } else {
            current.push(tok.clone());
        }
    }
    parts.push(current);
    parts
}

/// Parse a token slice as a single init/update statement.
fn parse_init_or_update(toks: &[Token]) -> Result<Option<Box<Stmt>>, Error> {
    if toks.is_empty() {
        return Ok(None);
    }

    if toks[0].typ == Type::VarDecl {
        if toks.len() < 4 || toks[1].typ != Type::Ident {
            return Err(Error::InvalidStatement(None));
        }
        let start = toks[0].pos;
        let name = toks[1].literal.clone();

        let type_annotation;
        let value_start_idx;
        if toks[2].typ == Type::Colon {
            let assign_idx = find_token(toks, Type::Assign).ok_or(Error::InvalidStatement(None))?;
            if assign_idx <= 3 {
                return Err(Error::InvalidStatement(None));
            }
            type_annotation = Some(parse_type_expr_tokens(&toks[3..assign_idx])?);
            value_start_idx = assign_idx + 1;
        } else if toks[2].typ == Type::Assign {
            type_annotation = None;
            value_start_idx = 3;
        } else {
            return Err(Error::InvalidStatement(None));
        }

        let expr = parse_expr_tokens(&toks[value_start_idx..])?;
        return Ok(Some(Box::new(Stmt::VarDecl(VarDeclStmt {
            span: Span::from_token(start),
            name,
            type_annotation,
            value: expr,
        }))));
    }

    if let Some(idx) = find_token(toks, Type::Assign) {
        let start = toks.first().map(|t| t.pos).unwrap_or(0);
        let name_expr = parse_expr_tokens(&toks[..idx])?;
        let value_expr = parse_expr_tokens(&toks[idx + 1..])?;
        return Ok(Some(Box::new(Stmt::Assign(AssignStmt {
            span: Span::from_token(start),
            name: name_expr,
            value: value_expr,
        }))));
    }

    Err(Error::InvalidStatement(None))
}

pub(crate) fn parse_type_expr_tokens(toks: &[Token]) -> Result<TypeExpr, Error> {
    let mut parser = StmtParser::new(toks, "");
    let type_expr = parser.parse_type_expr()?;
    if !parser.at_end() {
        return Err(Error::InvalidStatement(None));
    }
    Ok(type_expr)
}

pub(crate) fn build_compound_assign(
    stmt_toks: &[Token],
    assign_idx: usize,
    op: Type,
) -> Result<Stmt, Error> {
    let start = stmt_toks.first().map(|t| t.pos).unwrap_or(0);
    let name_expr = parse_expr_tokens(&stmt_toks[..assign_idx])?;
    let value_expr = parse_expr_tokens(&stmt_toks[assign_idx + 1..])?;
    let binary_expr = BinaryExpr {
        span: Span::from_token(start),
        left: name_expr.clone(),
        right: value_expr,
        op,
    };
    Ok(Stmt::Assign(AssignStmt {
        span: Span::from_token(start),
        name: name_expr,
        value: Box::new(Expr::Binary(binary_expr)),
    }))
}
