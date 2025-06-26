use std::collections::HashMap;

use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct AstDocument {
    pub imports: Vec<ImportDeclaration>,
    pub processes: Vec<ProcessDeclaration>,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDeclaration {
    pub path: String,
    pub alias: Option<String>,
    pub items: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessDeclaration {
    pub name: String,
    pub attributes: HashMap<String, AttributeValue>,
    pub elements: Vec<ProcessElement>,
    pub flows: Vec<Flow>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessElement {
    StartEvent {
        id: Option<String>,
        event_type: Option<EventType>,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    },
    EndEvent {
        id: Option<String>,
        event_type: Option<EventType>,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    },
    Task {
        id: String,
        task_type: TaskType,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    },
    Gateway {
        id: Option<String>,
        gateway_type: GatewayType,
        branches: Vec<GatewayBranch>,
        span: Span,
    },
    IntermediateEvent {
        id: Option<String>,
        event_type: EventType,
        payload: Option<String>,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    },
    Subprocess {
        id: String,
        elements: Vec<ProcessElement>,
        flows: Vec<Flow>,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    },
    CallActivity {
        id: String,
        called_element: String,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    },
    Pool {
        name: String,
        lanes: Vec<Lane>,
        elements: Vec<ProcessElement>,
        flows: Vec<Flow>,
        span: Span,
    },
    Group {
        label: String,
        elements: Vec<ProcessElement>,
        span: Span,
    },
    Annotation {
        text: String,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    Generic,
    User,
    Service,
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayType {
    Exclusive,
    Parallel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayBranch {
    pub condition: Option<String>,
    pub target: String,
    pub is_default: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    Message(String),
    Timer(String),
    Error(String),
    Signal(String),
    Terminate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    pub name: String,
    pub elements: Vec<ProcessElement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub from: String,
    pub to: String,
    pub flow_type: FlowType,
    pub condition: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowType {
    Sequence,
    Message,
    Default,
    Association,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Duration(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
}

impl AstDocument {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            imports: Vec::new(),
            processes: Vec::new(),
            errors: Vec::new(),
        }
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.errors
            .iter()
            .any(|e| e.severity == ErrorSeverity::Error)
    }

    pub fn add_error(&mut self, message: String, span: Span) {
        self.errors.push(ParseError {
            message,
            span,
            severity: ErrorSeverity::Error,
        });
    }

    pub fn add_warnings(&mut self, message: String, span: Span) {
        self.errors.push(ParseError {
            message,
            span,
            severity: ErrorSeverity::Warning,
        });
    }
}

impl Default for AstDocument {
    fn default() -> Self {
        Self::new()
    }
}
