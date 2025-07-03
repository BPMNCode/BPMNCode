use super::suggestions::{detect_keyword_typo, is_likely_keyword_typo};
use super::{DiagnosticError, Severity};
use crate::lexer::{Span, Token, TokenKind};

pub struct ContextValidator {
    errors: Vec<DiagnosticError>,
    #[allow(dead_code)]
    source_code: String,
}

impl ContextValidator {
    #[must_use]
    pub const fn new(source_code: String) -> Self {
        Self {
            errors: Vec::new(),
            source_code,
        }
    }

    pub fn validate_tokens(&mut self, tokens: &[Token]) -> Vec<DiagnosticError> {
        self.errors.clear();

        for (i, token) in tokens.iter().enumerate() {
            match &token.kind {
                TokenKind::Identifier => {
                    self.check_identifier_typo(token, tokens, i);
                }
                TokenKind::Unknown => {
                    self.check_unknown_token(token);
                }
                _ => {}
            }
        }

        self.check_flow_syntax(tokens);
        self.check_missing_braces(tokens);

        self.errors.clone()
    }

    fn check_identifier_typo(&mut self, token: &Token, tokens: &[Token], index: usize) {
        let identifier = &token.text;

        if self.is_contextual_identifier(tokens, index) {
            return;
        }

        if self.is_statement_start(tokens, index) {
            if let Some(suggestion) = detect_keyword_typo(identifier) {
                self.errors.push(DiagnosticError::UnexpectedToken {
                    found: identifier.clone(),
                    expected: format!("keyword (did you mean '{suggestion}'?)"),
                    span: token.span.clone(),
                    suggestions: vec![suggestion],
                });
            } else if is_likely_keyword_typo(identifier) {
                let suggestions = super::suggestions::suggest_keywords(identifier);
                self.errors.push(DiagnosticError::UnexpectedToken {
                    found: identifier.clone(),
                    expected: "BPMN keyword".to_string(),
                    span: token.span.clone(),
                    suggestions,
                });
            }
        }
    }

    fn check_unknown_token(&mut self, token: &Token) {
        if matches!(token.text.as_str(), "<" | ">" | "=" | "!" | "&" | "|") {
            return;
        }

        self.errors.push(DiagnosticError::SyntaxError {
            message: format!("Unknown token '{}'", token.text),
            span: token.span.clone(),
            severity: Severity::Error,
            suggestions: Vec::new(),
        });
    }

    fn check_flow_syntax(&mut self, tokens: &[Token]) {
        for (i, token) in tokens.iter().enumerate() {
            if token.text == "-" {
                if let Some(next_token) = tokens.get(i + 1) {
                    if next_token.text == ">" {
                        continue;
                    }
                }

                if self.looks_like_flow_context(tokens, i) {
                    self.errors.push(DiagnosticError::SyntaxError {
                        message: "Invalid flow operator: use '->' for sequence flow".to_string(),
                        span: token.span.clone(),
                        severity: Severity::Error,
                        suggestions: vec!["->".to_string()],
                    });
                }
            }
        }
    }

    fn check_missing_braces(&mut self, tokens: &[Token]) {
        for (i, token) in tokens.iter().enumerate() {
            if matches!(token.kind, TokenKind::Xor | TokenKind::And) {
                self.check_gateway_braces(tokens, i);
            }
        }
    }

    fn check_gateway_braces(&mut self, tokens: &[Token], gateway_index: usize) {
        let token = &tokens[gateway_index];
        let gateway_type = if matches!(token.kind, TokenKind::Xor) {
            "XOR"
        } else {
            "AND"
        };

        let mut j = gateway_index + 1;
        let mut gateway_name_end = token.span.end;

        if let Some(next) = tokens.get(j) {
            if matches!(next.kind, TokenKind::Identifier) {
                gateway_name_end = next.span.end;
                j += 1;
            }
        }

        if let Some(next) = tokens.get(j) {
            if matches!(next.kind, TokenKind::Question) {
                gateway_name_end = next.span.end;
                j += 1;
            }
        }

        let gateway_span = Span {
            start: token.span.start,
            end: gateway_name_end,
            line: token.span.line,
            column: token.span.column,
            file: token.span.file.clone(),
        };

        let has_opening_brace = self
            .find_next_significant_token(tokens, j)
            .is_some_and(|idx| matches!(tokens[idx].kind, TokenKind::LeftBrace));

        if has_opening_brace {
            if let Some(open_idx) = self.find_next_significant_token(tokens, j) {
                if let Some(_close_idx) = self.find_gateway_closing_brace(tokens, open_idx) {
                } else {
                    self.errors.push(DiagnosticError::SyntaxError {
                        message: format!("{gateway_type} gateway missing closing brace '}}'"),
                        span: gateway_span,
                        severity: Severity::Error,
                        suggestions: vec!["}".to_string()],
                    });
                }
            }
        } else if self.has_gateway_conditions_ahead(tokens, j) {
            self.errors.push(DiagnosticError::SyntaxError {
                message: format!(
                    "{gateway_type} gateway missing opening brace '{{' before conditions"
                ),
                span: gateway_span,
                severity: Severity::Error,
                suggestions: vec!["{".to_string()],
            });
        }
    }

    #[allow(clippy::needless_continue)]
    #[allow(clippy::needless_range_loop)]
    #[allow(clippy::unused_self)]
    fn find_next_significant_token(&self, tokens: &[Token], start: usize) -> Option<usize> {
        for i in start..tokens.len() {
            match tokens[i].kind {
                TokenKind::Newline
                | TokenKind::CarriageReturnNewline
                | TokenKind::LineComment
                | TokenKind::BlockComment => continue,
                _ => return Some(i),
            }
        }
        None
    }

    #[allow(clippy::needless_continue)]
    #[allow(clippy::needless_range_loop)]
    #[allow(clippy::unused_self)]
    fn has_gateway_conditions_ahead(&self, tokens: &[Token], start: usize) -> bool {
        for i in start..tokens.len().min(start + 10) {
            match tokens[i].kind {
                TokenKind::LeftBracket | TokenKind::DefaultFlow => return true,
                TokenKind::RightBrace => return false,
                TokenKind::Newline
                | TokenKind::CarriageReturnNewline
                | TokenKind::LineComment
                | TokenKind::BlockComment => continue,
                _ => {}
            }
        }
        false
    }

    #[allow(clippy::needless_range_loop)]
    #[allow(clippy::unused_self)]
    fn find_gateway_closing_brace(&self, tokens: &[Token], open_idx: usize) -> Option<usize> {
        if !matches!(tokens[open_idx].kind, TokenKind::LeftBrace) {
            return None;
        }

        let mut brace_count = 1;
        let mut found_gateway_content = false;

        for i in (open_idx + 1)..tokens.len() {
            match tokens[i].kind {
                TokenKind::LeftBrace => brace_count += 1,
                TokenKind::RightBrace => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return if found_gateway_content { Some(i) } else { None };
                    }
                }
                TokenKind::LeftBracket | TokenKind::DefaultFlow | TokenKind::SequenceFlow => {
                    if brace_count == 1 {
                        found_gateway_content = true;
                    }
                }
                TokenKind::Xor
                | TokenKind::And
                | TokenKind::Task
                | TokenKind::User
                | TokenKind::Service
                | TokenKind::Script
                | TokenKind::End => {
                    if brace_count == 1 {
                        return None;
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[allow(clippy::unused_self)]
    fn is_contextual_identifier(&self, tokens: &[Token], index: usize) -> bool {
        if let Some(next) = tokens.get(index + 1) {
            if matches!(next.kind, TokenKind::LeftParen) {
                return true;
            }
        }

        if index > 0 {
            if let Some(prev) = tokens.get(index - 1) {
                if matches!(
                    prev.kind,
                    TokenKind::SequenceFlow
                        | TokenKind::MessageFlow
                        | TokenKind::DefaultFlow
                        | TokenKind::Association
                ) {
                    return true;
                }
            }
        }

        if let Some(next) = tokens.get(index + 1) {
            if matches!(
                next.kind,
                TokenKind::SequenceFlow
                    | TokenKind::MessageFlow
                    | TokenKind::DefaultFlow
                    | TokenKind::Association
            ) {
                return true;
            }
        }

        if let Some(next) = tokens.get(index + 1) {
            if next.text == "-" {
                return true;
            }
        }

        false
    }

    #[allow(clippy::needless_continue)]
    #[allow(clippy::unused_self)]
    fn is_statement_start(&self, tokens: &[Token], index: usize) -> bool {
        if index == 0 {
            return true;
        }

        for i in (0..index).rev() {
            match &tokens[i].kind {
                TokenKind::LeftBrace | TokenKind::RightBrace | TokenKind::Newline => return true,
                TokenKind::Identifier | TokenKind::StringLiteral | TokenKind::NumberLiteral => {
                    continue;
                }
                _ => return false,
            }
        }

        false
    }

    #[allow(clippy::unused_self)]
    fn looks_like_flow_context(&self, tokens: &[Token], index: usize) -> bool {
        if index > 0 {
            if let Some(prev) = tokens.get(index - 1) {
                if matches!(prev.kind, TokenKind::Identifier) {
                    if let Some(next) = tokens.get(index + 1) {
                        if matches!(next.kind, TokenKind::Identifier) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}
