#[cfg(test)]
mod tests {
    use bpmncode::lexer::Lexer;
    use bpmncode::parser::{ast::*, parse_tokens};

    fn parse_input(input: &str) -> AstDocument {
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();
        parse_tokens(tokens)
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let ast = parse_input(input);

        assert_eq!(ast.imports.len(), 0);
        assert_eq!(ast.processes.len(), 0);
        assert_eq!(ast.errors.len(), 0);
    }

    #[test]
    fn test_simple_process() {
        let input = r"
            process SimpleProcess {
                start
                task DoSomething
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );
        assert_eq!(ast.processes.len(), 1);

        let process = &ast.processes[0];
        assert_eq!(process.name, "SimpleProcess");
        assert_eq!(process.elements.len(), 3);

        // Проверяем элементы
        match &process.elements[0] {
            ProcessElement::StartEvent { id, .. } => assert!(id.is_none()),
            _ => panic!("Expected StartEvent"),
        }

        match &process.elements[1] {
            ProcessElement::Task { id, task_type, .. } => {
                assert_eq!(id, "DoSomething");
                assert_eq!(*task_type, TaskType::Generic);
            }
            _ => panic!("Expected Task"),
        }

        match &process.elements[2] {
            ProcessElement::EndEvent { id, .. } => assert!(id.is_none()),
            _ => panic!("Expected EndEvent"),
        }
    }

    #[test]
    fn test_process_with_attributes() {
        let input = r#"
        process MyProcess @version "1.0" @author "Developer" {
            task MyTask (timeout=30s, assignee="user1")
            end
        }
    "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];
        let attributes = &process.attributes;

        assert!(attributes.contains_key("version"));
        assert!(attributes.contains_key("author"));

        if let Some(AttributeValue::String(version)) = attributes.get("version") {
            assert_eq!(version, "1.0");
        }

        // Проверяем атрибуты задачи
        if let ProcessElement::Task {
            attributes: task_attrs,
            ..
        } = &process.elements[0]
        {
            assert!(task_attrs.contains_key("timeout"));
            assert!(task_attrs.contains_key("assignee"));

            if let Some(AttributeValue::Duration(timeout)) = task_attrs.get("timeout") {
                assert_eq!(timeout, "30s");
            }

            if let Some(AttributeValue::String(assignee)) = task_attrs.get("assignee") {
                assert_eq!(assignee, "user1");
            }
        }
    }

    #[test]
    fn test_different_task_types() {
        let input = r"
            process TaskTypes {
                task GenericTask
                user UserTask
                service ServiceTask
                script ScriptTask
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];
        assert_eq!(process.elements.len(), 5);

        let expected_types = [
            TaskType::Generic,
            TaskType::User,
            TaskType::Service,
            TaskType::Script,
        ];

        for (i, expected_type) in expected_types.iter().enumerate() {
            if let ProcessElement::Task { task_type, .. } = &process.elements[i] {
                assert_eq!(task_type, expected_type);
            } else {
                panic!("Expected Task at position {i}");
            }
        }
    }

    #[test]
    fn test_exclusive_gateway() {
        let input = r"
            process GatewayTest {
                start
                xor Decision? {
                    [condition1] -> Task1
                    [condition2] -> Task2
                    => DefaultTask
                }
                task Task1
                task Task2
                task DefaultTask
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        // Найдем gateway
        let gateway = process
            .elements
            .iter()
            .find(|e| matches!(e, ProcessElement::Gateway { .. }))
            .expect("Should have gateway");

        if let ProcessElement::Gateway {
            id,
            gateway_type,
            branches,
            ..
        } = gateway
        {
            assert_eq!(id.as_ref().unwrap(), "Decision");
            assert_eq!(*gateway_type, GatewayType::Exclusive);
            assert_eq!(branches.len(), 3);

            // Проверяем ветки
            assert_eq!(branches[0].condition.as_ref().unwrap(), "condition1");
            assert_eq!(branches[0].target, "Task1");
            assert!(!branches[0].is_default);

            assert_eq!(branches[1].condition.as_ref().unwrap(), "condition2");
            assert_eq!(branches[1].target, "Task2");
            assert!(!branches[1].is_default);

            assert!(branches[2].condition.is_none());
            assert_eq!(branches[2].target, "DefaultTask");
            assert!(branches[2].is_default);
        } else {
            panic!("Expected Gateway");
        }
    }

    #[test]
    fn test_parallel_gateway() {
        let input = r"
            process ParallelTest {
                start
                and Split {
                    branch1 -> Task1
                    branch2 -> Task2
                }
                task Task1
                task Task2
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        let gateway = process
            .elements
            .iter()
            .find(|e| matches!(e, ProcessElement::Gateway { .. }))
            .expect("Should have gateway");

        if let ProcessElement::Gateway {
            gateway_type,
            branches,
            ..
        } = gateway
        {
            assert_eq!(*gateway_type, GatewayType::Parallel);
            assert_eq!(branches.len(), 2);
        }
    }

    #[test]
    fn test_flows() {
        let input = r"
            process FlowTest {
                task Task1
                task Task2
                task Task3
                end
                
                Task1 -> Task2
                Task2 -> Task3
                Task3 -> end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];
        assert_eq!(process.flows.len(), 3);

        // Проверяем flows
        assert_eq!(process.flows[0].from, "Task1");
        assert_eq!(process.flows[0].to, "Task2");
        assert_eq!(process.flows[0].flow_type, FlowType::Sequence);

        assert_eq!(process.flows[1].from, "Task2");
        assert_eq!(process.flows[1].to, "Task3");

        assert_eq!(process.flows[2].from, "Task3");
        assert_eq!(process.flows[2].to, "end");
    }

    #[test]
    fn test_different_flow_types() {
        let input = r"
            process FlowTypes {
                task Task1
                task Task2
                task Task3
                task Task4
                end
                
                Task1 -> Task2
                Task2 --> Task3
                Task3 => Task4
                Task4 ..> end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];
        assert_eq!(process.flows.len(), 4);

        assert_eq!(process.flows[0].flow_type, FlowType::Sequence);
        assert_eq!(process.flows[1].flow_type, FlowType::Message);
        assert_eq!(process.flows[2].flow_type, FlowType::Default);
        assert_eq!(process.flows[3].flow_type, FlowType::Association);
    }

    #[test]
    fn test_conditional_flows() {
        let input = r#"
        process ConditionalTest {
            task Source
            task Target1
            task Target2
            end
            
            Source -> Target1 [amount > 1000]
            Source -> Target2 [status == "approved"]
            Target1 -> end
            Target2 -> end
        }
    "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];
        let conditional_flows: Vec<_> = process
            .flows
            .iter()
            .filter(|f| f.condition.is_some())
            .collect();

        assert_eq!(conditional_flows.len(), 2);

        // Проверяем первое условие (может содержать пробелы)
        let first_condition = conditional_flows[0].condition.as_ref().unwrap();
        assert!(first_condition.contains("amount") && first_condition.contains("1000"));

        // Проверяем второе условие
        let second_condition = conditional_flows[1].condition.as_ref().unwrap();
        assert!(second_condition.contains("status") && second_condition.contains("approved"));
    }

    #[test]
    fn test_call_activity() {
        let input = r"
            process CallTest {
                start
                call SubProcess
                call external::RemoteProcess
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        // Найдем call activities
        let calls: Vec<_> = process
            .elements
            .iter()
            .filter(|e| matches!(e, ProcessElement::CallActivity { .. }))
            .collect();

        assert_eq!(calls.len(), 2);

        if let ProcessElement::CallActivity {
            id, called_element, ..
        } = calls[0]
        {
            assert_eq!(id, "SubProcess");
            assert_eq!(called_element, "SubProcess");
        }

        if let ProcessElement::CallActivity {
            id, called_element, ..
        } = calls[1]
        {
            assert_eq!(id, "external");
            assert_eq!(called_element, "external::RemoteProcess");
        }
    }

    #[test]
    fn test_subprocess() {
        let input = r"
            process SubprocessTest {
                subprocess MySubprocess {
                    start
                    task InnerTask
                    end
                }
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        if let ProcessElement::Subprocess {
            id,
            elements,
            flows,
            ..
        } = &process.elements[0]
        {
            assert_eq!(id, "MySubprocess");
            assert_eq!(elements.len(), 3);
            assert_eq!(flows.len(), 0);
        } else {
            panic!("Expected Subprocess");
        }
    }

    #[test]
    fn test_pools_and_lanes() {
        let input = r"
            process PoolTest {
                pool CustomerPool {
                    lane FrontOffice {
                        task ReceiveOrder
                    }
                    lane BackOffice {
                        task ProcessOrder
                    }
                }
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        if let ProcessElement::Pool { name, lanes, .. } = &process.elements[0] {
            assert_eq!(name, "CustomerPool");
            assert_eq!(lanes.len(), 2);

            assert_eq!(lanes[0].name, "FrontOffice");
            assert_eq!(lanes[0].elements.len(), 1);

            assert_eq!(lanes[1].name, "BackOffice");
            assert_eq!(lanes[1].elements.len(), 1);
        } else {
            panic!("Expected Pool");
        }
    }

    #[test]
    fn test_events() {
        let input = r#"
            process EventTest {
                start @message "StartMessage"
                task IntermediateTask
                end @error "ErrorCode"
            }
        "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        // Проверяем start event
        if let ProcessElement::StartEvent { event_type, .. } = &process.elements[0] {
            if let Some(EventType::Message(msg)) = event_type {
                assert_eq!(msg, "StartMessage");
            } else {
                panic!("Expected Message event type, got: {event_type:?}");
            }
        } else {
            panic!("Expected StartEvent, got: {:?}", process.elements[0]);
        }

        // Проверяем end event
        if let ProcessElement::EndEvent { event_type, .. } = &process.elements[2] {
            if let Some(EventType::Error(code)) = event_type {
                assert_eq!(code, "ErrorCode");
            } else {
                panic!("Expected Error event type, got: {event_type:?}");
            }
        } else {
            panic!("Expected EndEvent, got: {:?}", process.elements[2]);
        }
    }

    #[test]
    fn test_imports() {
        let input = r#"
            import "external.bpmn" as external
            import ProcessPayment from "payment.bpmn"
            import ValidateData, SendEmail from "common.bpmn"
            
            process MainProcess {
                start
                call external::SomeProcess
                end
            }
        "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );
        assert_eq!(ast.imports.len(), 3);

        // Проверяем первый import
        let import1 = &ast.imports[0];
        assert_eq!(import1.path, "external.bpmn");
        assert_eq!(import1.alias.as_ref().unwrap(), "external");
        assert_eq!(import1.items.len(), 0);

        // Проверяем второй import
        let import2 = &ast.imports[1];
        assert_eq!(import2.path, "payment.bpmn");
        assert!(import2.alias.is_none());
        assert!(import2.items.contains(&"ProcessPayment".to_string()));

        // Проверяем третий import
        let import3 = &ast.imports[2];
        assert_eq!(import3.path, "common.bpmn");
        assert!(import3.alias.is_none());
        assert!(import3.items.contains(&"ValidateData".to_string()));
        assert!(import3.items.contains(&"SendEmail".to_string()));
    }

    #[test]
    fn test_groups_and_annotations() {
        let input = r#"
            process GroupTest {
                group "Data Processing" {
                    task LoadData
                    task TransformData
                }
                note "This is an important note"
                end
            }
        "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        // Проверяем group
        if let ProcessElement::Group {
            label, elements, ..
        } = &process.elements[0]
        {
            assert_eq!(label, "Data Processing");
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected Group");
        }

        // Проверяем annotation
        if let ProcessElement::Annotation { text, .. } = &process.elements[1] {
            assert_eq!(text, "This is an important note");
        } else {
            panic!("Expected Annotation");
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_attribute_values() {
        let input = r"
            process AttributeTest {
                task MyTask (
                    timeout=30s,
                    retries=3,
                    async=true,
                    priority=high,
                    amount=100.50
                )
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        if let ProcessElement::Task { attributes, .. } = &process.elements[0] {
            // Duration
            if let Some(AttributeValue::Duration(timeout)) = attributes.get("timeout") {
                assert_eq!(timeout, "30s");
            } else {
                panic!(
                    "timeout attribute not found or wrong type: {:?}",
                    attributes.get("timeout")
                );
            }

            // Number
            if let Some(AttributeValue::Number(retries)) = attributes.get("retries") {
                assert_eq!(*retries, 3.0);
            } else {
                panic!("retries attribute not found or wrong type");
            }

            // Boolean
            if let Some(AttributeValue::Boolean(async_val)) = attributes.get("async") {
                assert!(*async_val);
            } else {
                panic!("async attribute not found or wrong type");
            }

            // String identifier
            if let Some(AttributeValue::String(priority)) = attributes.get("priority") {
                assert_eq!(priority, "high");
            } else {
                panic!("priority attribute not found or wrong type");
            }

            // Number with decimal
            if let Some(AttributeValue::Number(amount)) = attributes.get("amount") {
                assert_eq!(*amount, 100.50);
            } else {
                panic!("amount attribute not found or wrong type");
            }
        }
    }

    #[test]
    fn test_complex_process() {
        let input = r#"
            process ComplexOrder @version "2.0" @author "Business Analyst" {
                start @message "OrderReceived"
                
                task ValidateOrder (timeout=5m assignee="validator")
                
                xor OrderValid? {
                    [validation_result == "valid"] -> ProcessOrder
                    [validation_result == "invalid"] -> RejectOrder
                    => ReviewManually
                }
                
                task ProcessOrder
                task RejectOrder
                task ReviewManually
                
                and PrepareShipment {
                    branch1 -> PackItems
                    branch2 -> GenerateInvoice
                }
                
                task PackItems (assignee="warehouse")
                service GenerateInvoice (endpoint="/api/invoice")
                
                end @message "OrderCompleted"
                
                // Flows
                ProcessOrder -> PrepareShipment
                PackItems -> end
                GenerateInvoice -> end
                RejectOrder -> end
                ReviewManually -> ProcessOrder
            }
        "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );
        assert_eq!(ast.processes.len(), 1);

        let process = &ast.processes[0];
        assert_eq!(process.name, "ComplexOrder");

        // Проверяем атрибуты процесса
        assert!(process.attributes.contains_key("version"));
        assert!(process.attributes.contains_key("author"));

        // Проверяем количество элементов
        let start_events = process
            .elements
            .iter()
            .filter(|e| matches!(e, ProcessElement::StartEvent { .. }))
            .count();
        let tasks = process
            .elements
            .iter()
            .filter(|e| matches!(e, ProcessElement::Task { .. }))
            .count();
        let gateways = process
            .elements
            .iter()
            .filter(|e| matches!(e, ProcessElement::Gateway { .. }))
            .count();
        let end_events = process
            .elements
            .iter()
            .filter(|e| matches!(e, ProcessElement::EndEvent { .. }))
            .count();

        assert_eq!(start_events, 1);
        assert!(tasks >= 4);
        assert_eq!(gateways, 2);
        assert_eq!(end_events, 1);

        // Проверяем flows
        assert!(process.flows.len() >= 5);
    }

    #[test]
    fn test_error_recovery() {
        let input = r"
            process ErrorTest {
                start
                invalid_token_here
                task ValidTask
                another_invalid & token
                end
            }
        ";

        let ast = parse_input(input);

        // Должны быть ошибки, но парсер должен восстановиться
        assert!(!ast.errors.is_empty());
        assert_eq!(ast.processes.len(), 1);

        let process = &ast.processes[0];
        assert_eq!(process.name, "ErrorTest");

        // Валидные элементы должны быть распознаны

        assert!(
            process
                .elements
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        ProcessElement::StartEvent { .. }
                            | ProcessElement::Task { .. }
                            | ProcessElement::EndEvent { .. }
                    )
                })
                .count()
                >= 2
        ); // start и end как минимум
    }

    #[test]
    fn test_missing_braces() {
        let input = r"
            process MissingBrace {
                start
                task MyTask
                // Отсутствует закрывающая скобка
        ";

        let ast = parse_input(input);

        // Должна быть ошибка о недостающей скобке
        assert!(!ast.errors.is_empty());

        // Но процесс должен быть частично распознан
        assert_eq!(ast.processes.len(), 1);
    }

    #[test]
    fn test_invalid_flow_syntax() {
        let input = r"
            process InvalidFlow {
                task Task1
                task Task2
                end
                
                Task1 invalid_arrow Task2
                Task2 -> end
            }
        ";

        let ast = parse_input(input);

        // Должны быть ошибки для невалидного flow
        assert!(!ast.errors.is_empty());

        let process = &ast.processes[0];

        // Валидный flow должен быть распознан

        assert_eq!(
            process
                .flows
                .iter()
                .filter(|f| f.from == "Task2" && f.to == "end")
                .count(),
            1
        );
    }

    #[test]
    fn test_multiple_processes() {
        let input = r"
            process FirstProcess {
                start
                task Task1
                end
            }
            
            process SecondProcess {
                start
                task Task2
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );
        assert_eq!(ast.processes.len(), 2);

        assert_eq!(ast.processes[0].name, "FirstProcess");
        assert_eq!(ast.processes[1].name, "SecondProcess");
    }

    #[test]
    fn test_comments_ignored() {
        let input = r"
            // This is a line comment
            process CommentTest {
                start
                /* This is a 
                   block comment */
                task MyTask // Another comment
                end
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );
        assert_eq!(ast.processes.len(), 1);

        let process = &ast.processes[0];
        assert_eq!(process.name, "CommentTest");
        assert_eq!(process.elements.len(), 3); // start, task, end
    }

    #[test]
    fn test_whitespace_handling() {
        let input = "process   SpacedProcess   {   start   task   MyTask   end   }";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );
        assert_eq!(ast.processes.len(), 1);

        let process = &ast.processes[0];
        assert_eq!(process.name, "SpacedProcess");
        assert_eq!(process.elements.len(), 3);
    }

    #[test]
    fn test_string_literal_escaping() {
        let input = r#"
            process StringTest {
                task MyTask @description "Task with \"quotes\" and \n newlines"
                end
            }
        "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        if let ProcessElement::Task { attributes, .. } = &process.elements[0] {
            if let AttributeValue::String(desc) = &attributes["description"] {
                assert!(desc.contains("\"quotes\""));
                assert!(desc.contains('\n'));
            } else {
                panic!("Expected String attribute");
            }
        }
    }

    #[test]
    fn test_nested_subprocess_flows() {
        let input = r"
            process NestedTest {
                subprocess OuterSub {
                    start
                    task OuterTask
                    
                    subprocess InnerSub {
                        start
                        task InnerTask
                        end
                        
                        InnerTask -> end
                    }
                    
                    OuterTask -> InnerSub
                    InnerSub -> end
                }
            }
        ";

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "Should have no errors: {:?}",
            ast.errors
        );

        let process = &ast.processes[0];

        if let ProcessElement::Subprocess {
            elements, flows, ..
        } = &process.elements[0]
        {
            assert!(elements.len() >= 3); // start, task, subprocess
            assert!(flows.len() >= 2); // flows внутри subprocess

            // Проверяем вложенный subprocess
            let inner_subprocess = elements
                .iter()
                .find(|e| matches!(e, ProcessElement::Subprocess { .. }))
                .expect("Should have inner subprocess");

            if let ProcessElement::Subprocess {
                elements: inner_elements,
                flows: inner_flows,
                ..
            } = inner_subprocess
            {
                assert_eq!(inner_elements.len(), 3); // start, task, end
                assert_eq!(inner_flows.len(), 1); // InnerTask -> end
            }
        } else {
            panic!("Expected Subprocess");
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use std::fs;

    use bpmncode::{
        lexer::Lexer,
        parser::{
            ast::{AstDocument, FlowType, ProcessElement},
            parse_tokens,
        },
    };

    fn parse_input(input: &str) -> AstDocument {
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();
        parse_tokens(tokens)
    }

    #[test]
    fn test_parse_example_files() {
        // Тест простого примера
        let simple_input =
            fs::read_to_string("examples/simple.bpmn").expect("Should read simple.bpmn");

        let ast = parse_input(&simple_input);
        assert_eq!(
            ast.errors.len(),
            0,
            "Simple example should parse without errors"
        );
        assert_eq!(ast.processes.len(), 1);
    }

    #[test]
    fn test_parse_with_recovery() {
        let input = r#"
            import "nonexistent.bpmn" as missing
            
            process RecoveryTest {
                start
                invalid_element_type SomeName
                task ValidTask
                another_invalid -> something
                end
                
                ValidTask -> end
            }
            
            process AnotherProcess {
                start
                task AnotherTask
                end
            }
        "#;

        let ast = parse_input(input);

        // Должны быть ошибки, но парсер должен восстановиться
        assert!(!ast.errors.is_empty());

        // Но валидные части должны быть распознаны
        assert_eq!(ast.imports.len(), 1);
        assert_eq!(ast.processes.len(), 2);

        // Второй процесс должен быть полностью валидным
        let second_process = &ast.processes[1];
        assert_eq!(second_process.name, "AnotherProcess");
        assert_eq!(second_process.elements.len(), 3);
    }

    #[test]
    fn test_deeply_nested_structures() {
        use std::fmt::Write;
        let mut input = String::from("process DeepNesting {\n");

        // Создаем глубоко вложенную структуру
        for i in 0..10 {
            writeln!(input, "    subprocess Level{i} {{").unwrap();
            input.push_str("        start\n");
            writeln!(input, "        task Task{i}").unwrap();
        }

        // Закрываем все subprocess'ы
        for _ in 0..10 {
            input.push_str("        end\n");
            input.push_str("    }\n");
        }

        input.push_str("}\n");

        let ast = parse_input(&input);

        // Проверяем что глубокая вложенность обрабатывается корректно
        assert_eq!(
            ast.errors.len(),
            0,
            "Deep nesting should parse without errors"
        );
        assert_eq!(ast.processes.len(), 1);
    }

    #[test]
    fn test_edge_cases() {
        // Тест различных граничных случаев
        let test_cases = vec![
            // Пустой процесс
            ("process Empty {}", 1, 0),
            // Процесс только с start
            ("process OnlyStart { start }", 1, 1),
            // Процесс только с end
            ("process OnlyEnd { end }", 1, 1),
            // Процесс с одним элементом
            ("process Single { task Lonely }", 1, 1),
        ];

        for (input, expected_processes, expected_elements) in test_cases {
            let ast = parse_input(input);

            assert_eq!(
                ast.processes.len(),
                expected_processes,
                "Failed for input: {input}"
            );

            if expected_processes > 0 {
                assert_eq!(
                    ast.processes[0].elements.len(),
                    expected_elements,
                    "Failed element count for input: {input}"
                );
            }
        }
    }

    #[test]
    fn test_malformed_syntax_recovery() {
        let input = r"
            process MalformedTest {
                start
                
                // Отсутствует имя задачи
                task
                
                // Неправильный синтаксис gateway
                xor {
                    condition1 -> Task1
                }
                
                // Правильная задача после ошибок
                task ValidTask
                
                // Неправильный flow
                ValidTask invalid_arrow
                
                // Правильный flow
                ValidTask -> end
                
                end
            }
        ";

        let ast = parse_input(input);

        // Должны быть ошибки
        assert!(!ast.errors.is_empty());

        // Но валидные элементы должны быть распознаны
        assert_eq!(ast.processes.len(), 1);

        let process = &ast.processes[0];

        // Должны быть распознаны start, ValidTask, end

        assert!(
            process
                .elements
                .iter()
                .filter(|e| match e {
                    ProcessElement::EndEvent { .. } | ProcessElement::StartEvent { .. } => true,
                    ProcessElement::Task { id, .. } => id == "ValidTask",
                    _ => false,
                })
                .count()
                >= 3
        );

        // Валидный flow должен быть распознан

        assert_eq!(
            process
                .flows
                .iter()
                .filter(|f| f.from == "ValidTask" && f.to == "end")
                .count(),
            1
        );
    }

    #[test]
    fn test_all_flow_combinations() {
        let input = r#"
            process FlowCombinations {
                task Source1
                task Source2
                task Target1
                task Target2
                task Target3
                end
                
                // Простые flows
                Source1 -> Target1
                Source1 --> Target2
                Source1 => Target3
                Source1 ..> end
                
                // Условные flows
                Source2 -> Target1 [condition1]
                Source2 --> Target2 [condition2 && condition3]
                Source2 => Target3 [status == "approved"]
                
                // Flows с комплексными условиями
                Target1 -> end [amount > 1000 && currency == "USD"]
                Target2 -> end [user.role == "admin" || priority == "high"]
                Target3 -> end
            }
        "#;

        let ast = parse_input(input);

        assert_eq!(
            ast.errors.len(),
            0,
            "All flow combinations should parse correctly"
        );

        let process = &ast.processes[0];
        assert_eq!(process.flows.len(), 10);

        // Проверяем типы flows
        let sequence_flows = process
            .flows
            .iter()
            .filter(|f| f.flow_type == FlowType::Sequence)
            .count();
        let message_flows = process
            .flows
            .iter()
            .filter(|f| f.flow_type == FlowType::Message)
            .count();
        let default_flows = process
            .flows
            .iter()
            .filter(|f| f.flow_type == FlowType::Default)
            .count();
        let association_flows = process
            .flows
            .iter()
            .filter(|f| f.flow_type == FlowType::Association)
            .count();

        assert!(sequence_flows >= 1);
        assert!(message_flows >= 1);
        assert!(default_flows >= 1);
        assert!(association_flows >= 1);

        // Проверяем условные flows
        let conditional_flows = process
            .flows
            .iter()
            .filter(|f| f.condition.is_some())
            .count();
        assert!(conditional_flows >= 5);
    }
}

#[cfg(test)]
mod benchmark_tests {
    use std::time::Instant;

    use bpmncode::{
        lexer::Lexer,
        parser::{ast::AstDocument, parse_tokens},
    };

    fn parse_input(input: &str) -> AstDocument {
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();
        parse_tokens(tokens)
    }

    #[test]
    fn benchmark_simple_process() {
        let input = r"
            process BenchmarkProcess {
                start
                task Task1
                task Task2
                task Task3
                end
                
                Task1 -> Task2
                Task2 -> Task3
                Task3 -> end
            }
        ";

        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let ast = parse_input(input);
            assert_eq!(ast.errors.len(), 0);
        }

        let duration = start.elapsed();
        let avg_duration = duration / iterations;

        println!("Average parsing time for simple process: {avg_duration:?}");

        // Парсинг простого процесса должен быть быстрым
        assert!(
            avg_duration.as_micros() < 1000,
            "Parsing is too slow: {avg_duration:?}"
        );
    }
}
