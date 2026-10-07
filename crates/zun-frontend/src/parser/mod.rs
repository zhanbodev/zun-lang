// Parser module - converts tokens into AST statements.
pub mod expr;
pub mod literal;
pub mod stmt;

use self::stmt::StmtParser;
use crate::ast::Stmt;
use crate::error::Error;
use crate::lexer::Lexer;

pub use expr::{parse_expr_tokens, parse_expr_tokens_with_src};

/// Parse turns source into AST statements.
pub fn parse(src: &str) -> Result<Vec<Stmt>, Error> {
    let mut lexer = Lexer::new(src);
    let toks = lexer.lex_all().map_err(Error::from)?;

    let mut parser = StmtParser::new(&toks, src);
    parser.parse_stmts()
}

/// Calculate line and column from byte position
pub fn calc_line_col(src: &str, pos: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, c) in src.char_indices() {
        if i >= pos {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::ast::{Stmt, TypeExpr};
    use crate::diagnostics::codes;

    #[test]
    fn parses_basic_function_and_assignment() {
        let source = r#"
$fn add(a, b) -> Int {
    $# a + b;
}

$ value = add(1, 2);
"#;

        let statements = parse(source).expect("source should parse");
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn parses_private_function_and_wildcard_module_import() {
        let source = r#"
$mod math.*;

_$fn hidden() -> Int {
    $# 1;
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::ModDecl(stmt) => {
                assert_eq!(stmt.path, "math");
                assert!(stmt.wildcard);
            }
            other => panic!("expected module declaration, got {other:?}"),
        }

        match &statements[1] {
            Stmt::FnDecl(stmt) => assert!(!stmt.is_public),
            other => panic!("expected function declaration, got {other:?}"),
        }
    }

    #[test]
    fn parses_route_set_hdr_annotations() {
        let source = r#"
@SET_HDR({ "Cache-Control": "max-age=60" })
$GET("/items") list() {
    $# [];
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::HttpFn(stmt) => {
                assert_eq!(stmt.headers.len(), 1);
                assert_eq!(stmt.headers[0].name, "Cache-Control");
                assert_eq!(stmt.headers[0].value, "max-age=60");
            }
            other => panic!("expected HTTP route, got {other:?}"),
        }
    }

    #[test]
    fn parses_http_block_set_hdr_annotations() {
        let source = r#"
@SET_HDR({ "X-Frame-Options": "DENY" })
$HTTP("/api") {
    $GET("/users") list() {
        $# [];
    }
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::HttpBlock(stmt) => {
                assert_eq!(stmt.headers.len(), 1);
                assert_eq!(stmt.headers[0].name, "X-Frame-Options");
                assert_eq!(stmt.headers[0].value, "DENY");
            }
            other => panic!("expected HTTP block, got {other:?}"),
        }
    }

    #[test]
    fn rejects_set_hdr_in_invalid_position() {
        let source = r#"
@SET_HDR({ "X-Test": "1" })
$fn helper() -> Int {
    $# 1;
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(codes::PARSE_SET_HDR_INVALID_POSITION),
            "{rendered}"
        );
    }

    #[test]
    fn rejects_set_hdr_invalid_syntax() {
        let source = r#"
@SET_HDR({ "X-Test": 1 })
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(codes::PARSE_SET_HDR_INVALID_SYNTAX),
            "{rendered}"
        );
    }

    #[test]
    fn rejects_legacy_set_hdr_two_string_form() {
        let source = r#"
@SET_HDR("X-Test", "1")
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(codes::PARSE_SET_HDR_INVALID_SYNTAX),
            "{rendered}"
        );
        assert!(
            rendered.contains("@SET_HDR expects a config object"),
            "{rendered}"
        );
    }

    #[test]
    fn last_set_hdr_annotation_wins_on_same_node() {
        let source = r#"
@SET_HDR({ "X-Test": "1" })
@SET_HDR({ "x-test": "2" })
$GET("/items") list() {
    $# [];
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::HttpFn(stmt) => {
                assert_eq!(stmt.headers.len(), 2);
                assert_eq!(stmt.headers[0].name, "X-Test");
                assert_eq!(stmt.headers[0].value, "1");
                assert_eq!(stmt.headers[1].name, "x-test");
                assert_eq!(stmt.headers[1].value, "2");
            }
            other => panic!("expected HTTP route, got {other:?}"),
        }
    }

    #[test]
    fn rejects_set_hdr_non_string_key() {
        let source = r#"
@SET_HDR({ header: "1" })
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(codes::PARSE_SET_HDR_INVALID_SYNTAX),
            "{rendered}"
        );
        assert!(
            rendered.contains("expected string header name"),
            "{rendered}"
        );
    }

    #[test]
    fn parses_set_hdr_multiple_entries_in_single_annotation() {
        let source = r#"
@SET_HDR({ "X-One": "1", "X-Two": "2" })
$GET("/items") list() {
    $# [];
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::HttpFn(stmt) => {
                assert_eq!(stmt.headers.len(), 2);
                assert_eq!(stmt.headers[0].name, "X-One");
                assert_eq!(stmt.headers[0].value, "1");
                assert_eq!(stmt.headers[1].name, "X-Two");
                assert_eq!(stmt.headers[1].value, "2");
            }
            other => panic!("expected HTTP route, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_annotation_with_annotation_specific_diagnostic() {
        let source = r#"
@CACHE
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let diagnostic = error.diagnostic();

        assert_eq!(diagnostic.code, codes::PARSE_SET_HDR_INVALID_SYNTAX);
        assert!(
            diagnostic.message.contains("unknown annotation"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains("@CACHE"),
            "{}",
            diagnostic.message
        );
    }

    #[test]
    fn rejects_set_hdr_prefix_typo_as_unknown_annotation() {
        let source = r#"
@SET_HDRX("Cache-Control", "max-age=60")
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let diagnostic = error.diagnostic();

        assert_eq!(diagnostic.code, codes::PARSE_SET_HDR_INVALID_SYNTAX);
        assert!(
            diagnostic.message.contains("unknown annotation"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains("@SET_HDRX"),
            "{}",
            diagnostic.message
        );
    }

    #[test]
    fn parses_route_cors_allow_all_annotation() {
        let source = r#"
@CORS("*")
$GET("/items") list() {
    $# [];
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::HttpFn(stmt) => {
                let cors = stmt.cors.as_ref().expect("route cors");
                assert!(cors.allow_all);
                assert!(cors.origins.is_empty());
                assert!(cors.methods.is_empty());
                assert!(cors.headers.is_empty());
                assert_eq!(cors.max_age, None);
                assert!(!cors.credentials);
            }
            other => panic!("expected HTTP route, got {other:?}"),
        }
    }

    #[test]
    fn parses_block_and_global_cors_object_annotations() {
        let source = r#"
@CORS({
    origins: ["https://example.com"],
    methods: ["GET", "POST"],
    headers: ["Authorization"],
    max_age: 3600,
    credentials: true,
})
$main() {}

@CORS({ origins: ["https://admin.example.com"] })
$HTTP("/admin") {
    $GET("/users") list() {
        $# [];
    }
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::MainDecl(stmt) => {
                let cors = stmt.global_cors.as_ref().expect("global cors");
                assert!(!cors.allow_all);
                assert_eq!(cors.origins, vec!["https://example.com".to_string()]);
                assert_eq!(cors.methods, vec!["GET".to_string(), "POST".to_string()]);
                assert_eq!(cors.headers, vec!["Authorization".to_string()]);
                assert_eq!(cors.max_age, Some(3600));
                assert!(cors.credentials);
            }
            other => panic!("expected main declaration, got {other:?}"),
        }

        match &statements[1] {
            Stmt::HttpBlock(stmt) => {
                let cors = stmt.cors.as_ref().expect("block cors");
                assert_eq!(cors.origins, vec!["https://admin.example.com".to_string()]);
            }
            other => panic!("expected HTTP block, got {other:?}"),
        }
    }

    #[test]
    fn rejects_cors_in_invalid_position() {
        let source = r#"
@CORS("*")
$fn helper() -> Int {
    $# 1;
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(codes::PARSE_CORS_INVALID_POSITION),
            "{rendered}"
        );
    }

    #[test]
    fn rejects_duplicate_cors_on_same_node() {
        let source = r#"
@CORS("*")
@CORS({ origins: ["https://example.com"] })
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let diagnostic = error.diagnostic();
        let duplicate_pos = source
            .find("@CORS({ origins: [\"https://example.com\"] })")
            .expect("duplicate annotation");

        assert_eq!(diagnostic.code, codes::PARSE_CORS_DUPLICATE);
        assert!(
            diagnostic.message.contains("duplicate @CORS"),
            "{}",
            diagnostic.message
        );
        assert_eq!(
            diagnostic.span.map(|span| (span.start, span.end)),
            Some((duplicate_pos, duplicate_pos))
        );
    }

    #[test]
    fn rejects_cors_invalid_syntax() {
        let source = r#"
@CORS(42)
$GET("/items") list() {
    $# [];
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(codes::PARSE_CORS_INVALID_SYNTAX),
            "{rendered}"
        );
    }

    #[test]
    fn parse_type_decl_accepts_optional_list_field_type() {
        let source = r#"
$Type Feed {
    items: List<User>?
}
"#;

        let statements = parse(source).expect("source should parse");
        match &statements[0] {
            Stmt::TypeDecl(stmt) => {
                assert_eq!(
                    stmt.fields[0].type_expr,
                    TypeExpr::Optional(Box::new(TypeExpr::List(Box::new(TypeExpr::Named(
                        "User".to_string()
                    )))))
                );
            }
            other => panic!("expected type declaration, got {other:?}"),
        }
    }

    #[test]
    fn rejects_optional_list_item_type_expression() {
        let source = r#"
$Type Feed {
    items: List<User?>
}
"#;

        let error = parse(source).expect_err("source should fail");
        let rendered = error.to_string();
        assert!(rendered.contains(codes::PARSE_GENERIC), "{rendered}");
        assert!(
            rendered.contains("optional list item types are not supported"),
            "{rendered}"
        );
    }
}
