// Literal parser - handles function literals and anonymous functions.
use crate::ast::{Expr, FnLiteral, FnParam, Span, Stmt};
use crate::error::Error;
use crate::parser::expr::ExprParser;
use crate::parser::stmt::parse_type_expr_tokens;
use crate::token::Type;

/// Parse anonymous function literal: $fn(x, y) -> Int { body }
pub fn parse_fn_literal<'a>(parser: &mut ExprParser<'a>) -> Result<Box<Expr>, Error> {
    let start = parser.tokens.get(parser.pos).map(|t| t.pos).unwrap_or(0);
    // Parse parameters: (param1, param2, ...)
    if parser.pos >= parser.tokens.len() || parser.tokens[parser.pos].typ != Type::LParen {
        return Err(parser.error_expected("expected function parameters", "("));
    }
    parser.pos += 1; // consume '('

    let mut params: Vec<FnParam> = Vec::new();
    let mut variadic_param: Option<String> = None;
    if parser.pos < parser.tokens.len() && parser.tokens[parser.pos].typ != Type::RParen {
        loop {
            // Check for variadic parameter: ...identifier
            if parser.tokens[parser.pos].typ == Type::Spread {
                parser.pos += 1; // consume '...'
                if parser.pos >= parser.tokens.len() || parser.tokens[parser.pos].typ != Type::Ident
                {
                    return Err(
                        parser.error_expected("expected variadic parameter name", "identifier")
                    );
                }
                variadic_param = Some(parser.tokens[parser.pos].literal.clone());
                parser.pos += 1;
                // Variadic must be the last parameter
                break;
            }
            if parser.pos >= parser.tokens.len() || parser.tokens[parser.pos].typ != Type::Ident {
                return Err(parser.error_expected("expected parameter name", "identifier"));
            }
            let name = parser.tokens[parser.pos].literal.clone();
            parser.pos += 1;
            let type_annotation = if parser.pos < parser.tokens.len()
                && parser.tokens[parser.pos].typ == Type::Colon
            {
                parser.pos += 1;
                let type_start = parser.pos;
                while parser.pos < parser.tokens.len()
                    && parser.tokens[parser.pos].typ != Type::Comma
                    && parser.tokens[parser.pos].typ != Type::RParen
                {
                    parser.pos += 1;
                }
                Some(parse_type_expr_tokens(
                    &parser.tokens[type_start..parser.pos],
                )?)
            } else {
                None
            };
            params.push(FnParam {
                name,
                type_annotation,
            });
            if parser.pos >= parser.tokens.len() || parser.tokens[parser.pos].typ != Type::Comma {
                break;
            }
            parser.pos += 1; // consume ','
        }
    }
    if parser.pos >= parser.tokens.len() || parser.tokens[parser.pos].typ != Type::RParen {
        return Err(parser.error_expected("expected closing parenthesis", ")"));
    }
    parser.pos += 1; // consume ')'

    // Parse optional return type: -> Type
    let return_type = if parser.pos < parser.tokens.len()
        && parser.tokens[parser.pos].typ == Type::Arrow
    {
        parser.pos += 1; // consume '->'
        let type_start = parser.pos;
        while parser.pos < parser.tokens.len() && parser.tokens[parser.pos].typ != Type::LBrace {
            parser.pos += 1;
        }
        if type_start == parser.pos {
            return Err(parser.error_expected("expected return type", "type expression"));
        }
        Some(parse_type_expr_tokens(
            &parser.tokens[type_start..parser.pos],
        )?)
    } else {
        None
    };

    // Parse body: { ... } - collect tokens inside the braces
    if parser.pos >= parser.tokens.len() || parser.tokens[parser.pos].typ != Type::LBrace {
        return Err(parser.error_expected("expected function body", "{"));
    }

    // Find matching closing brace
    parser.pos += 1; // consume '{'
    let start_pos = parser.pos;
    let mut brace_depth = 1;

    while parser.pos < parser.tokens.len() && brace_depth > 0 {
        match parser.tokens[parser.pos].typ {
            Type::LBrace => brace_depth += 1,
            Type::RBrace => brace_depth -= 1,
            _ => {}
        }
        parser.pos += 1;
    }

    let end_pos = parser.pos - 1; // back up to position before '}'
    // Extract body tokens (without braces)
    let body_tokens = &parser.tokens[start_pos..end_pos];
    // For function body, we need to parse each statement separately
    // Split by semicolons and parse each
    let mut body_stmts: Vec<Stmt> = Vec::new();
    let mut current_stmt_tokens: Vec<crate::token::Token> = Vec::new();

    for token in body_tokens.iter() {
        current_stmt_tokens.push(token.clone());
        if token.typ == Type::Semicolon {
            // Parse the statement
            let stmt_str: String = current_stmt_tokens
                .iter()
                .map(|t| t.literal.clone())
                .collect::<Vec<_>>()
                .join(" ");
            // Try to parse as statement
            match crate::parser::parse(&stmt_str) {
                Ok(stmts) => body_stmts.extend(stmts),
                Err(_) => {
                    // Try as expression
                    if let Ok(expr) = crate::parser::expr::parse_expr_tokens(&current_stmt_tokens) {
                        body_stmts.push(Stmt::ExprStmt(expr));
                    }
                }
            }
            current_stmt_tokens.clear();
        }
    }

    // Handle any remaining tokens (without semicolon)
    if !current_stmt_tokens.is_empty() {
        let stmt_str: String = current_stmt_tokens
            .iter()
            .map(|t| t.literal.clone())
            .collect::<Vec<_>>()
            .join(" ");
        // Try with virtual semicolon appended (for statements like $>> or $#)
        let stmt_str_with_semi = stmt_str.clone() + ";";
        match crate::parser::parse(&stmt_str_with_semi) {
            Ok(stmts) => body_stmts.extend(stmts),
            Err(_) => {
                // Try without semicolon (for expressions)
                match crate::parser::parse(&stmt_str) {
                    Ok(stmts) => body_stmts.extend(stmts),
                    Err(_) => {
                        // Try as expression
                        if let Ok(expr) =
                            crate::parser::expr::parse_expr_tokens(&current_stmt_tokens)
                        {
                            body_stmts.push(Stmt::ExprStmt(expr));
                        }
                    }
                }
            }
        }
    }

    Ok(Box::new(Expr::FnLiteral(FnLiteral {
        span: Span::from_token(start),
        params,
        variadic_param,
        return_type,
        body: body_stmts,
    })))
}
