use std::collections::HashMap;
use std::path::PathBuf;

use bpmncode::{
    lexer::{Lexer, Span},
    parser::{
        ast::{
            AstDocument, ErrorSeverity, Flow, FlowType, ProcessDeclaration, ProcessElement,
            TaskType,
        },
        validator::validate_syntax,
    },
};

fn create_test_span() -> Span {
    Span {
        start: 0,
        end: 10,
        line: 1,
        column: 1,
        file: PathBuf::from("test.bpmn"),
    }
}

#[test]
fn test_duplicate_id_validation() {
    let span = create_test_span();

    let task1 = ProcessElement::Task {
        id: "task1".to_string(),
        task_type: TaskType::Generic,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let task2 = ProcessElement::Task {
        id: "task1".to_string(),
        task_type: TaskType::User,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let process = ProcessDeclaration {
        name: "TestProcess".to_string(),
        attributes: HashMap::new(),
        elements: vec![task1, task2],
        flows: vec![],
        span,
    };

    let document = AstDocument {
        imports: vec![],
        processes: vec![process],
        errors: vec![],
    };

    let result = validate_syntax(&document);
    assert!(result.is_err());

    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("Duplicate node id"));
    assert_eq!(errors[0].severity, ErrorSeverity::Error);
}

#[test]
fn test_invalid_flow_validation() {
    let span = create_test_span();

    let task1 = ProcessElement::Task {
        id: "task1".to_string(),
        task_type: TaskType::Generic,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    // Поток ведущий к несуществующему узлу
    let invalid_flow = Flow {
        from: "task1".to_string(),
        to: "nonexistent".to_string(),
        flow_type: FlowType::Sequence,
        condition: None,
        span: span.clone(),
    };

    let process = ProcessDeclaration {
        name: "TestProcess".to_string(),
        attributes: HashMap::new(),
        elements: vec![task1],
        flows: vec![invalid_flow],
        span,
    };

    let document = AstDocument {
        imports: vec![],
        processes: vec![process],
        errors: vec![],
    };

    let result = validate_syntax(&document);
    assert!(result.is_err());

    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("Unknown flow target"))
    );
}

#[test]
fn test_valid_document() {
    let span = create_test_span();

    let start = ProcessElement::StartEvent {
        id: None,
        event_type: None,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let task1 = ProcessElement::Task {
        id: "task1".to_string(),
        task_type: TaskType::Generic,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let end = ProcessElement::EndEvent {
        id: None,
        event_type: None,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let flow1 = Flow {
        from: "start".to_string(),
        to: "task1".to_string(),
        flow_type: FlowType::Sequence,
        condition: None,
        span: span.clone(),
    };

    let flow2 = Flow {
        from: "task1".to_string(),
        to: "end".to_string(),
        flow_type: FlowType::Sequence,
        condition: None,
        span: span.clone(),
    };

    let process = ProcessDeclaration {
        name: "ValidProcess".to_string(),
        attributes: HashMap::new(),
        elements: vec![start, task1, end],
        flows: vec![flow1, flow2],
        span,
    };

    let document = AstDocument {
        imports: vec![],
        processes: vec![process],
        errors: vec![],
    };

    let result = validate_syntax(&document);
    assert!(result.is_ok());
}

#[test]
fn test_missing_start_event_warning() {
    let span = create_test_span();

    let task1 = ProcessElement::Task {
        id: "task1".to_string(),
        task_type: TaskType::Generic,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let process = ProcessDeclaration {
        name: "ProcessWithoutStart".to_string(),
        attributes: HashMap::new(),
        elements: vec![task1],
        flows: vec![],
        span,
    };

    let document = AstDocument {
        imports: vec![],
        processes: vec![process],
        errors: vec![],
    };

    let result = validate_syntax(&document);
    assert!(result.is_err());

    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| {
        e.message.contains("must contain at least one start event")
            && e.severity == ErrorSeverity::Warning
    }));
}

#[test]
fn test_integration_with_lexer_and_parser() {
    let input = r"
        process TestProcess {
            start
            task ValidateData
            task ValidateData
            ValidateData -> end
        }
    ";

    let mut lexer = Lexer::new(input, "test.bpmn");
    let tokens = lexer.tokenize();

    let document = bpmncode::parser::parse_tokens_with_validation(tokens);

    assert!(document.has_errors());
    assert!(
        document
            .errors
            .iter()
            .any(|e| e.message.contains("Duplicate"))
    );
}

#[test]
fn test_complex_flow_validation() {
    let span = create_test_span();

    let start = ProcessElement::StartEvent {
        id: None,
        event_type: None,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let gateway = ProcessElement::Gateway {
        id: Some("decision".to_string()),
        gateway_type: bpmncode::parser::ast::GatewayType::Exclusive,
        branches: vec![],
        span: span.clone(),
    };

    let task1 = ProcessElement::Task {
        id: "approve".to_string(),
        task_type: TaskType::User,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    let task2 = ProcessElement::Task {
        id: "reject".to_string(),
        task_type: TaskType::User,
        attributes: HashMap::new(),
        span: span.clone(),
    };

    // Дефолтный поток от гейтвея (валидный)
    let default_flow = Flow {
        from: "decision".to_string(),
        to: "approve".to_string(),
        flow_type: FlowType::Default,
        condition: None,
        span: span.clone(),
    };

    // Обычный поток к второй задаче
    let conditional_flow = Flow {
        from: "decision".to_string(),
        to: "reject".to_string(),
        flow_type: FlowType::Sequence,
        condition: Some("amount > 1000".to_string()),
        span: span.clone(),
    };

    let process = ProcessDeclaration {
        name: "ApprovalProcess".to_string(),
        attributes: HashMap::new(),
        elements: vec![start, gateway, task1, task2],
        flows: vec![default_flow, conditional_flow],
        span,
    };

    let document = AstDocument {
        imports: vec![],
        processes: vec![process],
        errors: vec![],
    };

    let result = validate_syntax(&document);

    assert!(result.is_ok());
}
