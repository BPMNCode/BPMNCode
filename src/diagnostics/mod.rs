use crate::lexer::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub mod formatter;
pub mod suggestions;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticError {
    #[error("Syntax error: {message}")]
    SyntaxError {
        message: String,
        #[serde(flatten)]
        span: Span,
        severity: Severity,
        suggestions: Vec<String>,
    },

    #[error("Unexpected token '{found}', expected {expected}")]
    UnexpectedToken {
        found: String,
        expected: String,
        #[serde(flatten)]
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("Undefined reference '{name}'")]
    UndefinedReference {
        name: String,
        #[serde(flatten)]
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("Duplicate identifier '{name}'")]
    DuplicateIdentifier {
        name: String,
        #[serde(flatten)]
        span: Span,
        first_definition: Option<Span>,
    },

    #[error("Invalid attribute '{attribute}' for element '{element}'")]
    InvalidAttribute {
        attribute: String,
        element: String,
        #[serde(flatten)]
        span: Span,
        valid_attributes: Vec<String>,
    },

    #[error("Missing required element '{element}'")]
    MissingElement {
        element: String,
        #[serde(flatten)]
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("Invalid flow: {message}")]
    InvalidFlow {
        message: String,
        #[serde(flatten)]
        span: Span,
        suggestions: Vec<String>,
    },

    #[error("Import error: {message}")]
    ImportError {
        message: String,
        #[serde(flatten)]
        span: Span,
        path: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Hint => write!(f, "hint"),
        }
    }
}

impl DiagnosticError {
    #[must_use]
    pub const fn span(&self) -> &Span {
        match self {
            Self::SyntaxError { span, .. }
            | Self::UnexpectedToken { span, .. }
            | Self::UndefinedReference { span, .. }
            | Self::DuplicateIdentifier { span, .. }
            | Self::InvalidAttribute { span, .. }
            | Self::MissingElement { span, .. }
            | Self::InvalidFlow { span, .. }
            | Self::ImportError { span, .. } => span,
        }
    }

    #[must_use]
    pub const fn severity(&self) -> Severity {
        match self {
            Self::SyntaxError { severity, .. } => *severity,
            Self::UnexpectedToken { .. }
            | Self::UndefinedReference { .. }
            | Self::DuplicateIdentifier { .. }
            | Self::InvalidAttribute { .. }
            | Self::MissingElement { .. }
            | Self::InvalidFlow { .. }
            | Self::ImportError { .. } => Severity::Error,
        }
    }

    #[must_use]
    pub fn suggestions(&self) -> &[String] {
        match self {
            Self::SyntaxError { suggestions, .. }
            | Self::UnexpectedToken { suggestions, .. }
            | Self::UndefinedReference { suggestions, .. }
            | Self::MissingElement { suggestions, .. }
            | Self::InvalidFlow { suggestions, .. } => suggestions,
            Self::InvalidAttribute {
                valid_attributes, ..
            } => valid_attributes,
            Self::DuplicateIdentifier { .. } | Self::ImportError { .. } => &[],
        }
    }

    #[must_use]
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        match &mut self {
            Self::SyntaxError { suggestions, .. }
            | Self::UnexpectedToken { suggestions, .. }
            | Self::UndefinedReference { suggestions, .. }
            | Self::MissingElement { suggestions, .. }
            | Self::InvalidFlow { suggestions, .. } => {
                suggestions.push(suggestion);
            }
            _ => {}
        }
        self
    }

    #[must_use]
    pub fn with_suggestions(mut self, new_suggestions: Vec<String>) -> Self {
        match &mut self {
            Self::SyntaxError { suggestions, .. }
            | Self::UnexpectedToken { suggestions, .. }
            | Self::UndefinedReference { suggestions, .. }
            | Self::MissingElement { suggestions, .. }
            | Self::InvalidFlow { suggestions, .. } => {
                suggestions.extend(new_suggestions);
            }
            Self::InvalidAttribute {
                valid_attributes, ..
            } => {
                valid_attributes.extend(new_suggestions);
            }
            _ => {}
        }
        self
    }
}

impl Diagnostic for DiagnosticError {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        None
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span = self.span();
        Some(Box::new(std::iter::once(miette::LabeledSpan::new(
            Some(self.to_string()),
            span.start,
            span.end - span.start,
        ))))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        let suggestions = self.suggestions();
        if suggestions.is_empty() {
            return None;
        }

        Some(Box::new(format!(
            "Did you mean: {}?",
            suggestions.join(", ")
        )))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.severity() {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Info | Severity::Hint => miette::Severity::Advice,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub errors: Vec<DiagnosticError>,
    pub file_path: String,
    pub source_code: String,
}

impl DiagnosticReport {
    #[must_use]
    pub const fn new(file_path: String, source_code: String) -> Self {
        Self {
            errors: Vec::new(),
            file_path,
            source_code,
        }
    }

    pub fn add_error(&mut self, error: DiagnosticError) {
        self.errors.push(error);
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.errors
            .iter()
            .any(|e| matches!(e.severity(), Severity::Error))
    }

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.errors
            .iter()
            .filter(|e| matches!(e.severity(), Severity::Error))
            .count()
    }

    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.errors
            .iter()
            .filter(|e| matches!(e.severity(), Severity::Warning))
            .count()
    }
}
