use std::collections::HashMap;

use crate::{
    lexer::{Span, Token, TokenKind},
    parser::{
        ast::{
            AstDocument, AttributeValue, ErrorSeverity, EventType, Flow, FlowType, GatewayBranch,
            GatewayType, ImportDeclaration, Lane, ParseError, ProcessDeclaration, ProcessElement,
            TaskType,
        },
        error::ParserError,
        recovery::ErrorRecovery,
    },
};

pub mod ast;
pub mod builder;
pub mod error;
pub mod recovery;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    #[must_use]
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse_with_recovery(&mut self) -> AstDocument {
        let mut document = AstDocument::new();
        let mut recovery = ErrorRecovery::new();

        self.skip_whitespace_and_comments();

        while self.check_token(&TokenKind::Import) {
            match self.parse_import() {
                Ok(import) => document.imports.push(import),
                Err(err) => {
                    document.add_error(err.to_string(), self.current_span());

                    let sync_pos = recovery.find_sync_point(&self.tokens, self.position);
                    self.position = sync_pos;
                }
            }
            self.skip_whitespace_and_comments();
        }

        while self.check_token(&TokenKind::Process) {
            match self.parse_process_with_recovery(&mut recovery) {
                Ok(process) => document.processes.push(process),
                Err(err) => {
                    document.add_error(err.to_string(), self.current_span());

                    let sync_pos = recovery.find_sync_point(&self.tokens, self.position);
                    self.position = sync_pos;
                }
            }
            self.skip_whitespace_and_comments();
        }

        for error in recovery.errors {
            document.errors.push(error);
        }

        if !self.is_at_end() && !self.check_token(&TokenKind::Eof) {
            document.add_error(
                format!("Unexpected token '{}'", self.current_token().text),
                self.current_span(),
            );
        }

        document
    }

    fn parse_process_with_recovery(
        &mut self,
        recovery: &mut ErrorRecovery,
    ) -> Result<ProcessDeclaration, Box<ParserError>> {
        let start_span = self.current_span();
        self.consume_token(&TokenKind::Process)?;

        let name = self.parse_identifier()?;
        let attributes = self.parse_attributes().unwrap_or_default();

        self.consume_token(&TokenKind::LeftBrace)?;

        let mut elements = Vec::new();
        let mut flows = Vec::new();

        self.skip_whitespace_and_comments();

        while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
            let current_pos = self.position;

            if let Ok(element) = self.parse_process_element() {
                elements.push(element);
            } else {
                self.position = current_pos;
                if let Ok(flow) = self.parse_flow() {
                    flows.push(flow);
                } else {
                    self.position = current_pos;

                    if let Some((recovered_element, new_pos)) =
                        recovery.recover_process_element(&self.tokens, self.position)
                    {
                        elements.push(recovered_element);
                        self.position = new_pos;
                    } else if let Some((recovered_flow, new_pos)) =
                        recovery.recover_flow(&self.tokens, self.position)
                    {
                        flows.push(recovered_flow);
                        self.position = new_pos;
                    } else {
                        recovery.errors.push(ParseError {
                            message: format!(
                                "Skipping unexpected token '{}'",
                                self.current_token().text
                            ),
                            span: self.current_span(),
                            severity: ErrorSeverity::Warning,
                        });
                        self.advance();
                    }
                }
            }

            self.skip_whitespace_and_comments();
        }

        if self.check_token(&TokenKind::RightBrace) {
            self.advance();
        } else {
            recovery.errors.push(ParseError {
                message: "Missing closing brace for process".to_string(),
                span: self.current_span(),
                severity: ErrorSeverity::Error,
            });
        }

        Ok(ProcessDeclaration {
            name,
            attributes,
            elements,
            flows,
            span: start_span,
        })
    }

    pub fn parse(&mut self) -> AstDocument {
        let mut document = AstDocument::new();

        self.skip_whitespace_and_comments();

        while self.check_token(&TokenKind::Import) {
            match self.parse_import() {
                Ok(import) => document.imports.push(import),
                Err(err) => {
                    document.add_error(err.to_string(), self.current_span());

                    self.recover_to_next_statement();
                }
            }
            self.skip_whitespace_and_comments();
        }

        while self.check_token(&TokenKind::Process) {
            match self.parse_process() {
                Ok(process) => document.processes.push(process),
                Err(err) => {
                    document.add_error(err.to_string(), self.current_span());

                    self.recover_to_next_statement();
                }
            }

            self.skip_whitespace_and_comments();
        }

        if !self.is_at_end() && !self.check_token(&TokenKind::Eof) {
            document.add_error(
                format!("Unexpected token '{}'", self.current_token().text),
                self.current_span(),
            );
        }

        document
    }

    fn parse_import(&mut self) -> Result<ImportDeclaration, Box<ParserError>> {
        let start_span = self.current_span();

        self.consume_token(&TokenKind::Import)?;

        if self.check_token(&TokenKind::StringLiteral) {
            let path = self.parse_string_literal()?;

            let alias = if self.check_token(&TokenKind::As) {
                self.advance();
                Some(self.parse_identifier()?)
            } else {
                None
            };

            return Ok(ImportDeclaration {
                path,
                alias,
                items: Vec::new(),
                span: start_span,
            });
        }

        let mut items = Vec::new();

        while !self.check_token(&TokenKind::From) && !self.is_at_end() {
            if self.check_token(&TokenKind::Identifier) {
                items.push(self.parse_identifier()?);
            } else {
                self.advance();
            }

            if self.check_token(&TokenKind::Comma) {
                self.advance();
            } else if !self.check_token(&TokenKind::From) {
                break;
            }
        }

        self.consume_token(&TokenKind::From)?;
        let path = self.parse_string_literal()?;

        Ok(ImportDeclaration {
            path,
            alias: None,
            items,
            span: start_span,
        })
    }

    fn parse_process(&mut self) -> Result<ProcessDeclaration, Box<ParserError>> {
        let start_span = self.current_span();
        self.consume_token(&TokenKind::Process)?;

        let name = self.parse_identifier()?;
        let attributes = self.parse_attributes()?;

        self.consume_token(&TokenKind::LeftBrace)?;

        let mut elements = Vec::new();
        let mut flows = Vec::new();

        self.skip_whitespace_and_comments();

        while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
            if let Ok(element) = self.parse_process_element() {
                elements.push(element);
            } else if let Ok(flow) = self.parse_flow() {
                flows.push(flow);
            } else {
                self.advance();
            }

            self.skip_whitespace_and_comments();
        }

        self.consume_token(&TokenKind::RightBrace)?;

        let process = ProcessDeclaration {
            name,
            attributes,
            elements,
            flows,
            span: start_span,
        };

        Ok(process)
    }

    #[allow(clippy::too_many_lines)]
    fn parse_process_element(&mut self) -> Result<ProcessElement, Box<ParserError>> {
        let span = self.current_span();

        match &self.current_token().kind {
            TokenKind::Start => {
                self.advance();
                let event_type = self.parse_event_type()?;
                let attributes = self.parse_attributes()?;

                Ok(ProcessElement::StartEvent {
                    id: None,
                    event_type,
                    attributes,
                    span,
                })
            }
            TokenKind::End => {
                self.advance();
                let event_type = self.parse_event_type()?;
                let attributes = self.parse_attributes()?;

                Ok(ProcessElement::EndEvent {
                    id: None,
                    event_type,
                    attributes,
                    span,
                })
            }
            TokenKind::Task => {
                self.advance();
                let id = self.parse_identifier()?;
                let attributes = self.parse_attributes()?;

                let task = ProcessElement::Task {
                    id,
                    task_type: TaskType::Generic,
                    attributes,
                    span,
                };

                Ok(task)
            }
            TokenKind::User => {
                self.advance();
                let id = self.parse_identifier()?;
                let attributes = self.parse_attributes()?;

                let task = ProcessElement::Task {
                    id,
                    task_type: TaskType::User,
                    attributes,
                    span,
                };

                Ok(task)
            }
            TokenKind::Service => {
                self.advance();
                let id = self.parse_identifier()?;
                let attributes = self.parse_attributes()?;

                Ok(ProcessElement::Task {
                    id,
                    task_type: TaskType::Service,
                    attributes,
                    span,
                })
            }
            TokenKind::Script => {
                self.advance();
                let id = self.parse_identifier()?;
                let attributes = self.parse_attributes()?;

                Ok(ProcessElement::Task {
                    id,
                    task_type: TaskType::Script,
                    attributes,
                    span,
                })
            }
            TokenKind::Call => {
                self.advance();
                let id = self.parse_identifier()?;
                let called_element = if self.check_token(&TokenKind::Namespace) {
                    self.advance();
                    format!("{}::{}", id, self.parse_identifier()?)
                } else {
                    id.clone()
                };
                let attributes = self.parse_attributes()?;

                Ok(ProcessElement::CallActivity {
                    id,
                    called_element,
                    attributes,
                    span,
                })
            }
            TokenKind::Xor => {
                self.advance();
                let id = if self.check_token(&TokenKind::Identifier) {
                    Some(self.parse_identifier()?)
                } else {
                    None
                };

                if self.check_token(&TokenKind::Question) {
                    self.advance();
                }

                self.consume_token(&TokenKind::LeftBrace)?;
                let branches = self.parse_gateway_branches()?;
                self.consume_token(&TokenKind::RightBrace)?;

                Ok(ProcessElement::Gateway {
                    id,
                    gateway_type: GatewayType::Exclusive,
                    branches,
                    span,
                })
            }
            TokenKind::And => {
                self.advance();
                let id = if self.check_token(&TokenKind::Identifier) {
                    Some(self.parse_identifier()?)
                } else {
                    None
                };

                self.consume_token(&TokenKind::LeftBrace)?;
                let branches = self.parse_gateway_branches()?;
                self.consume_token(&TokenKind::RightBrace)?;

                Ok(ProcessElement::Gateway {
                    id,
                    gateway_type: GatewayType::Parallel,
                    branches,
                    span,
                })
            }
            TokenKind::Event => {
                self.advance();
                let event_type =
                    self.parse_event_type()?
                        .ok_or_else(|| ParserError::UnexpectedToken {
                            found: self.current_token().text,
                            expected: "event type (timer, message, etc.)".to_string(),
                            span: self.current_span(),
                        })?;

                let payload = if self.check_token(&TokenKind::StringLiteral)
                    || self.check_token(&TokenKind::NumberLiteral)
                    || self.check_token(&TokenKind::Identifier)
                {
                    Some(self.current_token().text)
                } else {
                    None
                };

                if payload.is_some() {
                    self.advance();
                }

                let attributes = self.parse_attributes()?;

                Ok(ProcessElement::IntermediateEvent {
                    id: None,
                    event_type,
                    payload,
                    attributes,
                    span,
                })
            }
            TokenKind::Subprocess => {
                self.advance();
                let id = self.parse_identifier()?;
                let attributes = self.parse_attributes()?;

                self.consume_token(&TokenKind::LeftBrace)?;

                let mut elements = Vec::new();
                let mut flows = Vec::new();

                self.skip_whitespace_and_comments();

                while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
                    if let Ok(element) = self.parse_process_element() {
                        elements.push(element);
                    } else if let Ok(flow) = self.parse_flow() {
                        flows.push(flow);
                    } else {
                        self.advance();
                    }
                    self.skip_whitespace_and_comments();
                }

                self.consume_token(&TokenKind::RightBrace)?;

                Ok(ProcessElement::Subprocess {
                    id,
                    elements,
                    flows,
                    attributes,
                    span,
                })
            }
            TokenKind::Pool => {
                self.advance();
                let name = self.parse_identifier()?;

                self.consume_token(&TokenKind::LeftBrace)?;

                let mut lanes = Vec::new();
                let mut elements = Vec::new();
                let mut flows = Vec::new();

                self.skip_whitespace_and_comments();

                while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
                    if self.check_token(&TokenKind::Lane) {
                        lanes.push(self.parse_lane()?);
                    } else if let Ok(element) = self.parse_process_element() {
                        elements.push(element);
                    } else if let Ok(flow) = self.parse_flow() {
                        flows.push(flow);
                    } else {
                        self.advance();
                    }
                    self.skip_whitespace_and_comments();
                }

                self.consume_token(&TokenKind::RightBrace)?;

                Ok(ProcessElement::Pool {
                    name,
                    lanes,
                    elements,
                    flows,
                    span,
                })
            }
            TokenKind::Group => {
                self.advance();
                let label = self.parse_string_literal()?;

                self.consume_token(&TokenKind::LeftBrace)?;

                let mut elements = Vec::new();
                self.skip_whitespace_and_comments();

                while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
                    if let Ok(element) = self.parse_process_element() {
                        elements.push(element);
                    } else {
                        self.advance();
                    }
                    self.skip_whitespace_and_comments();
                }

                self.consume_token(&TokenKind::RightBrace)?;

                Ok(ProcessElement::Group {
                    label,
                    elements,
                    span,
                })
            }
            TokenKind::Note => {
                self.advance();
                let text = self.parse_string_literal()?;

                Ok(ProcessElement::Annotation { text, span })
            }
            _ => Err(Box::new(ParserError::UnexpectedToken {
                found: self.current_token().text,
                expected: "process element".to_string(),
                span: self.current_span(),
            })),
        }
    }

    fn parse_flow(&mut self) -> Result<Flow, Box<ParserError>> {
        let span = self.current_span();
        let from = self.parse_identifier()?;

        let flow_type = match &self.current_token().kind {
            TokenKind::SequenceFlow => {
                self.advance();
                FlowType::Sequence
            }
            TokenKind::MessageFlow => {
                self.advance();
                FlowType::Message
            }
            TokenKind::DefaultFlow => {
                self.advance();
                FlowType::Default
            }
            TokenKind::Association => {
                self.advance();
                FlowType::Association
            }
            _ => {
                return Err(Box::new(ParserError::UnexpectedToken {
                    found: self.current_token().text,
                    expected: "flow arrow (-> --> => ..>)".to_string(),
                    span: self.current_span(),
                }));
            }
        };

        let to = if self.check_token(&TokenKind::End) {
            self.advance();
            "end".to_string()
        } else {
            self.parse_identifier()?
        };

        let condition = if self.check_token(&TokenKind::LeftBracket) {
            self.advance();
            let cond = self.parse_condition_expression()?;
            self.consume_token(&TokenKind::RightBracket)?;
            Some(cond)
        } else {
            None
        };

        Ok(Flow {
            from,
            to,
            flow_type,
            condition,
            span,
        })
    }

    fn parse_gateway_branches(&mut self) -> Result<Vec<GatewayBranch>, Box<ParserError>> {
        let mut branches = Vec::new();

        self.skip_whitespace_and_comments();

        while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
            let span = self.current_span();

            let (condition, is_default) = if self.check_token(&TokenKind::LeftBracket) {
                self.advance();
                let cond = self.parse_condition_expression()?;
                self.consume_token(&TokenKind::RightBracket)?;
                (Some(cond), false)
            } else if self.check_token(&TokenKind::DefaultFlow) {
                self.advance();
                (None, true)
            } else {
                let cond = self.parse_identifier()?;
                (Some(cond), false)
            };

            if !self.check_token(&TokenKind::SequenceFlow)
                && !self.check_token(&TokenKind::DefaultFlow)
            {
                return Err(Box::new(ParserError::UnexpectedToken {
                    found: self.current_token().text,
                    expected: "-> or =>".to_string(),
                    span: self.current_span(),
                }));
            }

            self.advance();

            let target = self.parse_identifier()?;

            branches.push(GatewayBranch {
                condition,
                target,
                is_default,
                span,
            });

            self.skip_whitespace_and_comments();
        }

        Ok(branches)
    }

    fn parse_lane(&mut self) -> Result<Lane, Box<ParserError>> {
        let span = self.current_span();
        self.consume_token(&TokenKind::Lane)?;
        let name = self.parse_identifier()?;

        self.consume_token(&TokenKind::LeftBrace)?;

        let mut elements = Vec::new();
        self.skip_whitespace_and_comments();

        while !self.check_token(&TokenKind::RightBrace) && !self.is_at_end() {
            if let Ok(element) = self.parse_process_element() {
                elements.push(element);
            } else {
                self.advance();
            }
            self.skip_whitespace_and_comments();
        }

        self.consume_token(&TokenKind::RightBrace)?;

        Ok(Lane {
            name,
            elements,
            span,
        })
    }

    fn parse_event_type(&mut self) -> Result<Option<EventType>, Box<ParserError>> {
        if !self.check_token(&TokenKind::At) {
            return Ok(None);
        }

        self.advance();

        if !self.check_token(&TokenKind::Identifier) {
            return Err(Box::new(ParserError::UnexpectedToken {
                found: self.current_token().text,
                expected: "event type identifier".to_string(),
                span: self.current_span(),
            }));
        }

        let event_type_name = self.current_token().text;
        self.advance();

        match event_type_name.as_str() {
            "message" => {
                let payload = if self.check_token(&TokenKind::StringLiteral) {
                    self.parse_string_literal()?
                } else {
                    String::new()
                };
                Ok(Some(EventType::Message(payload)))
            }
            "timer" => {
                let duration = if self.check_token(&TokenKind::NumberLiteral)
                    || self.check_token(&TokenKind::Identifier)
                {
                    let dur = self.current_token().text;
                    self.advance();
                    dur
                } else {
                    String::new()
                };
                Ok(Some(EventType::Timer(duration)))
            }
            "error" => {
                let error_code = if self.check_token(&TokenKind::StringLiteral) {
                    self.parse_string_literal()?
                } else {
                    String::new()
                };
                Ok(Some(EventType::Error(error_code)))
            }
            "signal" => {
                let signal_name = if self.check_token(&TokenKind::StringLiteral) {
                    self.parse_string_literal()?
                } else {
                    String::new()
                };
                Ok(Some(EventType::Signal(signal_name)))
            }
            "terminate" => Ok(Some(EventType::Terminate)),
            _ => Err(Box::new(ParserError::UnexpectedToken {
                found: event_type_name,
                expected: "event type (message, timer, error, signal, terminate)".to_string(),
                span: self.current_span(),
            })),
        }
    }

    fn parse_attributes(&mut self) -> Result<HashMap<String, AttributeValue>, Box<ParserError>> {
        let mut attributes = HashMap::new();

        while self.check_token(&TokenKind::At) {
            self.advance();
            let key = self.parse_identifier()?;

            let value = if self.check_token(&TokenKind::StringLiteral)
                || self.check_token(&TokenKind::NumberLiteral)
                || self.check_token(&TokenKind::Identifier)
            {
                self.parse_attribute_value()?
            } else {
                AttributeValue::Boolean(true)
            };
            attributes.insert(key, value);
        }

        if self.check_token(&TokenKind::LeftParen) {
            self.advance();
            self.skip_whitespace_and_comments();

            while !self.check_token(&TokenKind::RightParen) && !self.is_at_end() {
                let key = self.parse_identifier()?;

                if !self.check_token(&TokenKind::Equals) {
                    return Err(Box::new(ParserError::UnexpectedToken {
                        found: self.current_token().text,
                        expected: "=".to_string(),
                        span: self.current_span(),
                    }));
                }
                self.advance();

                let value = self.parse_attribute_value()?;

                attributes.insert(key.clone(), value);
                self.skip_whitespace_and_comments();

                if self.check_token(&TokenKind::Comma) {
                    self.advance();
                    self.skip_whitespace_and_comments();
                } else if !self.check_token(&TokenKind::RightParen) {
                    break;
                }
            }

            if self.check_token(&TokenKind::RightParen) {
                self.advance();
            } else {
                return Err(Box::new(ParserError::UnexpectedToken {
                    found: self.current_token().text,
                    expected: ")".to_string(),
                    span: self.current_span(),
                }));
            }
        }

        Ok(attributes)
    }

    fn parse_attribute_value(&mut self) -> Result<AttributeValue, Box<ParserError>> {
        match &self.current_token().kind {
            TokenKind::StringLiteral => {
                let value = self.parse_string_literal()?;
                Ok(AttributeValue::String(value))
            }
            TokenKind::NumberLiteral => {
                let text = self.current_token().text;
                self.advance();

                if text.ends_with('m')
                    || text.ends_with('s')
                    || text.ends_with("ms")
                    || text.ends_with('h')
                {
                    Ok(AttributeValue::Duration(text))
                } else if let Ok(num) = text.parse::<f64>() {
                    Ok(AttributeValue::Number(num))
                } else {
                    Err(Box::new(ParserError::InvalidAttributeValue {
                        value: text,
                        span: self.current_span(),
                    }))
                }
            }
            TokenKind::Identifier => {
                let text = self.current_token().text;
                self.advance();

                match text.as_str() {
                    "true" => Ok(AttributeValue::Boolean(true)),
                    "false" => Ok(AttributeValue::Boolean(false)),
                    _ => Ok(AttributeValue::String(text)),
                }
            }
            _ => Err(Box::new(ParserError::UnexpectedToken {
                found: self.current_token().text,
                expected: "attribute value (string, number, boolean)".to_string(),
                span: self.current_span(),
            })),
        }
    }

    fn parse_condition_expression(&mut self) -> Result<String, Box<ParserError>> {
        let mut condition = String::new();
        let mut token_count = 0;

        while !self.check_token(&TokenKind::RightBracket) && !self.is_at_end() && token_count < 50 {
            if !condition.is_empty() {
                let current_text = &self.current_token().text;
                if !matches!(current_text.as_str(), "=" | "!" | "<" | ">" | "&" | "|") {
                    condition.push(' ');
                }
            }
            condition.push_str(&self.current_token().text);
            self.advance();
            token_count += 1;
        }

        if condition.is_empty() {
            return Err(Box::new(ParserError::UnexpectedToken {
                found: "]".to_string(),
                expected: "condition expression".to_string(),
                span: self.current_span(),
            }));
        }

        Ok(condition)
    }

    fn parse_identifier(&mut self) -> Result<String, Box<ParserError>> {
        if !self.check_token(&TokenKind::Identifier) {
            return Err(Box::new(ParserError::UnexpectedToken {
                found: self.current_token().text,
                expected: "identifier".to_string(),
                span: self.current_span(),
            }));
        }

        let identifier = self.current_token().text;
        self.advance();
        Ok(identifier)
    }

    fn parse_string_literal(&mut self) -> Result<String, Box<ParserError>> {
        if !self.check_token(&TokenKind::StringLiteral) {
            return Err(Box::new(ParserError::UnexpectedToken {
                found: self.current_token().text,
                expected: "string literal".to_string(),
                span: self.current_span(),
            }));
        }

        let mut literal = self.current_token().text;
        self.advance();

        if literal.len() >= 2 && literal.starts_with('"') && literal.ends_with('"') {
            literal = literal[1..literal.len() - 1].to_string();
            literal = literal.replace("\\\"", "\"");
            literal = literal.replace("\\\\", "\\");
            literal = literal.replace("\\n", "\n");
            literal = literal.replace("\\t", "\t");
        }

        Ok(literal)
    }

    fn current_token(&self) -> Token {
        self.tokens
            .get(self.position)
            .cloned()
            .unwrap_or_else(|| Token {
                kind: TokenKind::Eof,
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                    file: std::path::PathBuf::new(),
                },
                text: String::new(),
            })
    }

    fn current_span(&self) -> Span {
        self.current_token().span
    }

    fn check_token(&self, kind: &TokenKind) -> bool {
        &self.current_token().kind == kind
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.current_token()
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || matches!(self.current_token().kind, TokenKind::Eof)
    }

    fn consume_token(&mut self, expected: &TokenKind) -> Result<Token, Box<ParserError>> {
        if self.check_token(expected) {
            Ok(self.advance())
        } else {
            Err(Box::new(ParserError::UnexpectedToken {
                found: self.current_token().text,
                expected: format!("{expected:?}"),
                span: self.current_span(),
            }))
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while matches!(
            self.current_token().kind,
            TokenKind::Newline
                | TokenKind::CarriageReturnNewline
                | TokenKind::LineComment
                | TokenKind::BlockComment
        ) && !self.is_at_end()
        {
            self.advance();
        }
    }

    fn recover_to_next_statement(&mut self) {
        while !self.is_at_end() {
            match self.current_token().kind {
                TokenKind::Process | TokenKind::Import | TokenKind::RightBrace | TokenKind::Eof => {
                    break;
                }
                _ => self.advance(),
            };
        }
    }
}

#[must_use]
pub fn parse_tokens(tokens: Vec<Token>) -> AstDocument {
    let mut parser = Parser::new(tokens);

    parser.parse_with_recovery()
}
