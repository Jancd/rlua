#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    StringLit(String),
    Name(String),

    // Keywords
    And,
    Break,
    Do,
    Else,
    Elseif,
    End,
    False,
    For,
    Function,
    If,
    In,
    Local,
    Nil,
    Not,
    Or,
    Repeat,
    Return,
    Then,
    True,
    Until,
    While,

    // Operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Hash,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Colon,
    Comma,
    Dot,
    DotDot,
    DotDotDot,
    Assign,

    Eof,
}

impl TokenKind {
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s {
            "and" => Some(Self::And),
            "break" => Some(Self::Break),
            "do" => Some(Self::Do),
            "else" => Some(Self::Else),
            "elseif" => Some(Self::Elseif),
            "end" => Some(Self::End),
            "false" => Some(Self::False),
            "for" => Some(Self::For),
            "function" => Some(Self::Function),
            "if" => Some(Self::If),
            "in" => Some(Self::In),
            "local" => Some(Self::Local),
            "nil" => Some(Self::Nil),
            "not" => Some(Self::Not),
            "or" => Some(Self::Or),
            "repeat" => Some(Self::Repeat),
            "return" => Some(Self::Return),
            "then" => Some(Self::Then),
            "true" => Some(Self::True),
            "until" => Some(Self::Until),
            "while" => Some(Self::While),
            _ => None,
        }
    }
}
