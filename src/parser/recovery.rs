use std::collections::HashMap;

use crate::{
    lexer::{Token, TokenKind},
    parser::ast::{
        ErrorSeverity, Flow, FlowType, GatewayBranch, GatewayType, ParseError, ProcessElement,
        TaskType,
    },
};

pub struct ErrorRecovery {
    pub recovered_elements: Vec<ProcessElement>,
    pub recovered_flows: Vec<Flow>,
    pub errors: Vec<ParseError>,
}

impl ErrorRecovery {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            recovered_elements: Vec::new(),
            recovered_flows: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn recover_process_element(
        &mut self,
        tokens: &[Token],
        start_pos: usize,
    ) -> Option<(ProcessElement, usize)> {
        if start_pos >= tokens.len() {
            return None;
        }

        let token = &tokens[start_pos];
        let span = token.span.clone();

        match &token.kind {
            TokenKind::Start => {
                let element = ProcessElement::StartEvent {
                    id: None,
                    event_type: None,
                    attributes: std::collections::HashMap::new(),
                    span,
                };
                Some((element, start_pos + 1))
            }
            TokenKind::End => {
                let element = ProcessElement::EndEvent {
                    id: None,
                    event_type: None,
                    attributes: std::collections::HashMap::new(),
                    span,
                };
                Some((element, start_pos + 1))
            }
            TokenKind::Task | TokenKind::User | TokenKind::Service | TokenKind::Script => {
                self.recover_task(tokens, start_pos)
            }
            TokenKind::Xor | TokenKind::And => self.recover_gateway(tokens, start_pos),
            _ => {
                self.errors.push(ParseError {
                    message: format!("Cannot recover from token '{}'", token.text),
                    span,
                    severity: ErrorSeverity::Error,
                });
                None
            }
        }
    }

    fn recover_task(
        &mut self,
        tokens: &[Token],
        start_pos: usize,
    ) -> Option<(ProcessElement, usize)> {
        let mut pos = start_pos;
        let span = tokens[pos].span.clone();

        let task_type = match &tokens[pos].kind {
            TokenKind::Task => TaskType::Generic,
            TokenKind::User => TaskType::User,
            TokenKind::Service => TaskType::Service,
            TokenKind::Script => TaskType::Script,
            _ => return None,
        };

        pos += 1;

        let id = if pos < tokens.len() && tokens[pos].kind == TokenKind::Identifier {
            let id = tokens[pos].text.clone();
            pos += 1;
            id
        } else {
            self.errors.push(ParseError {
                message: "Missing task identifier, using default".to_string(),
                span: span.clone(),
                severity: ErrorSeverity::Warning,
            });
            format!("Task_{start_pos}")
        };

        pos = self.skip_malformed_attributes(tokens, pos);

        let element = ProcessElement::Task {
            id,
            task_type,
            attributes: HashMap::new(),
            span,
        };

        Some((element, pos))
    }

    fn recover_gateway(
        &mut self,
        tokens: &[Token],
        start_pos: usize,
    ) -> Option<(ProcessElement, usize)> {
        let mut pos = start_pos;
        let span = tokens[pos].span.clone();

        let gateway_type = match &tokens[pos].kind {
            TokenKind::Xor => GatewayType::Exclusive,
            TokenKind::And => GatewayType::Parallel,
            _ => return None,
        };

        pos += 1;

        let id = if pos < tokens.len() && tokens[pos].kind == TokenKind::Identifier {
            let id = tokens[pos].text.clone();
            pos += 1;
            Some(id)
        } else {
            None
        };

        if pos < tokens.len() && tokens[pos].kind == TokenKind::Question {
            pos += 1;
        }

        let branches = if pos < tokens.len() && tokens[pos].kind == TokenKind::LeftBrace {
            pos += 1;
            let (recovered_branches, new_pos) = self.recover_gateway_branches(tokens, pos);
            pos = new_pos;

            if pos < tokens.len() && tokens[pos].kind == TokenKind::RightBrace {
                pos += 1;
            }

            recovered_branches
        } else {
            self.errors.push(ParseError {
                message: "Gateway missing branches block".to_string(),
                span: span.clone(),
                severity: ErrorSeverity::Error,
            });
            Vec::new()
        };

        let element = ProcessElement::Gateway {
            id,
            gateway_type,
            branches,
            span,
        };

        Some((element, pos))
    }

    fn recover_gateway_branches(
        &mut self,
        tokens: &[Token],
        start_pos: usize,
    ) -> (Vec<GatewayBranch>, usize) {
        let mut branches = Vec::new();
        let mut pos = start_pos;

        while pos < tokens.len() && tokens[pos].kind != TokenKind::RightBrace {
            if let Some((branch, new_pos)) = self.recover_single_branch(tokens, pos) {
                branches.push(branch);
                pos = new_pos;
            } else {
                pos += 1;
            }
        }

        (branches, pos)
    }

    fn recover_single_branch(
        &mut self,
        tokens: &[Token],
        start_pos: usize,
    ) -> Option<(GatewayBranch, usize)> {
        let mut pos = start_pos;
        let span = tokens[pos].span.clone();

        let (condition, is_default) = if tokens[pos].kind == TokenKind::LeftBracket {
            pos += 1;
            let mut cond = String::new();
            while pos < tokens.len() && tokens[pos].kind != TokenKind::RightBracket {
                if !cond.is_empty() {
                    cond.push(' ');
                }
                cond.push_str(&tokens[pos].text);
                pos += 1;
            }
            if pos < tokens.len() {
                pos += 1;
            }
            (Some(cond), false)
        } else if tokens[pos].kind == TokenKind::DefaultFlow {
            (None, true)
        } else if tokens[pos].kind == TokenKind::Identifier {
            let cond = tokens[pos].text.clone();
            pos += 1;
            (Some(cond), false)
        } else {
            return None;
        };

        if pos >= tokens.len()
            || (!matches!(
                tokens[pos].kind,
                TokenKind::SequenceFlow | TokenKind::DefaultFlow
            ))
        {
            self.errors.push(ParseError {
                message: "Missing arrow in gateway branch".to_string(),
                span,
                severity: ErrorSeverity::Error,
            });
            return None;
        }
        pos += 1;

        let target = if pos < tokens.len() && tokens[pos].kind == TokenKind::Identifier {
            let target = tokens[pos].text.clone();
            pos += 1;
            target
        } else {
            self.errors.push(ParseError {
                message: "Missing target in gateway branch".to_string(),
                span: span.clone(),
                severity: ErrorSeverity::Error,
            });
            format!("UnknownTarget_{pos}")
        };

        let branch = GatewayBranch {
            condition,
            target,
            is_default,
            span,
        };

        Some((branch, pos))
    }

    fn skip_malformed_attributes(&mut self, tokens: &[Token], start_pos: usize) -> usize {
        let mut pos = start_pos;

        while pos < tokens.len() && tokens[pos].kind == TokenKind::At {
            pos += 1;
            while pos < tokens.len()
                && !matches!(
                    tokens[pos].kind,
                    TokenKind::At
                        | TokenKind::LeftParen
                        | TokenKind::Start
                        | TokenKind::End
                        | TokenKind::Task
                        | TokenKind::User
                        | TokenKind::Service
                        | TokenKind::Script
                        | TokenKind::Xor
                        | TokenKind::And
                        | TokenKind::RightBrace
                )
            {
                pos += 1;
            }
        }

        if pos < tokens.len() && tokens[pos].kind == TokenKind::LeftParen {
            pos += 1;
            let mut paren_count = 1;
            while pos < tokens.len() && paren_count > 0 {
                match tokens[pos].kind {
                    TokenKind::LeftParen => paren_count += 1,
                    TokenKind::RightParen => paren_count -= 1,
                    _ => {}
                }
                pos += 1;
            }
        }

        pos
    }

    pub fn recover_flow(&mut self, tokens: &[Token], start_pos: usize) -> Option<(Flow, usize)> {
        let mut pos = start_pos;

        let from = if pos < tokens.len() && tokens[pos].kind == TokenKind::Identifier {
            let from = tokens[pos].text.clone();
            pos += 1;
            from
        } else {
            return None;
        };

        let flow_type = if pos < tokens.len() {
            match tokens[pos].kind {
                TokenKind::SequenceFlow => {
                    pos += 1;
                    FlowType::Sequence
                }
                TokenKind::MessageFlow => {
                    pos += 1;
                    FlowType::Message
                }
                TokenKind::DefaultFlow => {
                    pos += 1;
                    FlowType::Default
                }
                TokenKind::Association => {
                    pos += 1;
                    FlowType::Association
                }
                _ => return None,
            }
        } else {
            return None;
        };

        let to = if pos < tokens.len() && tokens[pos].kind == TokenKind::Identifier {
            let to = tokens[pos].text.clone();
            pos += 1;
            to
        } else {
            self.errors.push(ParseError {
                message: "Missing target in flow".to_string(),
                span: tokens[start_pos].span.clone(),
                severity: ErrorSeverity::Error,
            });
            format!("UnknownTarget_{pos}")
        };

        let condition = if pos < tokens.len() && tokens[pos].kind == TokenKind::LeftBracket {
            pos += 1;
            let mut cond = String::new();
            while pos < tokens.len() && tokens[pos].kind != TokenKind::RightBracket {
                if !cond.is_empty() {
                    cond.push(' ');
                }
                cond.push_str(&tokens[pos].text);
                pos += 1;
            }
            if pos < tokens.len() {
                pos += 1;
            }
            Some(cond)
        } else {
            None
        };

        let flow = Flow {
            from,
            to,
            flow_type,
            condition,
            span: tokens[start_pos].span.clone(),
        };

        Some((flow, pos))
    }

    #[must_use]
    pub fn find_sync_point(&self, tokens: &[Token], start_pos: usize) -> usize {
        let mut pos = start_pos;

        while pos < tokens.len() {
            match tokens[pos].kind {
                TokenKind::RightBrace => return pos + 1,

                TokenKind::Start
                | TokenKind::End
                | TokenKind::Task
                | TokenKind::User
                | TokenKind::Service
                | TokenKind::Script
                | TokenKind::Xor
                | TokenKind::And
                | TokenKind::Event
                | TokenKind::Process
                | TokenKind::Import
                | TokenKind::Subprocess
                | TokenKind::Pool
                | TokenKind::Lane => return pos,

                _ => pos += 1,
            }
        }

        pos
    }
}

impl Default for ErrorRecovery {
    fn default() -> Self {
        Self::new()
    }
}
