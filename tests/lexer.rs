#[cfg(test)]
mod tests {
    use bpmncode::lexer::{Lexer, TokenKind};

    use std::path::Path;

    #[test]
    fn test_basic_keywords() {
        let input = "process start end task user service script call xor and event";
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::Process,
            TokenKind::Start,
            TokenKind::End,
            TokenKind::Task,
            TokenKind::User,
            TokenKind::Service,
            TokenKind::Script,
            TokenKind::Call,
            TokenKind::Xor,
            TokenKind::And,
            TokenKind::Event,
            TokenKind::Eof,
        ];

        let actual: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_flow_arrows() {
        let input = "-> --> => ..>";
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::SequenceFlow);
        assert_eq!(tokens[1].kind, TokenKind::MessageFlow);
        assert_eq!(tokens[2].kind, TokenKind::DefaultFlow);
        assert_eq!(tokens[3].kind, TokenKind::Association);
    }

    #[test]
    fn test_string_literals() {
        let input = r#""Simple string" "String with spaces" "String with \"quotes\"""#;
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].text, r#""Simple string""#);

        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[1].text, r#""String with spaces""#);

        assert_eq!(tokens[2].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[2].text, r#""String with \"quotes\"""#);
    }

    #[test]
    fn test_number_literals() {
        let input = "42 3.14 5m 10s 100ms";
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[0].text, "42");

        assert_eq!(tokens[1].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[1].text, "3.14");

        assert_eq!(tokens[2].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[2].text, "5m");

        assert_eq!(tokens[3].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[3].text, "10s");

        assert_eq!(tokens[4].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[4].text, "100ms");
    }

    #[test]
    fn test_identifiers() {
        let input = "ValidateOrder _private camelCase snake_case Order123";
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();

        let identifiers: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Identifier)
            .map(|t| &t.text)
            .collect();

        assert_eq!(
            identifiers,
            vec![
                "ValidateOrder",
                "_private",
                "camelCase",
                "snake_case",
                "Order123"
            ]
        );
    }

    #[test]
    fn test_comments() {
        let input = r"
            // Single line comment
            task ValidateOrder
            /* Multi-line
            comment */
            end
        ";
        let mut lexer = Lexer::new(input, "test.bpmn");
        let tokens = lexer.tokenize();

        let comment_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment))
            .collect();

        assert_eq!(comment_tokens.len(), 2);
        assert_eq!(comment_tokens[0].kind, TokenKind::LineComment);
        assert_eq!(comment_tokens[1].kind, TokenKind::BlockComment);
    }

    #[test]
    fn test_complete_process() {
        let input = r#"
            process OrderFlow @version "1.2" {
                start
                task ValidateOrder (async=true retries=3)
                xor InStock? {
                    yes -> ShipOrder
                    no -> NotifyCustomer
                }
                and SplitDelivery {
                    branch1 -> Pack
                    branch2 -> EmailInvoice
                }
                ShipOrder -> end
                NotifyCustomer -> end
            }
        "#;

        let mut lexer = Lexer::new(input, "order_flow.bpmn");
        let tokens = lexer.tokenize();

        // Проверяем основные элементы
        let keywords: Vec<_> = tokens
            .iter()
            .filter(|t| {
                matches!(
                    t.kind,
                    TokenKind::Process
                        | TokenKind::Start
                        | TokenKind::End
                        | TokenKind::Task
                        | TokenKind::Xor
                        | TokenKind::And
                )
            })
            .map(|t| &t.kind)
            .collect();

        assert!(keywords.contains(&&TokenKind::Process));
        assert!(keywords.contains(&&TokenKind::Start));
        assert!(keywords.contains(&&TokenKind::End));
        assert!(keywords.contains(&&TokenKind::Task));
        assert!(keywords.contains(&&TokenKind::Xor));
        assert!(keywords.contains(&&TokenKind::And));
    }

    #[test]
    fn test_imports_and_namespaces() {
        let input = r#"
            import "flows/payment.bpmn" as payment
            import subprocess PaymentFlow, DataValidation from "common.bpmn"
            call payment::ProcessPayment
        "#;

        let mut lexer = Lexer::new(input, "main.bpmn");
        let tokens = lexer.tokenize();

        // Проверяем импорты
        let import_related: Vec<_> = tokens
            .iter()
            .filter(|t| {
                matches!(
                    t.kind,
                    TokenKind::Import
                        | TokenKind::As
                        | TokenKind::From
                        | TokenKind::Subprocess
                        | TokenKind::Call
                        | TokenKind::Namespace
                )
            })
            .map(|t| &t.kind)
            .collect();

        assert!(import_related.contains(&&TokenKind::Import));
        assert!(import_related.contains(&&TokenKind::As));
        assert!(import_related.contains(&&TokenKind::From));
        assert!(import_related.contains(&&TokenKind::Subprocess));
        assert!(import_related.contains(&&TokenKind::Call));
        assert!(import_related.contains(&&TokenKind::Namespace));
    }

    #[test]
    fn test_pools_and_lanes() {
        let input = r"
            pool Sales {
                lane FrontOffice {
                    task ReceiveOrder
                }
                lane BackOffice {
                    task ProcessOrder
                }
            }
        ";

        let mut lexer = Lexer::new(input, "pools.bpmn");
        let tokens = lexer.tokenize();

        let pool_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Pool | TokenKind::Lane))
            .collect();

        assert_eq!(pool_tokens.len(), 3); // 1 pool + 2 lanes
    }

    #[test]
    fn test_attributes_and_annotations() {
        let input = r#"
            process Test @version "1.0" @author "Developer" {
                task Validate (async=true retries=3 timeout=30s)
                event timer 5m
            }
        "#;

        let mut lexer = Lexer::new(input, "attributes.bpmn");
        let tokens = lexer.tokenize();

        // Проверяем атрибуты
        let at_tokens: Vec<_> = tokens.iter().filter(|t| t.kind == TokenKind::At).collect();

        assert_eq!(at_tokens.len(), 2); // @version и @author

        // Проверяем скобки для параметров
        let paren_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::LeftParen | TokenKind::RightParen))
            .collect();

        assert_eq!(paren_tokens.len(), 2); // ( и )
    }

    #[test]
    fn test_position_tracking() {
        let input = "start\ntask ValidateOrder\nend";
        let mut lexer = Lexer::new(input, "position.bpmn");
        let tokens = lexer.tokenize();

        // Проверяем позиции
        let start_token = tokens.iter().find(|t| t.kind == TokenKind::Start).unwrap();
        assert_eq!(start_token.span.line, 1);
        assert_eq!(start_token.span.column, 1);

        let task_token = tokens.iter().find(|t| t.kind == TokenKind::Task).unwrap();
        assert_eq!(task_token.span.line, 2);
        assert_eq!(task_token.span.column, 1);

        let end_token = tokens.iter().find(|t| t.kind == TokenKind::End).unwrap();
        assert_eq!(end_token.span.line, 3);
        assert_eq!(end_token.span.column, 1);
    }

    #[test]
    fn test_file_tracking() {
        let input = "process Test { start -> end }";
        let mut lexer = Lexer::new(input, "test/order.bpmn");
        let tokens = lexer.tokenize();

        // Проверяем что все токены содержат правильный путь к файлу
        for token in &tokens {
            assert_eq!(token.span.file, Path::new("test/order.bpmn"));
        }
    }

    #[test]
    fn test_error_recovery() {
        let input = "task ValidOrder & invalid @ symbols -> end";
        let mut lexer = Lexer::new(input, "error.bpmn");
        let tokens = lexer.tokenize();

        // Лексер должен продолжить работу даже с неизвестными символами
        let unknown_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Unknown)
            .collect();

        assert!(!unknown_tokens.is_empty());

        // Но валидные токены должны быть распознаны
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Task));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::SequenceFlow));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::End));
    }

    #[test]
    fn test_whitespace_handling() {
        let input = "  start  \t\n  task   ValidateOrder  \n  end  ";
        let mut lexer = Lexer::new(input, "whitespace.bpmn");
        let tokens = lexer.tokenize();

        // Пробелы должны быть пропущены, но newlines сохранены
        let non_whitespace: Vec<_> = tokens
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Newline | TokenKind::CarriageReturnNewline
                )
            })
            .map(|t| &t.kind)
            .collect();

        assert_eq!(
            non_whitespace,
            vec![
                &TokenKind::Start,
                &TokenKind::Task,
                &TokenKind::Identifier,
                &TokenKind::End,
                &TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_complex_expressions() {
        let input = r#"
xor PaymentValid? {
    [amount > 0 && currency == "USD"] -> ProcessPayment
    [amount <= 0] -> RejectPayment
}
"#;

        let mut lexer = Lexer::new(input, "complex.bpmn");
        let tokens = lexer.tokenize();

        // Проверяем что сложные выражения токенизируются
        let brackets: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::LeftBracket | TokenKind::RightBracket))
            .collect();

        assert_eq!(brackets.len(), 4); // 2 условия в квадратных скобках
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let mut lexer = Lexer::new(input, "empty.bpmn");
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_only_comments() {
        let input = r"
            // This is a comment
            /* This is another comment */
        ";

        let mut lexer = Lexer::new(input, "comments.bpmn");
        let tokens = lexer.tokenize();

        let non_whitespace: Vec<_> = tokens
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Newline | TokenKind::CarriageReturnNewline
                )
            })
            .collect();

        assert_eq!(non_whitespace.len(), 3); // 2 comments + EOF
    }
}

#[cfg(test)]
mod multi_file_tests {
    use std::fs;

    use bpmncode::lexer::{TokenKind, multi_file::MultiFileLexer};
    use tempfile::TempDir;

    #[test]
    fn test_multi_file_lexing() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Создаем основной файл
        let main_content = r#"
            import "common.bpmn" as common
            process MainFlow {
                start
                call common::Validate
                end
            }
        "#;
        fs::write(temp_path.join("main.bpmn"), main_content).unwrap();

        // Создаем импортируемый файл
        let common_content = r"
            subprocess Validate {
                task CheckData
                xor Valid? {
                    yes -> end
                    no -> error
                }
            }
        ";
        fs::write(temp_path.join("common.bpmn"), common_content).unwrap();

        let mut lexer = MultiFileLexer::new(temp_path);
        let tokens = lexer
            .tokenize_file(temp_path.join("main.bpmn").as_path())
            .unwrap();

        // Проверяем что токены из основного файла присутствуют
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Import));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Process));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Call));

        // Проверяем что файлы правильно отслеживаются
        let main_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.span.file.file_name().unwrap() == "main.bpmn")
            .collect();
        assert!(!main_tokens.is_empty());
    }

    #[test]
    fn test_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let mut lexer = MultiFileLexer::new(temp_dir.path());

        let result = lexer.tokenize_file(temp_dir.path().join("nonexistent.bpmn").as_path());
        assert!(result.is_err());
    }
}
