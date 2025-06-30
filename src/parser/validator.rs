use std::collections::HashMap;

use crate::{
    lexer::Span,
    parser::ast::{AstDocument, ErrorSeverity, Flow, FlowType, ParseError, ProcessElement},
};

pub type SyntaxError = ParseError;

pub type ValidationResult = Result<(), Vec<SyntaxError>>;

pub struct SyntaxValidator {
    errors: Vec<SyntaxError>,
}

impl SyntaxValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn validate(&mut self, document: &AstDocument) -> ValidationResult {
        self.errors.clear();

        for process in &document.processes {
            let mut node_ids = HashMap::new();

            for element in &process.elements {
                self.validate_element(element, &mut node_ids);
            }

            for flow in &process.flows {
                self.validate_flow(flow, &node_ids);
            }
        }

        self.validate_unknown_commands(document);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn validate_element(&mut self, element: &ProcessElement, node_ids: &mut HashMap<String, Span>) {
        let (id_opt, span) = match element {
            ProcessElement::Gateway { id, span, .. }
            | ProcessElement::EndEvent { id, span, .. }
            | ProcessElement::StartEvent { id, span, .. }
            | ProcessElement::IntermediateEvent { id, span, .. } => (id.as_ref(), span),
            ProcessElement::Subprocess {
                id, span, elements, ..
            } => {
                let mut nested_ids = HashMap::new();
                for nested_element in elements {
                    self.validate_element(nested_element, &mut nested_ids);
                }
                (Some(id), span)
            }
            ProcessElement::CallActivity { id, span, .. }
            | ProcessElement::Task { id, span, .. } => (Some(id), span),
            ProcessElement::Pool {
                name,
                span,
                elements,
                ..
            } => {
                let mut pool_ids = HashMap::new();
                for pool_element in elements {
                    self.validate_element(pool_element, &mut pool_ids);
                }
                (Some(name), span)
            }
            ProcessElement::Group { elements, span, .. } => {
                let mut group_ids = HashMap::new();
                for group_element in elements {
                    self.validate_element(group_element, &mut group_ids);
                }
                (None, span)
            }
            ProcessElement::Annotation { span, .. } => (None, span),
        };

        if let Some(id) = id_opt {
            if let Some(_first_span) = node_ids.get(id) {
                self.errors.push(SyntaxError {
                    message: format!("Duplicate node id '{id}'"),
                    span: span.clone(),
                    severity: ErrorSeverity::Error,
                });
            } else {
                node_ids.insert(id.clone(), span.clone());
            }
        }
    }

    fn validate_flow(&mut self, flow: &Flow, node_ids: &HashMap<String, Span>) {
        match flow.flow_type {
            FlowType::Sequence => {
                if !self.is_valid_sequence_flow(&flow.from, &flow.to, node_ids) {
                    self.errors.push(SyntaxError {
                        message: format!("Invalid sequential arrow: {} -> {}", flow.from, flow.to),
                        span: flow.span.clone(),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
            FlowType::Message => {
                if !self.is_valid_message_flow(&flow.from, &flow.to) {
                    self.errors.push(SyntaxError {
                        message: format!("Invalid message arrow: {} --> {}", flow.from, flow.to),
                        span: flow.span.clone(),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
            FlowType::Default => {
                if !self.is_valid_default_flow(&flow.from, node_ids) {
                    self.errors.push(SyntaxError {
                        message: format!(
                            "The default arrow can only come from the gateway: {} => {}",
                            flow.from, flow.to
                        ),
                        span: flow.span.clone(),
                        severity: ErrorSeverity::Error,
                    });
                }
            }
            FlowType::Association => {
                if !self.is_valid_association(&flow.from, &flow.to) {
                    self.errors.push(SyntaxError {
                        message: format!("Invalid associative link: {} ..> {}", flow.from, flow.to),
                        span: flow.span.clone(),
                        severity: ErrorSeverity::Warning,
                    });
                }
            }
        }

        if !node_ids.contains_key(&flow.from) && flow.from != "start" {
            self.errors.push(SyntaxError {
                message: format!("Unknown flow source: '{}'", flow.from),
                span: flow.span.clone(),
                severity: ErrorSeverity::Error,
            });
        }

        if !node_ids.contains_key(&flow.to) && flow.to != "end" {
            self.errors.push(SyntaxError {
                message: format!("Unknown flow target: '{}'", flow.to),
                span: flow.span.clone(),
                severity: ErrorSeverity::Error,
            });
        }
    }

    fn validate_unknown_commands(&mut self, document: &AstDocument) {
        for process in &document.processes {
            let has_start = process
                .elements
                .iter()
                .any(|element| matches!(element, ProcessElement::StartEvent { .. }));

            if !has_start {
                self.errors.push(SyntaxError {
                    message: format!(
                        "Process '{}' must contain at least one start event",
                        process.name
                    ),
                    span: process.span.clone(),
                    severity: ErrorSeverity::Warning,
                });
            }
        }
    }

    fn is_valid_sequence_flow(
        &self,
        from: &str,
        to: &str,
        _node_ids: &HashMap<String, Span>,
    ) -> bool {
        if from == "start" || to == "end" {
            return true;
        }

        true
    }

    const fn is_valid_message_flow(&self, _from: &str, _to: &str) -> bool {
        true
    }

    fn is_valid_default_flow(&self, from: &str, node_ids: &HashMap<String, Span>) -> bool {
        node_ids.contains_key(from) || from == "start"
    }

    const fn is_valid_association(&self, _from: &str, _to: &str) -> bool {
        true
    }
}

impl Default for SyntaxValidator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_syntax(document: &AstDocument) -> ValidationResult {
    let mut validator = SyntaxValidator::new();
    validator.validate(document)
}
