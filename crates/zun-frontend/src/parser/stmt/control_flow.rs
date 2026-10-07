use crate::ast::{
    ForInStmt, ForStmt, IfBranch, IfStmt, LoopStmt, Span, Stmt, ThrowStmt, TryStmt, WhileStmt,
};
use crate::error::Error;
use crate::token::{Token, Type};

use super::{StmtParser, parse_init_or_update, split_by_semi};

impl<'a> StmtParser<'a> {
    /// Parse a full $if ... { } [$elif ... { }]* [$else { }] statement.
    pub fn parse_if_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        let mut branches: Vec<IfBranch> = Vec::new();
        let mut else_body: Vec<Stmt> = Vec::new();

        self.advance();
        let cond_toks = self.collect_until_lbrace();
        if cond_toks.is_empty() {
            return Err(Error::InvalidExpression(None));
        }
        let cond = self.parse_expr_tokens(&cond_toks)?;
        let body = self.parse_block()?;
        branches.push(IfBranch {
            span: Span::from_token(0),
            condition: cond,
            body,
        });

        loop {
            self.skip_semis();
            if !self.at_end() && self.peek().typ == Type::Elif {
                self.advance();
                let cond_toks = self.collect_until_lbrace();
                if cond_toks.is_empty() {
                    return Err(Error::InvalidExpression(None));
                }
                let cond = self.parse_expr_tokens(&cond_toks)?;
                let body = self.parse_block()?;
                branches.push(IfBranch {
                    span: Span::from_token(0),
                    condition: cond,
                    body,
                });
            } else {
                break;
            }
        }

        self.skip_semis();
        if !self.at_end() && self.peek().typ == Type::Else {
            self.advance();
            else_body = self.parse_block()?;
        }

        Ok(Stmt::If(IfStmt {
            span: Span::from_token(start),
            branches,
            else_body,
        }))
    }

    /// Collect tokens up to the next `{` (not consuming it).
    pub fn collect_until_lbrace(&mut self) -> Vec<Token> {
        let mut toks = Vec::new();
        while !self.at_end() && self.peek().typ != Type::LBrace && self.peek().typ != Type::Eof {
            toks.push(self.advance().clone());
        }
        toks
    }

    pub fn parse_while_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        let cond_toks = self.collect_until_lbrace();
        if cond_toks.is_empty() {
            return Err(Error::InvalidExpression(None));
        }
        let condition = self.parse_expr_tokens(&cond_toks)?;
        let body = self.parse_block()?;
        Ok(Stmt::While(WhileStmt {
            span: Span::from_token(start),
            condition,
            body,
        }))
    }

    pub fn parse_loop_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        let body = self.parse_block()?;
        Ok(Stmt::Loop(LoopStmt {
            span: Span::from_token(start),
            body,
        }))
    }

    pub fn parse_for_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance();
        let header_toks = self.collect_until_lbrace();
        let has_in = header_toks.iter().any(|t| t.typ == Type::In);

        if has_in {
            let parts: Vec<&[Token]> = header_toks.split(|t| t.typ == Type::In).collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "invalid for-in syntax, expected: $for item in iterable { ... }"
                        .to_string(),
                    line: 1,
                    column: 1,
                    found: None,
                    expected: Some("variable and iterable".to_string()),
                }));
            }

            let var_toks = parts[0];
            if var_toks.len() != 1 || var_toks[0].typ != Type::Ident {
                return Err(Error::Parse(crate::error::ParseError {
                    message: "expected loop variable name".to_string(),
                    line: 1,
                    column: 1,
                    found: Some(format!("{:?}", var_toks.first().map(|t| &t.typ))),
                    expected: Some("identifier".to_string()),
                }));
            }

            let var = var_toks[0].literal.clone();
            let iterable = self.parse_expr_tokens(parts[1])?;
            let body = self.parse_block()?;

            return Ok(Stmt::ForIn(ForInStmt {
                span: Span::from_token(start),
                var,
                iterable,
                body,
            }));
        }

        let parts = split_by_semi(&header_toks);
        let (init, condition, update) = match parts.len() {
            0 => (None, None, None),
            1 => {
                let cond = if parts[0].is_empty() {
                    None
                } else {
                    Some(self.parse_expr_tokens(&parts[0])?)
                };
                (None, cond, None)
            }
            2 => {
                let init = parse_init_or_update(&parts[0])?;
                let cond = if parts[1].is_empty() {
                    None
                } else {
                    Some(self.parse_expr_tokens(&parts[1])?)
                };
                (init, cond, None)
            }
            _ => {
                let init = parse_init_or_update(&parts[0])?;
                let cond = if parts[1].is_empty() {
                    None
                } else {
                    Some(self.parse_expr_tokens(&parts[1])?)
                };
                let update = parse_init_or_update(&parts[2])?;
                (init, cond, update)
            }
        };

        let body = self.parse_block()?;
        Ok(Stmt::For(ForStmt {
            span: Span::from_token(start),
            init,
            condition,
            update,
            body,
        }))
    }

    /// $try { body } $catch ident { catch_body }
    pub fn parse_try_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance(); // consume $try
        let body = self.parse_block()?;
        self.skip_semis();
        self.expect(Type::Catch)?;
        let catch_var = {
            let tok = self.expect(Type::Ident)?;
            tok.literal.clone()
        };
        let catch_body = self.parse_block()?;
        Ok(Stmt::Try(TryStmt {
            span: Span::from_token(start),
            body,
            catch_var,
            catch_body,
        }))
    }

    /// $throw expr;
    pub fn parse_throw_stmt(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        self.advance(); // consume $throw
        let toks = self.collect_until_semi();
        if toks.is_empty() {
            return Err(Error::InvalidExpression(None));
        }
        let value = self.parse_expr_tokens(&toks)?;
        Ok(Stmt::Throw(ThrowStmt {
            span: Span::from_token(start),
            value,
        }))
    }
}
