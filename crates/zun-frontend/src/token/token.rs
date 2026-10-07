// Token types for Zun lexer.
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Eof,
    Semicolon,
    Plus,
    Minus, // -
    Mul,   // *
    Div,   // /
    Mod,   // %
    Print,
    Read, // $<< - read from stdin
    File, // FILE - file I/O
    Assign,
    VarDecl,   // $ - variable declaration
    ConstDecl, // $@ - constant declaration

    // Comparison operators
    Eq,  // ==
    Ne,  // !=
    Gt,  // >
    Lt,  // <
    Gte, // >=
    Lte, // <=

    // Logical operators
    And, // &&
    Or,  // ||
    Not, // !

    // Control flow keywords
    If,         // $if
    Elif,       // $elif
    Else,       // $else
    While,      // $while
    Loop,       // $loop
    For,        // $for
    In,         // $in (for-in iteration)
    Break,      // $break
    Continue,   // $continue
    Try,        // $try
    Catch,      // $catch
    Throw,      // $throw
    Fn,         // $fn
    PrivateFn,  // _$fn
    Return,     // $#
    ModDecl,    // $mod - module declaration
    MainDecl,   // $main - main entry point
    TypeDecl,   // $Type - type declaration
    Question,   // ?
    ConfigRead, // $<<CONFIG - read from package.toml
    HdrRead,    // $HDR - read HTTP header
    Json,       // $JSON - JSON constructor
    Html,       // $HTML - HTML constructor
    Res,        // $RES - HTTP response constructor
    Static,     // $STATIC - static file serving
    At,         // @ - annotation entrypoint
    AtCors,     // @CORS - CORS annotation
    AtSetHdr,   // @SET_HDR - response header annotation
    AtHide,     // @HIDE - field visibility annotation

    // HTTP method keywords
    HttpGet,   // $GET
    HttpPost,  // $POST
    HttpPut,   // $PUT
    HttpDel,   // $DEL
    HttpPatch, // $PATCH
    HttpBlock, // $HTTP - HTTP block

    // Block delimiters
    LBrace,   // {
    RBrace,   // }
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    Colon,    // :
    Dot,      // .
    Spread,   // ...
    Comma,    // ,
    Arrow,    // ->

    // Compound assignment operators
    PlusAssign,  // +=
    MinusAssign, // -=
    MulAssign,   // *=
    DivAssign,   // /=
    ModAssign,   // %=

    // Literals
    Number,
    Char,
    Bool,
    Null, // null
    String,
    FString, // f"..." formatted string
    Ident,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Eof => write!(f, "EOF"),
            Type::Semicolon => write!(f, ";"),
            Type::Plus => write!(f, "+"),
            Type::Minus => write!(f, "-"),
            Type::Mul => write!(f, "*"),
            Type::Div => write!(f, "/"),
            Type::Mod => write!(f, "%"),
            Type::Print => write!(f, "$>>"),
            Type::Read => write!(f, "$<<"),
            Type::File => write!(f, "FILE"),
            Type::Assign => write!(f, "="),
            Type::VarDecl => write!(f, "$"),
            Type::ConstDecl => write!(f, "$@"),
            Type::Eq => write!(f, "=="),
            Type::Ne => write!(f, "!="),
            Type::Gt => write!(f, ">"),
            Type::Lt => write!(f, "<"),
            Type::Gte => write!(f, ">="),
            Type::Lte => write!(f, "<="),
            Type::And => write!(f, "&&"),
            Type::Or => write!(f, "||"),
            Type::Not => write!(f, "!"),
            Type::If => write!(f, "$if"),
            Type::Elif => write!(f, "$elif"),
            Type::Else => write!(f, "$else"),
            Type::While => write!(f, "$while"),
            Type::Loop => write!(f, "$loop"),
            Type::For => write!(f, "$for"),
            Type::In => write!(f, "$in"),
            Type::Break => write!(f, "$break"),
            Type::Continue => write!(f, "$continue"),
            Type::Try => write!(f, "$try"),
            Type::Catch => write!(f, "$catch"),
            Type::Throw => write!(f, "$throw"),
            Type::Fn => write!(f, "$fn"),
            Type::PrivateFn => write!(f, "_$fn"),
            Type::Return => write!(f, "$#"),
            Type::ModDecl => write!(f, "$mod"),
            Type::MainDecl => write!(f, "$main"),
            Type::TypeDecl => write!(f, "$Type"),
            Type::Question => write!(f, "?"),
            Type::ConfigRead => write!(f, "$<<CONFIG"),
            Type::HdrRead => write!(f, "$HDR"),
            Type::Json => write!(f, "$JSON"),
            Type::Html => write!(f, "$HTML"),
            Type::Res => write!(f, "$RES"),
            Type::Static => write!(f, "$STATIC"),
            Type::At => write!(f, "@"),
            Type::AtCors => write!(f, "@CORS"),
            Type::AtSetHdr => write!(f, "@SET_HDR"),
            Type::AtHide => write!(f, "@HIDE"),
            Type::HttpGet => write!(f, "$GET"),
            Type::HttpPost => write!(f, "$POST"),
            Type::HttpPut => write!(f, "$PUT"),
            Type::HttpDel => write!(f, "$DEL"),
            Type::HttpPatch => write!(f, "$PATCH"),
            Type::HttpBlock => write!(f, "$HTTP"),
            Type::LBrace => write!(f, "{{"),
            Type::RBrace => write!(f, "}}"),
            Type::LParen => write!(f, "("),
            Type::RParen => write!(f, ")"),
            Type::LBracket => write!(f, "["),
            Type::RBracket => write!(f, "]"),
            Type::Colon => write!(f, ":"),
            Type::Dot => write!(f, "."),
            Type::Spread => write!(f, "..."),
            Type::Comma => write!(f, ","),
            Type::Arrow => write!(f, "->"),
            Type::PlusAssign => write!(f, "+="),
            Type::MinusAssign => write!(f, "-="),
            Type::MulAssign => write!(f, "*="),
            Type::DivAssign => write!(f, "/="),
            Type::ModAssign => write!(f, "%="),
            Type::Number => write!(f, "NUMBER"),
            Type::Char => write!(f, "CHAR"),
            Type::Bool => write!(f, "BOOL"),
            Type::Null => write!(f, "null"),
            Type::String => write!(f, "STRING"),
            Type::FString => write!(f, "FSTRING"),
            Type::Ident => write!(f, "IDENT"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub typ: Type,
    pub literal: String,
    #[allow(dead_code)]
    pub pos: usize, // rune index in the source
}

impl Token {
    pub fn new(typ: Type, literal: &str, pos: usize) -> Self {
        Self {
            typ,
            literal: literal.to_string(),
            pos,
        }
    }

    pub fn eof(pos: usize) -> Self {
        Self {
            typ: Type::Eof,
            literal: String::new(),
            pos,
        }
    }
}
