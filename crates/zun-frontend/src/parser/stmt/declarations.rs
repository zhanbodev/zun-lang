use crate::ast::{
    BreakStmt, ConstDeclStmt, ContinueStmt, ExitStmt, MainDeclStmt, ModDeclStmt, ReturnStmt, Span,
    Stmt, TypeDeclStmt, TypeExpr, TypeField, VarDeclStmt,
};
use crate::error::Error;
use crate::token::Type;

use super::{StmtParser, build_compound_assign, find_compound_assign, find_token};

impl<'a> StmtParser<'a> {
    pub fn parse_return_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        if !self.at_end() && self.peek().typ == Type::Semicolon {
            self.skip_semis();
            return Ok(Stmt::Return(ReturnStmt {
                span: Span::from_token(start),
                value: None,
            }));
        }
        let expr_toks = self.collect_until_semi();
        if expr_toks.is_empty() {
            self.skip_semis();
            return Ok(Stmt::Return(ReturnStmt {
                span: Span::from_token(start),
                value: None,
            }));
        }
        let expr = self.parse_expr_tokens(&expr_toks)?;
        self.skip_semis();
        Ok(Stmt::Return(ReturnStmt {
            span: Span::from_token(start),
            value: Some(expr),
        }))
    }

    pub fn parse_mod_decl(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        let mut path_parts: Vec<String> = Vec::new();
        let mut wildcard = false;

        if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected module path after $mod".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("module path".to_string()),
            }));
        }
        path_parts.push(self.peek().literal.clone());
        self.advance();

        while !self.at_end() && self.peek().typ == Type::Dot {
            self.advance();
            if !self.at_end() && self.peek().typ == Type::Mul {
                wildcard = true;
                self.advance();
                break;
            }
            if self.at_end() || self.peek().typ != Type::Ident {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "expected identifier after '.' in module path".to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some("identifier".to_string()),
                }));
            }
            path_parts.push(self.peek().literal.clone());
            self.advance();
        }

        if wildcard && !self.at_end() && self.peek().typ == Type::Dot {
            return Err(Error::Parse(crate::error::ParseError {
                message: "wildcard module import must end with '*'".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("end of module path".to_string()),
            }));
        }

        self.skip_semis();
        Ok(Stmt::ModDecl(ModDeclStmt {
            span: Span::from_token(start),
            path: path_parts.join("."),
            wildcard,
        }))
    }

    pub fn parse_main_decl(&mut self) -> Result<Stmt, Error> {
        self.parse_main_decl_with_global_cors(None)
    }

    pub fn parse_main_decl_with_global_cors(
        &mut self,
        global_cors: Option<crate::ast::CorsConfig>,
    ) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        if self.at_end() || self.peek().typ != Type::LParen {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected '(' after $main".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("(".to_string()),
            }));
        }
        self.advance();
        if self.at_end() || self.peek().typ != Type::RParen {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected ')' in $main()".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some(")".to_string()),
            }));
        }
        self.advance();
        if self.at_end() || self.peek().typ != Type::LBrace {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected '{' after $main()".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("{".to_string()),
            }));
        }
        let body = self.parse_block()?;
        Ok(Stmt::MainDecl(MainDeclStmt {
            span: Span::from_token(start),
            global_cors,
            body,
        }))
    }

    pub fn parse_break_stmt(&mut self) -> Stmt {
        let start = self.peek().pos;
        self.advance();
        self.skip_semis();
        Stmt::Break(BreakStmt {
            span: Span::from_token(start),
        })
    }

    pub fn parse_continue_stmt(&mut self) -> Stmt {
        let start = self.peek().pos;
        self.advance();
        self.skip_semis();
        Stmt::Continue(ContinueStmt {
            span: Span::from_token(start),
        })
    }

    pub fn parse_exit_stmt(&mut self) -> Stmt {
        let start = self.peek().pos;
        self.advance();
        self.skip_semis();
        Stmt::Exit(ExitStmt {
            span: Span::from_token(start),
        })
    }

    pub fn parse_var_decl(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::InvalidStatement(None));
        }
        let name = self.advance().literal.clone();

        let type_annotation = if self.peek().typ == Type::Colon {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        if self.at_end() || self.peek().typ != Type::Assign {
            return Err(Error::InvalidStatement(None));
        }
        self.advance();
        let expr_toks = self.collect_until_semi();
        let expr = self.parse_expr_tokens(&expr_toks)?;
        self.skip_semis();
        Ok(Stmt::VarDecl(VarDeclStmt {
            span: Span::from_token(start),
            name,
            type_annotation,
            value: expr,
        }))
    }

    pub fn parse_const_decl(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::InvalidStatement(None));
        }
        let name = self.advance().literal.clone();

        let type_annotation = if self.peek().typ == Type::Colon {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        if self.at_end() || self.peek().typ != Type::Assign {
            return Err(Error::InvalidStatement(None));
        }
        self.advance();
        let expr_toks = self.collect_until_semi();
        let expr = self.parse_expr_tokens(&expr_toks)?;
        self.skip_semis();
        Ok(Stmt::ConstDecl(ConstDeclStmt {
            span: Span::from_token(start),
            name,
            type_annotation,
            value: expr,
        }))
    }

    pub fn parse_type_decl(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance(); // consume $Type

        if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected type name after $Type".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("identifier".to_string()),
            }));
        }
        let name = self.advance().literal.clone();

        if self.at_end() || self.peek().typ != Type::LBrace {
            return Err(Error::Parse(crate::error::ParseError {
                message: format!("expected '{{' after $Type {}", name),
                line: 1,
                column: 1,
                found: None,
                expected: Some("{".to_string()),
            }));
        }
        self.advance(); // consume {

        let mut fields = Vec::new();

        // skip leading newlines / semicolons
        self.skip_semis();

        while !self.at_end() && self.peek().typ != Type::RBrace {
            // optional @HIDE annotation
            let hidden = if !self.at_end() && self.peek().typ == Type::AtHide {
                self.advance();
                true
            } else {
                false
            };

            // field name
            if self.peek().typ != Type::Ident {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "expected field name in $Type body".to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some("identifier or @HIDE annotation".to_string()),
                }));
            }
            let field_name = self.advance().literal.clone();

            if field_name.starts_with('_') {
                return Err(Error::Parse(crate::error::ParseError {
                    message: format!(
                        "field name '{}' uses '_' prefix; use '@HIDE {}' instead",
                        field_name,
                        field_name.trim_start_matches('_')
                    ),
                    line: 1,
                    column: 1,
                    found: Some(field_name),
                    expected: Some("@HIDE annotation or plain field name".to_string()),
                }));
            }

            // colon
            if self.at_end() || self.peek().typ != Type::Colon {
                return Err(Error::Parse(crate::error::ParseError {
                    message: format!("expected ':' after field name '{}'", field_name),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some(":".to_string()),
                }));
            }
            self.advance(); // consume :

            let type_expr = self.parse_type_expr()?;

            fields.push(TypeField {
                name: field_name,
                type_expr,
                hidden,
            });

            // allow comma or semicolon between fields (optional)
            while !self.at_end()
                && (self.peek().typ == Type::Comma || self.peek().typ == Type::Semicolon)
            {
                self.advance();
            }
        }

        if self.at_end() || self.peek().typ != Type::RBrace {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected '}' to close $Type body".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("}".to_string()),
            }));
        }
        self.advance(); // consume }
        self.skip_semis();

        Ok(Stmt::TypeDecl(TypeDeclStmt {
            span: Span::from_token(start),
            name,
            fields,
        }))
    }

    pub(crate) fn parse_type_expr(&mut self) -> Result<TypeExpr, Error> {
        let mut base = if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::Parse(crate::error::ParseError {
                message: "expected type name".to_string(),
                line: 1,
                column: 1,
                found: None,
                expected: Some("type name".to_string()),
            }));
        } else if self.peek().literal == "List" {
            self.advance();
            if self.at_end() || self.peek().typ != Type::Lt {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "bare 'List' is not allowed here; use 'List<T>'".to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some("<".to_string()),
                }));
            }
            self.advance();
            let inner = self.parse_type_expr()?;
            if matches!(inner, TypeExpr::Optional(_)) {
                return Err(Error::Parse(crate::error::ParseError {
                    message:
                        "optional list item types are not supported; use 'List<T>' or 'List<T>?'"
                            .to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some("non-optional list item type".to_string()),
                }));
            }
            if self.at_end() || self.peek().typ != Type::Gt {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "expected '>' after list item type".to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some(">".to_string()),
                }));
            }
            self.advance();
            TypeExpr::List(Box::new(inner))
        } else {
            TypeExpr::Named(self.advance().literal.clone())
        };

        if !self.at_end() && self.peek().typ == Type::Question {
            self.advance();
            if !self.at_end() && self.peek().typ == Type::Question {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "double optional marker '??' is not supported".to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some("single '?'".to_string()),
                }));
            }
            base = TypeExpr::Optional(Box::new(base));
        }

        Ok(base)
    }

    pub fn parse_expr_or_assignment_stmt(&mut self) -> Result<Option<Stmt>, Error> {
        if self.peek().typ == Type::Ident
            && self.pos + 1 < self.tokens.len()
            && self.tokens[self.pos + 1].typ == Type::LParen
        {
            let saved_pos = self.pos;
            let stmt_toks = self.collect_until_semi();
            self.skip_semis();
            match self.parse_expr_tokens(&stmt_toks) {
                Ok(expr) => return Ok(Some(Stmt::ExprStmt(expr))),
                Err(_) => {
                    self.pos = saved_pos;
                }
            }
        }

        let stmt_toks = self.collect_until_semi();
        self.skip_semis();

        if let Some((assign_idx, op)) = find_compound_assign(&stmt_toks) {
            return build_compound_assign(&stmt_toks, assign_idx, op).map(Some);
        }

        if let Some(assign_idx) = find_token(&stmt_toks, Type::Assign) {
            let start = stmt_toks.first().map(|t| t.pos).unwrap_or(0);
            let name_expr = self.parse_expr_tokens(&stmt_toks[..assign_idx])?;
            let value_expr = self.parse_expr_tokens(&stmt_toks[assign_idx + 1..])?;
            return Ok(Some(Stmt::Assign(crate::ast::AssignStmt {
                span: Span::from_token(start),
                name: name_expr,
                value: value_expr,
            })));
        }

        if !stmt_toks.is_empty() {
            return self
                .parse_expr_tokens(&stmt_toks)
                .map(Stmt::ExprStmt)
                .map(Some);
        }

        Err(Error::InvalidStatement(None))
    }
}
