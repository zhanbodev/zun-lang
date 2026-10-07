// Lexer for Zun - converts source text into tokens.
use crate::diagnostics::Diagnostic;
use crate::diagnostics::codes;
use crate::token::{Token, Type};

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn lex_all(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut toks = Vec::new();

        loop {
            let tok = self.next_token()?;
            toks.push(tok.clone());
            if tok.typ == Type::Eof {
                break;
            }
        }

        Ok(toks)
    }

    fn next_token(&mut self) -> Result<Token, Diagnostic> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(Token::eof(self.pos));
        }

        let ch = self.peek();
        let start = self.pos;

        if self.match_seq("_$fn") {
            self.advance_n(4);
            return Ok(Token::new(Type::PrivateFn, "_$fn", start));
        }

        match ch {
            ';' => {
                self.advance();
                Ok(Token::new(Type::Semicolon, ";", start))
            }
            '+' => {
                self.advance();
                // Check for += (compound assignment)
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::PlusAssign, "+=", start));
                }
                Ok(Token::new(Type::Plus, "+", start))
            }
            '-' => {
                self.advance();
                // Check for -= (compound assignment)
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::MinusAssign, "-=", start));
                }
                // Check for -> (arrow)
                if self.pos < self.input.len() && self.peek() == '>' {
                    self.advance();
                    return Ok(Token::new(Type::Arrow, "->", start));
                }
                Ok(Token::new(Type::Minus, "-", start))
            }
            '*' => {
                self.advance();
                // Check for *= (compound assignment)
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::MulAssign, "*=", start));
                }
                Ok(Token::new(Type::Mul, "*", start))
            }
            '/' => {
                self.advance();
                // Check for single-line comment //
                if self.pos < self.input.len() && self.peek() == '/' {
                    self.advance();
                    self.skip_single_line_comment();
                    return self.next_token();
                }
                // Check for multi-line comment /*
                if self.pos < self.input.len() && self.peek() == '*' {
                    self.advance();
                    self.skip_multi_line_comment()?;
                    return self.next_token();
                }
                // Check for /= (compound assignment)
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::DivAssign, "/=", start));
                }
                Ok(Token::new(Type::Div, "/", start))
            }
            '%' => {
                self.advance();
                // Check for %= (compound assignment)
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::ModAssign, "%=", start));
                }
                Ok(Token::new(Type::Mod, "%", start))
            }
            '!' => {
                self.advance();
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::Ne, "!=", start));
                }
                Ok(Token::new(Type::Not, "!", start))
            }
            '<' => {
                // Check for <=
                self.advance();
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::Lte, "<=", start));
                }
                Ok(Token::new(Type::Lt, "<", start))
            }
            '>' => {
                self.advance();
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::Gte, ">=", start));
                }
                Ok(Token::new(Type::Gt, ">", start))
            }
            '=' => {
                self.advance();
                if self.pos < self.input.len() && self.peek() == '=' {
                    self.advance();
                    return Ok(Token::new(Type::Eq, "==", start));
                }
                // Single = is assignment
                Ok(Token::new(Type::Assign, "=", start))
            }
            '&' => {
                self.advance();
                if self.pos < self.input.len() && self.peek() == '&' {
                    self.advance();
                    return Ok(Token::new(Type::And, "&&", start));
                }
                Err(self.error(codes::LEX_UNEXPECTED_CHAR, "unexpected '&'", start))
            }
            '|' => {
                self.advance();
                if self.pos < self.input.len() && self.peek() == '|' {
                    self.advance();
                    return Ok(Token::new(Type::Or, "||", start));
                }
                Err(self.error(codes::LEX_UNEXPECTED_CHAR, "unexpected '|'", start))
            }
            '{' => {
                self.advance();
                Ok(Token::new(Type::LBrace, "{", start))
            }
            '}' => {
                self.advance();
                Ok(Token::new(Type::RBrace, "}", start))
            }
            '(' => {
                self.advance();
                Ok(Token::new(Type::LParen, "(", start))
            }
            ')' => {
                self.advance();
                Ok(Token::new(Type::RParen, ")", start))
            }
            '[' => {
                self.advance();
                Ok(Token::new(Type::LBracket, "[", start))
            }
            ']' => {
                self.advance();
                Ok(Token::new(Type::RBracket, "]", start))
            }
            ',' => {
                self.advance();
                Ok(Token::new(Type::Comma, ",", start))
            }
            ':' => {
                self.advance();
                Ok(Token::new(Type::Colon, ":", start))
            }
            '@' => {
                if self.match_seq("@CORS") && self.annotation_boundary_after(5) {
                    self.advance_n(5);
                    return Ok(Token::new(Type::AtCors, "@CORS", start));
                }
                if self.match_seq("@SET_HDR") && self.annotation_boundary_after(8) {
                    self.advance_n(8);
                    return Ok(Token::new(Type::AtSetHdr, "@SET_HDR", start));
                }
                if self.match_seq("@HIDE") && self.annotation_boundary_after(5) {
                    self.advance_n(5);
                    return Ok(Token::new(Type::AtHide, "@HIDE", start));
                }
                self.advance();
                Ok(Token::new(Type::At, "@", start))
            }
            '.' => {
                self.advance();
                // Check for ... (spread)
                if self.pos < self.input.len() && self.peek() == '.' {
                    self.advance();
                    if self.pos < self.input.len() && self.peek() == '.' {
                        self.advance();
                        return Ok(Token::new(Type::Spread, "...", start));
                    }
                }
                Ok(Token::new(Type::Dot, ".", start))
            }
            '?' => {
                self.advance();
                Ok(Token::new(Type::Question, "?", start))
            }
            '$' => self.read_dollar(start),
            '"' => self.read_string(start),
            '\'' => self.read_char(start),
            _ => {
                if ch.is_ascii_digit() || ch == '-' {
                    return self.read_number(start);
                }
                // Identifier or keyword.
                let lit = self.read_until_delimiter();
                // Check for f-string: "f" followed by '"'
                if lit == "f" && self.pos < self.input.len() && self.input[self.pos] == '"' {
                    return self.read_fstring(start);
                }
                if lit == "true" {
                    return Ok(Token::new(Type::Bool, "true", start));
                }
                if lit == "false" {
                    return Ok(Token::new(Type::Bool, "false", start));
                }
                if lit == "null" {
                    return Ok(Token::new(Type::Null, "null", start));
                }
                // Check for 'in' keyword (for-in loop)
                if lit == "in" {
                    return Ok(Token::new(Type::In, "in", start));
                }
                Ok(Token::new(Type::Ident, &lit, start))
            }
        }
    }

    fn read_dollar(&mut self, start: usize) -> Result<Token, Diagnostic> {
        // Order matters: match longer sequences first to avoid prefix collisions

        if self.match_seq("$>>") {
            self.advance_n(3);
            return Ok(Token::new(Type::Print, "$>>", start));
        }

        if self.match_seq("$<<CONFIG") {
            self.advance_n(9);
            return Ok(Token::new(Type::ConfigRead, "$<<CONFIG", start));
        }

        if self.match_seq("$HDR") {
            self.advance_n(4);
            return Ok(Token::new(Type::HdrRead, "$HDR", start));
        }

        if self.match_seq("$HTTP") {
            self.advance_n(5);
            return Ok(Token::new(Type::HttpBlock, "$HTTP", start));
        }

        if self.match_seq("$<<") {
            self.advance_n(3);
            return Ok(Token::new(Type::Read, "$<<", start));
        }

        if self.match_seq("$continue") {
            self.advance_n(9);
            return Ok(Token::new(Type::Continue, "$continue", start));
        }

        if self.match_seq("$throw") {
            self.advance_n(6);
            return Ok(Token::new(Type::Throw, "$throw", start));
        }

        if self.match_seq("$try") {
            self.advance_n(4);
            return Ok(Token::new(Type::Try, "$try", start));
        }

        if self.match_seq("$catch") {
            self.advance_n(6);
            return Ok(Token::new(Type::Catch, "$catch", start));
        }

        if self.match_seq("$fn") {
            self.advance_n(3);
            return Ok(Token::new(Type::Fn, "$fn", start));
        }

        if self.match_seq("$while") {
            self.advance_n(6);
            return Ok(Token::new(Type::While, "$while", start));
        }

        if self.match_seq("$break") {
            self.advance_n(6);
            return Ok(Token::new(Type::Break, "$break", start));
        }

        if self.match_seq("$elif") {
            self.advance_n(5);
            return Ok(Token::new(Type::Elif, "$elif", start));
        }

        if self.match_seq("$else") {
            self.advance_n(5);
            return Ok(Token::new(Type::Else, "$else", start));
        }

        if self.match_seq("$loop") {
            self.advance_n(5);
            return Ok(Token::new(Type::Loop, "$loop", start));
        }

        if self.match_seq("$for") {
            self.advance_n(4);
            return Ok(Token::new(Type::For, "$for", start));
        }

        if self.match_seq("$if") {
            self.advance_n(3);
            return Ok(Token::new(Type::If, "$if", start));
        }

        if self.match_seq("$@") {
            self.advance_n(2);
            return Ok(Token::new(Type::ConstDecl, "$@", start));
        }

        if self.match_seq("$#") {
            self.advance_n(2);
            return Ok(Token::new(Type::Return, "$#", start));
        }

        if self.match_seq("$mod") {
            self.advance_n(4);
            return Ok(Token::new(Type::ModDecl, "$mod", start));
        }

        if self.match_seq("$main") {
            self.advance_n(5);
            return Ok(Token::new(Type::MainDecl, "$main", start));
        }

        if self.match_seq("$Type") {
            self.advance_n(5);
            return Ok(Token::new(Type::TypeDecl, "$Type", start));
        }

        if self.match_seq("$JSON") {
            self.advance_n(5);
            return Ok(Token::new(Type::Json, "$JSON", start));
        }

        if self.match_seq("$HTML") {
            self.advance_n(5);
            return Ok(Token::new(Type::Html, "$HTML", start));
        }

        if self.match_seq("$RES") {
            self.advance_n(4);
            return Ok(Token::new(Type::Res, "$RES", start));
        }

        if self.match_seq("$STATIC") {
            self.advance_n(7);
            return Ok(Token::new(Type::Static, "$STATIC", start));
        }

        if self.match_seq("$GET") {
            self.advance_n(4);
            return Ok(Token::new(Type::HttpGet, "$GET", start));
        }

        if self.match_seq("$POST") {
            self.advance_n(5);
            return Ok(Token::new(Type::HttpPost, "$POST", start));
        }

        if self.match_seq("$PUT") {
            self.advance_n(4);
            return Ok(Token::new(Type::HttpPut, "$PUT", start));
        }

        if self.match_seq("$DEL") {
            self.advance_n(4);
            return Ok(Token::new(Type::HttpDel, "$DEL", start));
        }

        if self.match_seq("$PATCH") {
            self.advance_n(6);
            return Ok(Token::new(Type::HttpPatch, "$PATCH", start));
        }

        // Single $ is variable declaration
        self.advance();
        Ok(Token::new(Type::VarDecl, "$", start))
    }

    fn read_string(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.advance(); // consume opening "
        let mut result = String::new();

        while self.pos < self.input.len() {
            let ch = self.peek();
            if ch == '"' {
                self.advance(); // consume closing "
                return Ok(Token::new(Type::String, &result, start));
            } else if ch == '\\' {
                // Escape sequence
                self.advance(); // consume '\'
                if self.pos >= self.input.len() {
                    return Err(self.error(
                        codes::LEX_UNTERMINATED_STRING,
                        "unclosed string",
                        start,
                    ));
                }
                let next = self.peek();
                let escaped = match next {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    '\\' => '\\',
                    '"' => '"',
                    _ => {
                        return Err(self.error(
                            codes::LEX_INVALID_CHAR,
                            format!("invalid escape sequence \"\\{}\"", next),
                            self.pos,
                        ));
                    }
                };
                result.push(escaped);
                self.advance(); // consume escaped char
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(self.error(codes::LEX_UNTERMINATED_STRING, "unclosed string", start))
    }

    /// Read an f-string: f"..."
    /// Format: f"Hello {name}, you have {count} items"
    /// - Text outside {} is literal
    /// - Inside {} can be: variable, expression, method call
    fn read_fstring(&mut self, start: usize) -> Result<Token, Diagnostic> {
        // Check if we need to consume 'f' or if we're already at '"'
        if self.pos < self.input.len() && self.input[self.pos] == 'f' {
            // Consume the 'f' character
            self.advance();
        }
        // Now consume the opening "
        self.advance();

        let mut literal = String::new();
        let mut brace_depth = 0;

        while self.pos < self.input.len() {
            let ch = self.peek();
            match ch {
                '{' => {
                    brace_depth += 1;
                    literal.push('{');
                    self.advance();
                }
                '}' => {
                    if brace_depth == 0 {
                        return Err(self.error(
                            codes::LEX_INVALID_CHAR,
                            "f-string syntax error: unexpected \"}\"",
                            self.pos,
                        ));
                    }
                    brace_depth -= 1;
                    literal.push('}');
                    self.advance();
                }
                '"' => {
                    if brace_depth > 0 {
                        return Err(self.error(
                            codes::LEX_INVALID_CHAR,
                            "f-string syntax error: unclosed \"{\"",
                            self.pos,
                        ));
                    }
                    self.advance(); // consume closing "
                    return Ok(Token::new(Type::FString, &literal, start));
                }
                '\\' => {
                    // Escape sequence
                    self.advance(); // consume '\'
                    if self.pos >= self.input.len() {
                        return Err(self.error(
                            codes::LEX_UNTERMINATED_STRING,
                            "unclosed f-string",
                            start,
                        ));
                    }
                    let next = self.peek();
                    let escaped = match next {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        '\\' => '\\',
                        '"' => '"',
                        '{' => '{',
                        '}' => '}',
                        _ => {
                            return Err(self.error(
                                codes::LEX_INVALID_CHAR,
                                format!("invalid escape sequence \"\\{}\" in f-string", next),
                                self.pos,
                            ));
                        }
                    };
                    literal.push(escaped);
                    self.advance(); // consume escaped char
                }
                _ => {
                    literal.push(ch);
                    self.advance();
                }
            }
        }

        Err(self.error(codes::LEX_UNTERMINATED_STRING, "unclosed f-string", start))
    }

    fn read_char(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.advance(); // consume opening '
        if self.pos >= self.input.len() {
            return Err(self.error(codes::LEX_INVALID_CHAR, "unclosed char", start));
        }
        let ch = self.peek();

        // Check for escape sequences - not supported in char
        if ch == '\\' {
            return Err(self.error(
                codes::LEX_INVALID_CHAR,
                "Char type does not support escape sequences",
                self.pos,
            ));
        }

        self.advance(); // consume the character
        if self.pos >= self.input.len() || self.peek() != '\'' {
            return Err(self.error(codes::LEX_INVALID_CHAR, "unclosed char", start));
        }
        self.advance(); // consume closing '
        Ok(Token::new(Type::Char, &ch.to_string(), start))
    }

    fn read_number(&mut self, start: usize) -> Result<Token, Diagnostic> {
        // Check for negative number
        let mut has_sign = false;
        if self.peek() == '-' {
            self.advance();
            has_sign = true;
        }

        // Read digits before decimal point
        let digits_start = self.pos;
        while self.pos < self.input.len() && self.peek().is_ascii_digit() {
            self.advance();
        }
        let _int_part_len = self.pos - digits_start;

        // Check for decimal point - only treat as float if digits follow
        let mut is_float = false;
        let float_end_pos;
        if self.pos < self.input.len() && self.peek() == '.' {
            let after_dot = self.pos + 1;
            // Check if there are digits after the decimal point
            if after_dot < self.input.len() && self.input[after_dot].is_ascii_digit() {
                is_float = true;
                self.advance();
                // Read digits after decimal point
                while self.pos < self.input.len() && self.peek().is_ascii_digit() {
                    self.advance();
                }
                float_end_pos = self.pos;
            } else {
                // No digits after decimal point - don't treat as float
                // Return integer and let the dot be a separate token
                float_end_pos = self.pos;
            }
        } else {
            float_end_pos = self.pos;
        }

        let literal: String = self.input[start..float_end_pos].iter().collect();
        if literal.is_empty() || (has_sign && literal == "-") {
            return Err(self.error(codes::LEX_INVALID_CHAR, "invalid number literal", start));
        }

        // Validate the number
        if is_float {
            literal
                .parse::<f64>()
                .map_err(|_| self.error(codes::LEX_INVALID_CHAR, "invalid float literal", start))?;
        } else {
            literal.parse::<i64>().map_err(|_| {
                self.error(codes::LEX_INVALID_CHAR, "invalid integer literal", start)
            })?;
        }

        Ok(Token::new(Type::Number, &literal, start))
    }

    fn read_until_delimiter(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() && !is_delimiter(self.peek()) {
            self.advance();
        }
        self.input[start..self.pos].iter().collect()
    }

    #[allow(dead_code)]
    fn read_until_rune(&mut self, target: char) -> Vec<char> {
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.peek() == target {
                return self.input[start..self.pos].to_vec();
            }
            self.advance();
        }
        Vec::new()
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.peek();
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '\\' {
                // Skip escape character (handles shell-escaped ! and other chars)
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skip single-line comment starting with //
    fn skip_single_line_comment(&mut self) {
        while self.pos < self.input.len() && self.peek() != '\n' {
            self.advance();
        }
    }

    /// Skip multi-line comment starting with /* and ending with */
    fn skip_multi_line_comment(&mut self) -> Result<(), Diagnostic> {
        while self.pos < self.input.len() {
            if self.peek() == '*'
                && self.pos + 1 < self.input.len()
                && self.input[self.pos + 1] == '/'
            {
                self.advance_n(2); // Skip */
                return Ok(());
            }
            self.advance();
        }
        Err(self.error(
            codes::LEX_UNTERMINATED_COMMENT,
            "unterminated multi-line comment",
            self.pos.saturating_sub(1),
        ))
    }

    fn error(&self, code: &'static str, message: impl Into<String>, pos: usize) -> Diagnostic {
        let (line, column) = self.line_col(pos);
        Diagnostic::error(code, message).with_location(line, column)
    }

    fn line_col(&self, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        for (index, ch) in self.input.iter().enumerate() {
            if index >= pos {
                break;
            }
            if *ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }

    fn peek(&self) -> char {
        self.input[self.pos]
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn advance_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn match_seq(&self, seq: &str) -> bool {
        if self.pos + seq.len() > self.input.len() {
            return false;
        }
        for (i, c) in seq.chars().enumerate() {
            if self.input[self.pos + i] != c {
                return false;
            }
        }
        true
    }

    fn annotation_boundary_after(&self, len: usize) -> bool {
        let next_pos = self.pos + len;
        next_pos >= self.input.len() || is_delimiter(self.input[next_pos])
    }
}

fn is_delimiter(ch: char) -> bool {
    matches!(
        ch,
        ' ' | '\t'
            | '\n'
            | '\r'
            | ';'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '<'
            | '>'
            | '='
            | '!'
            | '&'
            | '|'
            | '$'
            | '@'
            | '"'
            | '\''
            | '{'
            | '}'
            | '('
            | ')'
            | '['
            | ']'
            | ','
            | '.'
            | ':'
            | '?'
    )
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::token::Type;

    #[test]
    fn lexes_comments_and_compound_assignment() {
        let mut lexer = Lexer::new("// comment\n$ value = 1;\nvalue += 2;");
        let tokens = lexer.lex_all().expect("lexing should succeed");
        let token_types: Vec<Type> = tokens.into_iter().map(|token| token.typ).collect();
        assert!(token_types.contains(&Type::VarDecl));
        assert!(token_types.contains(&Type::PlusAssign));
    }

    #[test]
    fn lexes_private_function_keyword() {
        let mut lexer = Lexer::new("_$fn helper() { }");
        let tokens = lexer.lex_all().expect("lexing should succeed");
        assert_eq!(tokens[0].typ, Type::PrivateFn);
    }

    #[test]
    fn lexes_set_hdr_only_on_annotation_boundary() {
        let mut lexer = Lexer::new("@SET_HDRX");
        let tokens = lexer.lex_all().expect("lexing should succeed");

        assert_eq!(tokens[0].typ, Type::At);
        assert_eq!(tokens[1].typ, Type::Ident);
        assert_eq!(tokens[1].literal, "SET_HDRX");
    }
}
