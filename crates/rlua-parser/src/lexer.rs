use crate::token::{Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub line: u32,
    pub column: u32,
}

pub struct Lexer<'src> {
    source: &'src [u8],
    pos: usize,
    line: u32,
    column: u32,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.source.get(self.pos).copied()?;
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            column: self.column,
        }
    }

    fn err(&self, msg: impl Into<String>) -> LexError {
        LexError {
            message: msg.into(),
            line: self.line,
            column: self.column,
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch == b' ' || ch == b'\t' || ch == b'\r' || ch == b'\n' || ch == b'\x0c' {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for comment
            if self.peek() == Some(b'-') && self.peek_at(1) == Some(b'-') {
                self.advance(); // -
                self.advance(); // -

                // Check for long comment --[[ or --[==[
                if self.peek() == Some(b'[')
                    && let Some(level) = self.check_long_bracket_open()
                {
                    self.read_long_string(level)?;
                    continue;
                }
                // Short comment: skip to end of line
                while let Some(ch) = self.peek() {
                    if ch == b'\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            break;
        }
        Ok(())
    }

    /// Check if current position starts a long bracket `[===[`.
    /// Returns the level (number of `=` signs) if it matches, None otherwise.
    /// Does NOT advance the position.
    fn check_long_bracket_open(&self) -> Option<usize> {
        if self.peek() != Some(b'[') {
            return None;
        }
        let mut level = 0;
        let mut offset = 1;
        while self.peek_at(offset) == Some(b'=') {
            level += 1;
            offset += 1;
        }
        if self.peek_at(offset) == Some(b'[') {
            Some(level)
        } else {
            None
        }
    }

    /// Read a long string `[===[...]=== ]` of the given level.
    /// Assumes the position is at the opening `[`.
    fn read_long_string(&mut self, level: usize) -> Result<String, LexError> {
        // Skip opening bracket [====[
        self.advance(); // [
        for _ in 0..level {
            self.advance(); // =
        }
        self.advance(); // [

        // Skip immediate newline after opening bracket
        if self.peek() == Some(b'\n') {
            self.advance();
        } else if self.peek() == Some(b'\r') {
            self.advance();
            if self.peek() == Some(b'\n') {
                self.advance();
            }
        }

        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated long string")),
                Some(b']') => {
                    // Check for closing bracket
                    let mut eq_count = 0;
                    let mut offset = 1;
                    while self.peek_at(offset) == Some(b'=') {
                        eq_count += 1;
                        offset += 1;
                    }
                    if eq_count == level && self.peek_at(offset) == Some(b']') {
                        // Found closing bracket
                        self.advance(); // ]
                        for _ in 0..level {
                            self.advance(); // =
                        }
                        self.advance(); // ]
                        return Ok(s);
                    }
                    s.push(self.advance().unwrap() as char);
                }
                Some(ch) => {
                    self.advance();
                    if ch == b'\r' {
                        s.push('\n');
                        if self.peek() == Some(b'\n') {
                            self.advance();
                        }
                    } else {
                        s.push(ch as char);
                    }
                }
            }
        }
    }

    fn read_short_string(&mut self, quote: u8) -> Result<String, LexError> {
        self.advance(); // skip opening quote
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => return Err(self.err("unterminated string")),
                Some(ch) if ch == quote => {
                    self.advance();
                    return Ok(s);
                }
                Some(b'\\') => {
                    self.advance(); // skip backslash
                    match self.peek() {
                        None => return Err(self.err("unterminated string escape")),
                        Some(b'a') => {
                            self.advance();
                            s.push('\x07');
                        }
                        Some(b'b') => {
                            self.advance();
                            s.push('\x08');
                        }
                        Some(b'f') => {
                            self.advance();
                            s.push('\x0c');
                        }
                        Some(b'n') => {
                            self.advance();
                            s.push('\n');
                        }
                        Some(b'r') => {
                            self.advance();
                            s.push('\r');
                        }
                        Some(b't') => {
                            self.advance();
                            s.push('\t');
                        }
                        Some(b'v') => {
                            self.advance();
                            s.push('\x0b');
                        }
                        Some(b'\\') => {
                            self.advance();
                            s.push('\\');
                        }
                        Some(b'\'') => {
                            self.advance();
                            s.push('\'');
                        }
                        Some(b'"') => {
                            self.advance();
                            s.push('"');
                        }
                        Some(b'\n') => {
                            self.advance();
                            s.push('\n');
                        }
                        Some(b'\r') => {
                            self.advance();
                            s.push('\n');
                            if self.peek() == Some(b'\n') {
                                self.advance();
                            }
                        }
                        Some(ch) if ch.is_ascii_digit() => {
                            // Decimal escape \ddd (1-3 digits)
                            let mut val: u32 = (ch - b'0') as u32;
                            self.advance();
                            for _ in 0..2 {
                                if let Some(d) = self.peek() {
                                    if d.is_ascii_digit() {
                                        val = val * 10 + (d - b'0') as u32;
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                            if val > 255 {
                                return Err(self.err("escape sequence too large"));
                            }
                            s.push(val as u8 as char);
                        }
                        Some(ch) => {
                            return Err(
                                self.err(format!("invalid escape sequence '\\{}'", ch as char))
                            );
                        }
                    }
                }
                Some(ch) => {
                    self.advance();
                    s.push(ch as char);
                }
            }
        }
    }

    fn read_number(&mut self) -> Result<f64, LexError> {
        let start = self.pos;

        // Check for hex
        if self.peek() == Some(b'0') && matches!(self.peek_at(1), Some(b'x' | b'X')) {
            self.advance(); // 0
            self.advance(); // x/X
            if !matches!(self.peek(), Some(ch) if ch.is_ascii_hexdigit()) {
                return Err(self.err("malformed hex number"));
            }
            while matches!(self.peek(), Some(ch) if ch.is_ascii_hexdigit()) {
                self.advance();
            }
            let hex_str = std::str::from_utf8(&self.source[start..self.pos]).unwrap();
            return i64::from_str_radix(&hex_str[2..], 16)
                .map(|n| n as f64)
                .map_err(|_| self.err("malformed hex number"));
        }

        // Decimal integer or float
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
        }

        // Fractional part
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance(); // .
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.advance();
            }
        } else if self.peek() == Some(b'.') && !matches!(self.peek_at(1), Some(b'.')) {
            // Allow "1." as a valid number
            self.advance();
        }

        // Exponent
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            if !matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                return Err(self.err("malformed number: expected exponent digits"));
            }
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.advance();
            }
        }

        let num_str = std::str::from_utf8(&self.source[start..self.pos]).unwrap();
        num_str
            .parse::<f64>()
            .map_err(|_| self.err(format!("malformed number: {num_str}")))
    }

    fn read_name(&mut self) -> String {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.source[start..self.pos])
            .unwrap()
            .to_owned()
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments()?;

        let span = self.span();

        let ch = match self.peek() {
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    span,
                });
            }
            Some(ch) => ch,
        };

        // String literals
        if ch == b'"' || ch == b'\'' {
            let s = self.read_short_string(ch)?;
            return Ok(Token {
                kind: TokenKind::StringLit(s),
                span,
            });
        }

        // Long strings
        if ch == b'['
            && let Some(level) = self.check_long_bracket_open()
        {
            let s = self.read_long_string(level)?;
            return Ok(Token {
                kind: TokenKind::StringLit(s),
                span,
            });
        }

        // Numbers
        if ch.is_ascii_digit()
            || (ch == b'.' && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()))
        {
            let n = self.read_number()?;
            return Ok(Token {
                kind: TokenKind::Number(n),
                span,
            });
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let name = self.read_name();
            let kind = TokenKind::from_keyword(&name).unwrap_or(TokenKind::Name(name));
            return Ok(Token { kind, span });
        }

        // Operators and punctuation
        self.advance();
        let kind = match ch {
            b'+' => TokenKind::Plus,
            b'*' => TokenKind::Star,
            b'^' => TokenKind::Caret,
            b'%' => TokenKind::Percent,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b']' => TokenKind::RBracket,
            b';' => TokenKind::Semicolon,
            b':' => TokenKind::Colon,
            b',' => TokenKind::Comma,
            b'#' => TokenKind::Hash,
            b'-' => TokenKind::Minus,
            b'/' => TokenKind::Slash,
            b'[' => TokenKind::LBracket,
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Eq
                } else {
                    TokenKind::Assign
                }
            }
            b'~' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::Neq
                } else {
                    return Err(self.err("unexpected character '~'"));
                }
            }
            b'.' => {
                if self.peek() == Some(b'.') {
                    self.advance();
                    if self.peek() == Some(b'.') {
                        self.advance();
                        TokenKind::DotDotDot
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            _ => return Err(self.err(format!("unexpected character '{}'", ch as char))),
        };

        Ok(Token { kind, span })
    }

    /// Collect all tokens until EOF.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            if tok.kind == TokenKind::Eof {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        Ok(tokens)
    }

    /// Get current position (for parser lookahead).
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Get source bytes (for parser lookahead).
    pub fn source(&self) -> &[u8] {
        self.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("lex error");
        tokens.into_iter().map(|t| t.kind).collect()
    }

    fn lex_one(source: &str) -> TokenKind {
        let kinds = lex(source);
        assert_eq!(kinds.len(), 2); // token + EOF
        kinds[0].clone()
    }

    #[test]
    fn keywords() {
        assert_eq!(lex_one("and"), TokenKind::And);
        assert_eq!(lex_one("break"), TokenKind::Break);
        assert_eq!(lex_one("do"), TokenKind::Do);
        assert_eq!(lex_one("else"), TokenKind::Else);
        assert_eq!(lex_one("elseif"), TokenKind::Elseif);
        assert_eq!(lex_one("end"), TokenKind::End);
        assert_eq!(lex_one("false"), TokenKind::False);
        assert_eq!(lex_one("for"), TokenKind::For);
        assert_eq!(lex_one("function"), TokenKind::Function);
        assert_eq!(lex_one("if"), TokenKind::If);
        assert_eq!(lex_one("in"), TokenKind::In);
        assert_eq!(lex_one("local"), TokenKind::Local);
        assert_eq!(lex_one("nil"), TokenKind::Nil);
        assert_eq!(lex_one("not"), TokenKind::Not);
        assert_eq!(lex_one("or"), TokenKind::Or);
        assert_eq!(lex_one("repeat"), TokenKind::Repeat);
        assert_eq!(lex_one("return"), TokenKind::Return);
        assert_eq!(lex_one("then"), TokenKind::Then);
        assert_eq!(lex_one("true"), TokenKind::True);
        assert_eq!(lex_one("until"), TokenKind::Until);
        assert_eq!(lex_one("while"), TokenKind::While);
    }

    #[test]
    fn identifiers() {
        assert_eq!(lex_one("hello"), TokenKind::Name("hello".into()));
        assert_eq!(lex_one("_foo"), TokenKind::Name("_foo".into()));
        assert_eq!(lex_one("x123"), TokenKind::Name("x123".into()));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn number_literals() {
        assert_eq!(lex_one("42"), TokenKind::Number(42.0));
        assert_eq!(lex_one("3.14"), TokenKind::Number(3.14));
        assert_eq!(lex_one("1e10"), TokenKind::Number(1e10));
        assert_eq!(lex_one("1.5e-3"), TokenKind::Number(1.5e-3));
        assert_eq!(lex_one("0xff"), TokenKind::Number(255.0));
        assert_eq!(lex_one("0xFF"), TokenKind::Number(255.0));
        assert_eq!(lex_one("0x1A"), TokenKind::Number(26.0));
        assert_eq!(lex_one(".5"), TokenKind::Number(0.5));
    }

    #[test]
    fn string_literals() {
        assert_eq!(lex_one("\"hello\""), TokenKind::StringLit("hello".into()));
        assert_eq!(lex_one("'hello'"), TokenKind::StringLit("hello".into()));
        assert_eq!(
            lex_one("\"tab\\there\""),
            TokenKind::StringLit("tab\there".into())
        );
        assert_eq!(
            lex_one("\"newline\\n\""),
            TokenKind::StringLit("newline\n".into())
        );
        assert_eq!(
            lex_one("\"escaped\\\\\""),
            TokenKind::StringLit("escaped\\".into())
        );
        assert_eq!(
            lex_one("\"quote\\\"\""),
            TokenKind::StringLit("quote\"".into())
        );
        assert_eq!(lex_one("\"dec\\065\""), TokenKind::StringLit("decA".into()));
    }

    #[test]
    fn long_strings() {
        assert_eq!(lex_one("[[hello]]"), TokenKind::StringLit("hello".into()));
        assert_eq!(lex_one("[=[hello]=]"), TokenKind::StringLit("hello".into()));
        assert_eq!(
            lex_one("[==[hello]==]"),
            TokenKind::StringLit("hello".into())
        );
        assert_eq!(
            lex_one("[[line1\nline2]]"),
            TokenKind::StringLit("line1\nline2".into())
        );
    }

    #[test]
    fn operators() {
        assert_eq!(lex_one("+"), TokenKind::Plus);
        assert_eq!(lex_one("-"), TokenKind::Minus);
        assert_eq!(lex_one("*"), TokenKind::Star);
        assert_eq!(lex_one("/"), TokenKind::Slash);
        assert_eq!(lex_one("%"), TokenKind::Percent);
        assert_eq!(lex_one("^"), TokenKind::Caret);
        assert_eq!(lex_one("#"), TokenKind::Hash);
        assert_eq!(lex_one("=="), TokenKind::Eq);
        assert_eq!(lex_one("~="), TokenKind::Neq);
        assert_eq!(lex_one("<"), TokenKind::Lt);
        assert_eq!(lex_one("<="), TokenKind::Le);
        assert_eq!(lex_one(">"), TokenKind::Gt);
        assert_eq!(lex_one(">="), TokenKind::Ge);
        assert_eq!(lex_one("="), TokenKind::Assign);
        assert_eq!(lex_one(".."), TokenKind::DotDot);
        assert_eq!(lex_one("..."), TokenKind::DotDotDot);
        assert_eq!(lex_one("."), TokenKind::Dot);
    }

    #[test]
    fn comments_skipped() {
        assert_eq!(
            lex("-- comment\n42"),
            vec![TokenKind::Number(42.0), TokenKind::Eof]
        );
        assert_eq!(
            lex("--[[long comment]]42"),
            vec![TokenKind::Number(42.0), TokenKind::Eof]
        );
        assert_eq!(
            lex("--[=[long]=]42"),
            vec![TokenKind::Number(42.0), TokenKind::Eof]
        );
    }

    #[test]
    fn local_assignment() {
        let tokens = lex("local x = 1 + 2");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Local,
                TokenKind::Name("x".into()),
                TokenKind::Assign,
                TokenKind::Number(1.0),
                TokenKind::Plus,
                TokenKind::Number(2.0),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unterminated_string_error() {
        let mut lexer = Lexer::new("\"oops");
        assert!(lexer.next_token().is_err());
    }

    #[test]
    fn unterminated_long_string_error() {
        let mut lexer = Lexer::new("[[oops");
        assert!(lexer.next_token().is_err());
    }

    #[test]
    fn line_tracking() {
        let mut lexer = Lexer::new("a\nb\nc");
        let t1 = lexer.next_token().unwrap();
        assert_eq!(t1.span.line, 1);
        let t2 = lexer.next_token().unwrap();
        assert_eq!(t2.span.line, 2);
        let t3 = lexer.next_token().unwrap();
        assert_eq!(t3.span.line, 3);
    }
}
