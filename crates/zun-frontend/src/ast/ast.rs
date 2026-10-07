// Abstract Syntax Tree node definitions for Zun.
use std::borrow::Cow;

use crate::token::Type;

/// Source code span - start and end positions (character indices)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn from_token(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

/// Helper trait to get span from AST nodes
pub trait Spanned {
    fn span(&self) -> Span;
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(NumberLiteral),
    Char(CharLiteral),
    Bool(BoolLiteral),
    Null(NullLiteral),
    StringLiteral(StringLiteral),
    FString(FStringLiteral), // f"Hello {name}"
    ListLiteral(ListLiteral),
    MapLiteral(MapLiteral),
    VarLookup(VarLookup),
    IndexAccess(IndexAccess),
    MethodCall(MethodCall),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    FnCall(FnCallExpr),
    FnLiteral(FnLiteral),       // Anonymous function: $fn(x, y) -> Int { ... }
    Read(ExprRead),             // $<<ENV("KEY") or $<<LINE("prompt") as expression
    ConfigRead(ConfigReadExpr), // $<<CONFIG("KEY") as expression
    HdrRead(HdrReadExpr),       // $HDR("Header-Name")
    JsonConstructor(JsonConstructor), // $JSON { "key": value, ... }
    HtmlConstructor(HtmlConstructor), // $HTML("<h1>...</h1>")
    ResConstructor(ResConstructor), // $RES(status, body)
    TypeInstance(StructConstructor), // TypeName { field: value, ... }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Print(PrintStmt),
    Read(ReadStmt), // $<<ENV("KEY") or $<<LINE("prompt")
    Assign(AssignStmt),
    VarDecl(VarDeclStmt),     // $ a = 1;
    ConstDecl(ConstDeclStmt), // $@ a = 1;
    If(IfStmt),               // $if condition { ... } $elif ... $else ...
    While(WhileStmt),         // $while condition { ... }
    Loop(LoopStmt),           // $loop { ... }
    For(ForStmt),             // $for init; condition; update { ... }
    ForIn(ForInStmt),         // $for item in iterable { ... }
    Break(BreakStmt),         // $break;
    Continue(ContinueStmt),   // $continue;
    Exit(ExitStmt),
    FnDecl(FnDeclStmt),       // $fn name(params) -> type { body }
    HttpFn(HttpFnStmt),       // $GET("/path") name(params) -> type { body }
    HttpBlock(HttpBlockStmt), // $HTTP { routes... }
    Static(StaticStmt),       // $STATIC - static file serving
    Return(ReturnStmt),       // $# expression;
    Try(TryStmt),             // $try { ... } $catch name { ... }
    Throw(ThrowStmt),         // $throw expression;
    ModDecl(ModDeclStmt),     // $mod path;
    MainDecl(MainDeclStmt),   // $main() { body }
    TypeDecl(TypeDeclStmt),   // $Type Name { fields }
    ExprStmt(Box<Expr>),      // expression statement (for function calls as statements)
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub span: Span,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub op: Type,
}

#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub span: Span,
    pub value: Cow<'static, str>,
}

#[derive(Debug, Clone)]
pub struct CharLiteral {
    pub span: Span,
    pub value: Cow<'static, str>,
}

#[derive(Debug, Clone)]
pub struct BoolLiteral {
    pub span: Span,
    pub value: bool,
}

#[derive(Debug, Clone)]
pub struct NullLiteral {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub span: Span,
    pub value: Cow<'static, str>,
}

/// F-string literal: f"Hello {name}"
/// Segments are either literal text or expression to interpolate
#[derive(Debug, Clone)]
pub struct FStringLiteral {
    pub span: Span,
    pub segments: Vec<FStringSegment>,
}

#[derive(Debug, Clone)]
pub enum FStringSegment {
    /// Literal text (outside {braces})
    Literal(String),
    /// Expression to interpolate (inside {braces})
    Expression(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct ListLiteral {
    pub span: Span,
    pub elements: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct MapLiteral {
    pub span: Span,
    pub entries: Vec<(String, Expr)>, // (key, value expression)
}

#[derive(Debug, Clone)]
pub struct IndexAccess {
    pub span: Span,
    pub object: Box<Expr>,
    pub index: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct MethodCall {
    pub span: Span,
    pub object: Box<Expr>,
    pub method: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub span: Span,
    pub op: Type,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct VarLookup {
    pub span: Span,
    pub name: Cow<'static, str>,
}

#[derive(Debug, Clone)]
pub struct PrintStmt {
    pub span: Span,
    pub value: Box<Expr>,
    pub target: PrintTarget, // stdout or stderr
}

/// Read statement: $<<ENV("KEY") or $<<LINE("prompt")
#[derive(Debug, Clone)]
pub struct ReadStmt {
    pub span: Span,
    pub mode: ReadMode,            // ENV or LINE
    pub prompt: Option<Box<Expr>>, // optional prompt for LINE mode
}

/// Read mode: ENV reads environment variable, LINE reads stdin
#[derive(Debug, Clone, PartialEq)]
pub enum ReadMode {
    Env,  // $<<ENV("KEY")
    Line, // $<<LINE("prompt")
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrintTarget {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub span: Span,
    pub name: Box<Expr>,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct VarDeclStmt {
    pub span: Span,
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ConstDeclStmt {
    pub span: Span,
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct IfBranch {
    pub span: Span,
    pub condition: Box<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub span: Span,
    pub branches: Vec<IfBranch>,
    pub else_body: Vec<Stmt>, // $else branch
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub span: Span,
    pub condition: Box<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct LoopStmt {
    pub span: Span,
    pub body: Vec<Stmt>,
}

/// $for init; condition; update { body }
/// init and update are optional single statements (VarDecl or Assign).
#[derive(Debug, Clone)]
pub struct ForStmt {
    pub span: Span,
    pub init: Option<Box<Stmt>>,
    pub condition: Option<Box<Expr>>,
    pub update: Option<Box<Stmt>>,
    pub body: Vec<Stmt>,
}

/// $for item in iterable { body }
/// item is the loop variable, iterable is List, Map, or String
#[derive(Debug, Clone)]
pub struct ForInStmt {
    pub span: Span,
    pub var: String,         // loop variable name
    pub iterable: Box<Expr>, // expression to iterate over
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct BreakStmt {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ContinueStmt {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExitStmt {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDeclStmt {
    pub span: Span,
    pub name: String,
    pub is_public: bool,
    pub params: Vec<FnParam>,
    pub variadic_param: Option<String>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct FnParam {
    pub name: String,
    pub type_annotation: Option<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct SetHdrEntry {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsConfig {
    pub allow_all: bool,
    pub origins: Vec<String>,
    pub methods: Vec<String>,
    pub headers: Vec<String>,
    pub max_age: Option<i64>,
    pub credentials: bool,
}

/// HTTP function: $GET("/path") name(params) -> type { body }
#[derive(Debug, Clone)]
pub struct HttpFnStmt {
    pub span: Span,
    pub method: String, // "GET", "POST", "PUT", "DELETE", "PATCH"
    pub path: String,   // "/user/:id"
    pub name: String,   // function name
    pub params: Vec<FnParam>,
    pub variadic_param: Option<String>,
    pub return_type: Option<TypeExpr>,
    pub cors: Option<CorsConfig>,
    pub headers: Vec<SetHdrEntry>,
    pub body: Vec<Stmt>,
}

/// HTTP block: $HTTP { routes... } or $HTTP(prefix).link(module)
#[derive(Debug, Clone)]
pub struct HttpBlockStmt {
    pub span: Span,
    pub prefix: Option<String>, // 路径前缀，如 "/v1/api/"
    pub link: Option<String>,   // 要链接的模块，如 "api"
    pub cors: Option<CorsConfig>,
    pub headers: Vec<SetHdrEntry>,
    pub routes: Vec<HttpFnStmt>, // 内嵌的 HTTP 路由
}

/// Static file serving: $STATIC("/url-prefix", "dir.module") or $STATIC("dir")
#[derive(Debug, Clone)]
pub struct StaticStmt {
    pub span: Span,
    pub url_prefix: String,  // URL 前缀，如 "/css"
    pub module_path: String, // 模块路径，如 "css" 或 "css.zun"
}

/// Module declaration: $mod path;
#[derive(Debug, Clone)]
pub struct ModDeclStmt {
    pub span: Span,
    pub path: String, // e.g., "dao.user"
    pub wildcard: bool,
}

/// Main entry point: $main() { body }
#[derive(Debug, Clone)]
pub struct MainDeclStmt {
    pub span: Span,
    pub global_cors: Option<CorsConfig>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Named(String),
    List(Box<TypeExpr>),
    Optional(Box<TypeExpr>),
}

/// A single field in a $Type declaration: `name: TypeName?` or `@HIDE name: TypeName`
#[derive(Debug, Clone)]
pub struct TypeField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub hidden: bool,
}

/// Struct constructor: `TypeName { field: value, ... }`
#[derive(Debug, Clone)]
pub struct StructConstructor {
    pub span: Span,
    pub type_name: String,
    pub fields: Vec<(String, Expr)>,
}

/// Type declaration: $Type User { id: Int, name: Str, email: Str? }
#[derive(Debug, Clone)]
pub struct TypeDeclStmt {
    pub span: Span,
    pub name: String,
    pub fields: Vec<TypeField>,
}

#[derive(Debug, Clone)]
pub struct FnCallExpr {
    pub span: Span,
    pub name: String,
    pub args: Vec<Expr>,
}

/// Anonymous function literal: $fn(x, y) -> Int { ... }
#[derive(Debug, Clone)]
pub struct FnLiteral {
    pub span: Span,
    pub params: Vec<FnParam>,
    pub variadic_param: Option<String>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
}

/// Read expression: $<<ENV("KEY") or $<<LINE("prompt") as expression (in assignment)
#[derive(Debug, Clone)]
pub struct ExprRead {
    pub span: Span,
    pub mode: ReadMode,
    pub prompt: Option<Box<Expr>>,
}

/// Config read expression: $<<CONFIG("KEY") as expression
#[derive(Debug, Clone)]
pub struct ConfigReadExpr {
    pub span: Span,
    pub key: String,
}

/// HTTP header read expression: $HDR("Header-Name")
#[derive(Debug, Clone)]
pub struct HdrReadExpr {
    pub span: Span,
    pub header_name: String,
}

/// JSON constructor: $JSON { "key": value, ... }
#[derive(Debug, Clone)]
pub struct JsonConstructor {
    pub span: Span,
    pub entries: Vec<(String, Expr)>, // (key, value expression)
}

/// HTML constructor: $HTML("<h1>...</h1>")
#[derive(Debug, Clone)]
pub struct HtmlConstructor {
    pub span: Span,
    pub content: Box<Expr>, // HTML content expression
}

/// Response constructor: $RES(status, body)
#[derive(Debug, Clone)]
pub struct ResConstructor {
    pub span: Span,
    pub status: Box<Expr>,       // status code
    pub body: Option<Box<Expr>>, // response body
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub span: Span,
    pub value: Option<Box<Expr>>,
}

/// $try { body } $catch name { catch_body }
#[derive(Debug, Clone)]
pub struct TryStmt {
    pub span: Span,
    pub body: Vec<Stmt>,
    pub catch_var: String,
    pub catch_body: Vec<Stmt>,
}

/// $throw expression;
#[derive(Debug, Clone)]
pub struct ThrowStmt {
    pub span: Span,
    pub value: Box<Expr>,
}

// Implement Spanned trait for all AST nodes
impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl Spanned for NumberLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for CharLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for BoolLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for NullLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for StringLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for FStringLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ListLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for MapLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for IndexAccess {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for MethodCall {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for VarLookup {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for BinaryExpr {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for UnaryExpr {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for FnCallExpr {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for FnLiteral {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ExprRead {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ConfigReadExpr {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for HdrReadExpr {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for JsonConstructor {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for HtmlConstructor {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ResConstructor {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ModDeclStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for MainDeclStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for PrintStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ReadStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for AssignStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for VarDeclStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ConstDeclStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for IfBranch {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for IfStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for WhileStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for LoopStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ForStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ForInStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for BreakStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ContinueStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ExitStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for FnDeclStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for HttpFnStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for HttpBlockStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for StaticStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ReturnStmt {
    fn span(&self) -> Span {
        self.span
    }
}

// Spanned for Box<Expr>
impl Spanned for Box<Expr> {
    fn span(&self) -> Span {
        (**self).span()
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Number(n) => n.span(),
            Expr::Char(c) => c.span(),
            Expr::Bool(b) => b.span(),
            Expr::Null(n) => n.span(),
            Expr::StringLiteral(s) => s.span(),
            Expr::FString(f) => f.span(),
            Expr::ListLiteral(l) => l.span(),
            Expr::MapLiteral(m) => m.span(),
            Expr::IndexAccess(i) => i.span(),
            Expr::MethodCall(m) => m.span(),
            Expr::VarLookup(v) => v.span(),
            Expr::Binary(b) => b.span(),
            Expr::Unary(u) => u.span(),
            Expr::FnCall(f) => f.span(),
            Expr::FnLiteral(f) => f.span(),
            Expr::Read(r) => r.span(),
            Expr::ConfigRead(c) => c.span(),
            Expr::HdrRead(h) => h.span(),
            Expr::JsonConstructor(j) => j.span(),
            Expr::HtmlConstructor(h) => h.span(),
            Expr::ResConstructor(r) => r.span(),
            Expr::TypeInstance(t) => t.span,
        }
    }
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Print(s) => s.span(),
            Stmt::Assign(s) => s.span(),
            Stmt::VarDecl(s) => s.span(),
            Stmt::ConstDecl(s) => s.span(),
            Stmt::If(s) => s.span(),
            Stmt::While(s) => s.span(),
            Stmt::Loop(s) => s.span(),
            Stmt::For(s) => s.span(),
            Stmt::ForIn(s) => s.span(),
            Stmt::Break(s) => s.span(),
            Stmt::Continue(s) => s.span(),
            Stmt::Exit(s) => s.span(),
            Stmt::FnDecl(s) => s.span(),
            Stmt::HttpFn(s) => s.span(),
            Stmt::HttpBlock(s) => s.span(),
            Stmt::Static(s) => s.span(),
            Stmt::Return(s) => s.span(),
            Stmt::Try(s) => s.span,
            Stmt::Throw(s) => s.span,
            Stmt::ModDecl(s) => s.span(),
            Stmt::MainDecl(s) => s.span(),
            Stmt::TypeDecl(s) => s.span(),
            Stmt::Read(s) => s.span(),
            Stmt::ExprStmt(e) => e.span(),
        }
    }
}

impl Spanned for TryStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for TypeDeclStmt {
    fn span(&self) -> Span {
        self.span
    }
}

impl Spanned for ThrowStmt {
    fn span(&self) -> Span {
        self.span
    }
}
