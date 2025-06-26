pub mod multi_file;

use std::{
    fmt,
    path::{Path, PathBuf},
};

use logos::Logos;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: String,
}

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
#[logos(skip r"[ \t\f]+")]
pub enum TokenKind {
    // Keywords
    #[token("process")]
    Process,
    #[token("import")]
    Import,
    #[token("from")]
    From,
    #[token("as")]
    As,
    #[token("subprocess")]
    Subprocess,
    // BPMN Elements
    #[token("start")]
    Start,
    #[token("end")]
    End,
    #[token("task")]
    Task,
    #[token("user")]
    User,
    #[token("service")]
    Service,
    #[token("script")]
    Script,
    #[token("call")]
    Call,
    #[token("xor")]
    Xor,
    #[token("and")]
    And,
    #[token("event")]
    Event,
    #[token("group")]
    Group,
    #[token("pool")]
    Pool,
    #[token("lane")]
    Lane,
    #[token("note")]
    Note,
    // Flow arrows
    #[token("->")]
    SequenceFlow,
    #[token("-->")]
    MessageFlow,
    #[token("=>")]
    DefaultFlow,
    #[token("..>")]
    Association,
    #[token("::")]
    Namespace,
    // Brackets and delimiters
    #[token("{", priority = 2)]
    LeftBrace,
    #[token("}", priority = 2)]
    RightBrace,
    #[token("(", priority = 2)]
    LeftParen,
    #[token(")", priority = 2)]
    RightParen,
    #[token("[", priority = 2)]
    LeftBracket,
    #[token("]", priority = 2)]
    RightBracket,
    #[token(",", priority = 2)]
    Comma,
    #[token("=", priority = 2)]
    Equals,
    #[token("@", priority = 2)]
    At,
    #[token("?", priority = 2)]
    Question,
    // Literals
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,
    #[regex(r"[0-9]+(\.[0-9]+)?[a-zA-Z]*")]
    NumberLiteral,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    // Comments
    #[regex(r"//[^\n]*")]
    LineComment,
    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    BlockComment,
    // Whitespace and newlines
    #[token("\n")]
    Newline,
    #[token("\r\n")]
    CarriageReturnNewline,
    // Error recovery
    #[regex(r".", priority = 1)]
    Unknown,
    // End of file
    Eof,
}

pub struct Lexer<'a> {
    input: &'a str,
    logos: logos::Lexer<'a, TokenKind>,
    line: usize,
    column: usize,
    file_path: PathBuf,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str, file_path: impl AsRef<Path>) -> Self {
        Self {
            input,
            logos: TokenKind::lexer(input),
            line: 1,
            column: 1,
            file_path: file_path.as_ref().to_path_buf(),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while let Some(token_kind) = self.logos.next() {
            let span = self.logos.span();
            let text = self.input[span.clone()].to_string();
            let (line, column) = self.calculate_position(span.start);
            let token = Token {
                kind: token_kind.unwrap_or(TokenKind::Unknown),
                span: Span {
                    start: span.start,
                    end: span.end,
                    line,
                    column,
                    file: self.file_path.clone(),
                },
                text,
            };

            if matches!(
                token.kind,
                TokenKind::Newline | TokenKind::CarriageReturnNewline
            ) {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += token.span.end - token.span.start;
            }

            tokens.push(token);
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: self.input.len(),
                end: self.input.len(),
                line: self.line,
                column: self.column,
                file: self.file_path.clone(),
            },
            text: String::new(),
        });

        tokens
    }

    fn calculate_position(&self, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;

        for (i, ch) in self.input.char_indices() {
            if i >= pos {
                break;
            }

            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        (line, column)
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Process => write!(f, "process"),
            Self::Import => write!(f, "import"),
            Self::Identifier => write!(f, "identifier"),
            Self::StringLiteral => write!(f, "string"),
            Self::SequenceFlow => write!(f, "->"),
            Self::Unknown => write!(f, "unknown token"),
            _ => write!(f, "{self:?}"),
        }
    }
}
