use std::collections::HashMap;

use crate::{
    lexer::Span,
    parser::ast::{
        AttributeValue, EventType, Flow, FlowType, GatewayBranch, GatewayType, ProcessDeclaration,
        ProcessElement, TaskType,
    },
};

pub struct AstBuilder {
    current_process: Option<ProcessDeclaration>,
}

impl AstBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current_process: None,
        }
    }

    pub fn start_process(&mut self, name: String, span: Span) -> &mut Self {
        self.current_process = Some(ProcessDeclaration {
            name,
            attributes: HashMap::new(),
            elements: Vec::new(),
            flows: Vec::new(),
            span,
        });

        self
    }

    pub fn add_process_atribute(&mut self, key: String, value: AttributeValue) -> &mut Self {
        if let Some(ref mut process) = self.current_process {
            process.attributes.insert(key, value);
        }

        self
    }

    pub fn add_element(&mut self, element: ProcessElement) -> &mut Self {
        if let Some(ref mut process) = self.current_process {
            process.elements.push(element);
        }

        self
    }

    pub fn add_flow(&mut self, flow: Flow) -> &mut Self {
        if let Some(ref mut process) = self.current_process {
            process.flows.push(flow);
        }

        self
    }

    pub const fn finish_process(&mut self) -> Option<ProcessDeclaration> {
        self.current_process.take()
    }

    #[must_use]
    pub const fn create_start_event(
        &self,
        id: Option<String>,
        event_type: Option<EventType>,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    ) -> ProcessElement {
        ProcessElement::StartEvent {
            id,
            event_type,
            attributes,
            span,
        }
    }

    #[must_use]
    pub const fn create_task(
        &self,
        id: String,
        task_type: TaskType,
        attributes: HashMap<String, AttributeValue>,
        span: Span,
    ) -> ProcessElement {
        ProcessElement::Task {
            id,
            task_type,
            attributes,
            span,
        }
    }

    #[must_use]
    pub const fn create_gateway(
        &self,
        id: Option<String>,
        gateway_type: GatewayType,
        branches: Vec<GatewayBranch>,
        span: Span,
    ) -> ProcessElement {
        ProcessElement::Gateway {
            id,
            gateway_type,
            branches,
            span,
        }
    }

    #[must_use]
    pub const fn create_flow(
        &self,
        from: String,
        to: String,
        flow_type: FlowType,
        condition: Option<String>,
        span: Span,
    ) -> Flow {
        Flow {
            from,
            to,
            flow_type,
            condition,
            span,
        }
    }
}

impl Default for AstBuilder {
    fn default() -> Self {
        Self::new()
    }
}
