use crate::ast::{FnDeclStmt, FnParam, Span, Stmt};
use crate::error::Error;
use crate::token::Type;

use super::StmtParser;

impl<'a> StmtParser<'a> {
    pub fn parse_fn_decl(&mut self) -> Result<Stmt, Error> {
        let start = self.peek().pos;
        let is_public = self.peek().typ == Type::Fn;
        self.advance();

        if self.at_end() || self.peek().typ != Type::Ident {
            return Err(Error::InvalidStatement(None));
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

        Ok(Stmt::FnDecl(FnDeclStmt {
            span: Span::from_token(start),
            name,
            is_public,
            params,
            variadic_param,
            return_type,
            body,
        }))
    }
}
