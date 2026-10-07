use crate::ast::{BinaryExpr, Expr, IndexAccess, MethodCall, Span, Spanned, UnaryExpr};
use crate::error::Error;
use crate::token::Type;

use super::ExprParser;

impl<'a> ExprParser<'a> {
    pub fn parse_expr(&mut self) -> Result<Box<Expr>, Error> {
        self.parse_or_expr()
    }

    pub fn parse_or_expr(&mut self) -> Result<Box<Expr>, Error> {
        let mut left = self.parse_and_expr()?;

        while self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::Or {
            let op_pos = self.tokens[self.pos].pos;
            self.pos += 1;
            let right = self.parse_and_expr()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Box::new(Expr::Binary(BinaryExpr {
                span: Span::new(op_pos, span.end),
                left,
                right,
                op: Type::Or,
            }));
        }
        Ok(left)
    }

    pub fn parse_and_expr(&mut self) -> Result<Box<Expr>, Error> {
        let mut left = self.parse_equality_expr()?;

        while self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::And {
            let op_pos = self.tokens[self.pos].pos;
            self.pos += 1;
            let right = self.parse_equality_expr()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Box::new(Expr::Binary(BinaryExpr {
                span: Span::new(op_pos, span.end),
                left,
                right,
                op: Type::And,
            }));
        }
        Ok(left)
    }

    pub fn parse_equality_expr(&mut self) -> Result<Box<Expr>, Error> {
        let mut left = self.parse_comparison_expr()?;

        while self.pos < self.tokens.len() {
            let typ = &self.tokens[self.pos].typ;
            if *typ == Type::Eq || *typ == Type::Ne {
                self.pos += 1;
                let right = self.parse_comparison_expr()?;
                let span = Span::new(left.span().start, right.span().end);
                left = Box::new(Expr::Binary(BinaryExpr {
                    span,
                    left,
                    right,
                    op: typ.clone(),
                }));
            } else {
                break;
            }
        }
        Ok(left)
    }

    pub fn parse_comparison_expr(&mut self) -> Result<Box<Expr>, Error> {
        let mut left = self.parse_additive_expr()?;

        while self.pos < self.tokens.len() {
            let typ = &self.tokens[self.pos].typ;
            if *typ == Type::Gt || *typ == Type::Lt || *typ == Type::Gte || *typ == Type::Lte {
                self.pos += 1;
                let right = self.parse_additive_expr()?;
                let span = Span::new(left.span().start, right.span().end);
                left = Box::new(Expr::Binary(BinaryExpr {
                    span,
                    left,
                    right,
                    op: typ.clone(),
                }));
            } else {
                break;
            }
        }
        Ok(left)
    }

    pub fn parse_additive_expr(&mut self) -> Result<Box<Expr>, Error> {
        let mut left = self.parse_multiplicative_expr()?;

        while self.pos < self.tokens.len() {
            let typ = &self.tokens[self.pos].typ;
            if *typ == Type::Plus || *typ == Type::Minus {
                self.pos += 1;
                let right = self.parse_multiplicative_expr()?;
                let span = Span::new(left.span().start, right.span().end);
                left = Box::new(Expr::Binary(BinaryExpr {
                    span,
                    left,
                    right,
                    op: typ.clone(),
                }));
            } else {
                break;
            }
        }
        Ok(left)
    }

    pub fn parse_multiplicative_expr(&mut self) -> Result<Box<Expr>, Error> {
        let mut left = self.parse_unary_expr()?;

        while self.pos < self.tokens.len() {
            let typ = &self.tokens[self.pos].typ;
            if *typ == Type::Mul || *typ == Type::Div || *typ == Type::Mod {
                self.pos += 1;
                let right = self.parse_unary_expr()?;
                let span = Span::new(left.span().start, right.span().end);
                left = Box::new(Expr::Binary(BinaryExpr {
                    span,
                    left,
                    right,
                    op: typ.clone(),
                }));
            } else {
                break;
            }
        }
        Ok(left)
    }

    pub fn parse_unary_expr(&mut self) -> Result<Box<Expr>, Error> {
        if self.pos < self.tokens.len() {
            let typ = &self.tokens[self.pos].typ;
            if *typ == Type::Not || *typ == Type::Minus {
                let op_pos = self.tokens[self.pos].pos;
                self.pos += 1;
                let right = self.parse_unary_expr()?;
                let span = Span::new(op_pos, right.span().end);
                return Ok(Box::new(Expr::Unary(UnaryExpr {
                    span,
                    op: typ.clone(),
                    right,
                })));
            }
        }

        let mut expr = self.parse_primary()?;

        loop {
            if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::LBracket {
                let start = self.tokens[self.pos].pos;
                self.pos += 1;

                let index = self.parse_expr()?;

                if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RBracket {
                    return Err(
                        self.error_expected("expected closing bracket in index access", "]")
                    );
                }
                self.pos += 1;

                expr = Box::new(Expr::IndexAccess(IndexAccess {
                    span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
                    object: expr,
                    index,
                }));
                continue;
            }

            if self.pos < self.tokens.len() && self.tokens[self.pos].typ == Type::Dot {
                let start = self.tokens[self.pos].pos;
                self.pos += 1;

                if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Ident {
                    return Err(self.error_expected("expected method name after '.'", "identifier"));
                }
                let method = self.tokens[self.pos].literal.clone();
                self.pos += 1;

                if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::LParen {
                    // Allow zero-arg method / field access when:
                    // 1. chaining: next token is '.' (e.g. `services.auth.login()`)
                    // 2. end of token slice (e.g. `user.password` on LHS of assignment
                    //    or in `$>> user.password;` after token collection strips ';')
                    let next_is_terminator = self.pos >= self.tokens.len()
                        || matches!(
                            self.tokens[self.pos].typ,
                            Type::Dot
                                | Type::Semicolon
                                | Type::Comma
                                | Type::RParen
                                | Type::RBracket
                        );
                    if next_is_terminator {
                        expr = Box::new(Expr::MethodCall(MethodCall {
                            span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
                            object: expr,
                            method,
                            args: vec![],
                        }));
                        continue;
                    }
                    return Err(self.error_expected(
                        &format!(
                            "method '{}' requires parentheses, use '{}(...)' instead",
                            method, method
                        ),
                        "(",
                    ));
                }

                let mut args: Vec<Expr> = Vec::new();
                self.pos += 1;

                if self.pos < self.tokens.len() && self.tokens[self.pos].typ != Type::RParen {
                    loop {
                        let arg = self.parse_expr()?;
                        args.push(*arg);
                        if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::Comma
                        {
                            break;
                        }
                        self.pos += 1;
                    }
                }

                if self.pos >= self.tokens.len() || self.tokens[self.pos].typ != Type::RParen {
                    return Err(
                        self.error_expected("expected closing parenthesis in method call", ")")
                    );
                }
                self.pos += 1;

                expr = Box::new(Expr::MethodCall(MethodCall {
                    span: Span::new(start, self.tokens[self.pos - 1].pos + 1),
                    object: expr,
                    method,
                    args,
                }));
                continue;
            }

            break;
        }

        Ok(expr)
    }
}
