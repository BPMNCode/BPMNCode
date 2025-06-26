use thiserror::Error;

use crate::lexer::Span;

#[derive(Error, Debug)]
pub enum LexerError {
    #[error("Unexpected character '{character}' at {span}")]
    UnexpectedCharacter { character: char, span: Span },

    #[error("Unterminated string literal at {span}")]
    UnterminatedString { span: Span },

    #[error("Unterminated block comment at {span}")]
    UnterminatedComment { span: Span },

    #[error("Invalid number format '{text}' at {span}")]
    InvalidNumber { text: String, span: Span },
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)
    }
}
