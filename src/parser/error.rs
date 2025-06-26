use thiserror::Error;

use crate::lexer::Span;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    #[error("Unexpected token '{found}' at {span}, expected {expected}")]
    UnexpectedToken {
        found: String,
        expected: String,
        span: Span,
    },

    #[error("Missing closing brace for block starting at {start_span}")]
    UnclosedBlock {
        start_span: Span,
        current_span: Span,
    },

    #[error("Invalid attribute value '{value}' at {span}")]
    InvalidAttributeValue { value: String, span: Span },

    #[error("Duplicate element ID '{id}' at {span}, first defined at {first_span}")]
    DuplicateId {
        id: String,
        span: Span,
        first_span: Span,
    },

    #[error("Undefined reference '{reference}' at {span}")]
    UndefinedReference { reference: String, span: Span },

    #[error("Invalid flow: {message} at {span}")]
    InvalidFlow { message: String, span: Span },

    #[error("Unexpected end of input, expected {expected}")]
    UnexpectedEof { expected: String, span: Span },
}

impl ParserError {
    #[must_use]
    pub const fn span(&self) -> &Span {
        match self {
            Self::UnclosedBlock { current_span, .. } => current_span,
            Self::InvalidAttributeValue { span, .. }
            | Self::UnexpectedToken { span, .. }
            | Self::DuplicateId { span, .. }
            | Self::UndefinedReference { span, .. }
            | Self::InvalidFlow { span, .. }
            | Self::UnexpectedEof { span, .. } => span,
        }
    }
}
