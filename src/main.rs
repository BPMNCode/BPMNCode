use bpmncode::diagnostics::context_validator::ContextValidator;
use bpmncode::diagnostics::formatter::DiagnosticFormatter;
use bpmncode::diagnostics::suggestions::{suggest_identifiers, suggest_keywords};
use bpmncode::diagnostics::{DiagnosticError, DiagnosticReport, Severity};
use bpmncode::lexer::multi_file::MultiFileLexer;
use bpmncode::parser::ast::ProcessElement;
use bpmncode::parser::parse_tokens_with_validation;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "BPMNCode")]
#[command(about = "A textual DSL for BPMN 2.0 processes")]
#[command(version = "0.1.2")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check BPMN source files for errors
    Check {
        /// Input BPMN source file(s)
        #[arg(value_name = "INPUT")]
        input: Vec<PathBuf>,

        /// Show detailed error information
        #[arg(short, long)]
        verbose: bool,

        /// Output format for diagnostics
        #[arg(long, default_value = "human")]
        format: DiagnosticFormat,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Hide source code context
        #[arg(long)]
        no_source: bool,
    },
    /// Show information about `BPMNCode`
    Info {
        /// Show version information
        #[arg(long)]
        version: bool,

        /// Show supported syntax
        #[arg(long)]
        syntax: bool,

        /// Show examples
        #[arg(long)]
        examples: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum DiagnosticFormat {
    /// Human-readable format with colors and source highlighting
    Human,
    /// Short format for quick scanning
    Short,
    /// JSON format for IDE/plugin consumption
    Json,
    /// Fancy format using miette
    Fancy,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Check {
            input,
            verbose,
            format,
            no_color,
            no_source,
        } => check_command(input, verbose, &format, no_color, no_source),
        Commands::Info {
            version,
            syntax,
            examples,
        } => {
            info_command(version, syntax, examples);
            return;
        }
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        process::exit(1);
    }
}

fn check_command(
    inputs: Vec<PathBuf>,
    verbose: bool,
    format: &DiagnosticFormat,
    no_color: bool,
    no_source: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let use_colors = !no_color && atty::is(atty::Stream::Stdout);
    let show_source = !no_source;
    let formatter = DiagnosticFormatter::new(use_colors, show_source);

    let mut total_errors = 0;
    let mut total_warnings = 0;

    for input in inputs {
        let source_code = fs::read_to_string(&input)?;
        let mut report = DiagnosticReport::new(input.display().to_string(), source_code.clone());

        let base_dir = std::env::current_dir()?;
        let mut lexer = MultiFileLexer::new(base_dir);
        let tokens = lexer.tokenize_file(&input)?;
        
        // Context validation on tokens (catch typos and syntax errors)
        let mut context_validator = ContextValidator::new(source_code.clone());
        let context_errors = context_validator.validate_tokens(&tokens);
        for error in context_errors {
            report.add_error(error);
        }

        let ast = parse_tokens_with_validation(tokens);

        for error in &ast.errors {
            let diagnostic_error = convert_parser_error_to_diagnostic(error, &ast);
            report.add_error(diagnostic_error);
        }

        total_errors += report.error_count();
        total_warnings += report.warning_count();

        match format {
            DiagnosticFormat::Human => {
                print!("{}", formatter.format_cli(&report));

                if verbose && report.errors.is_empty() {
                    print_verbose_success_info(&ast, use_colors);
                }
            }
            DiagnosticFormat::Short => {
                print_short_format(&report);
            }
            DiagnosticFormat::Json => {
                println!("{}", formatter.format_json(&report)?);
            }
            DiagnosticFormat::Fancy => {
                print!("{}", formatter.format_fancy(&report));
            }
        }

        if verbose && !matches!(format, DiagnosticFormat::Json) {
            print_ast_debug_info(&ast, use_colors);
        }
    }

    if !matches!(format, DiagnosticFormat::Json) {
        print_summary(total_errors, total_warnings, use_colors)?;
    }

    if total_errors > 0 {
        Err("Check failed".into())
    } else {
        Ok(())
    }
}

fn convert_parser_error_to_diagnostic(
    error: &bpmncode::parser::ast::ParseError,
    ast: &bpmncode::parser::ast::AstDocument,
) -> DiagnosticError {
    let suggestions = if error.message.contains("Unexpected token") {
        error
            .message
            .find('\'')
            .map_or_else(Vec::new, |token_start| {
                error.message[token_start + 1..]
                    .find('\'')
                    .map_or_else(Vec::new, |token_end| {
                        let found_token =
                            &error.message[token_start + 1..token_start + 1 + token_end];
                        suggest_keywords(found_token)
                    })
            })
    } else if error.message.contains("Unknown") {
        let identifiers: Vec<String> =
            ast.processes
                .iter()
                .flat_map(|p| {
                    p.elements.iter().filter_map(|e| match e {
                        ProcessElement::CallActivity { id, .. }
                        | ProcessElement::Task { id, .. } => Some(id.clone()),
                        ProcessElement::Gateway { id, .. } => id.clone(),
                        _ => None,
                    })
                })
                .collect();

        error
            .message
            .find('\'')
            .map_or_else(Vec::new, |name_start| {
                error.message[name_start + 1..]
                    .find('\'')
                    .map_or_else(Vec::new, |name_end| {
                        let unknown_name =
                            &error.message[name_start + 1..name_start + 1 + name_end];
                        suggest_identifiers(unknown_name, &identifiers)
                    })
            })
    } else {
        Vec::new()
    };

    DiagnosticError::SyntaxError {
        message: error.message.clone(),
        span: error.span.clone(),
        severity: match error.severity {
            bpmncode::parser::ast::ErrorSeverity::Error => Severity::Error,
            bpmncode::parser::ast::ErrorSeverity::Warning => Severity::Warning,
        },
        suggestions,
    }
}

fn print_verbose_success_info(ast: &bpmncode::parser::ast::AstDocument, use_colors: bool) {
    if use_colors {
        println!("  {} processes: {}", "📊".blue(), ast.processes.len());
        println!("  {} imports: {}", "📦".blue(), ast.imports.len());

        for process in &ast.processes {
            println!(
                "  {} '{}' has {} elements",
                "🔄".blue(),
                process.name,
                process.elements.len()
            );
        }
    } else {
        println!("  📊 processes: {}", ast.processes.len());
        println!("  📦 imports: {}", ast.imports.len());

        for process in &ast.processes {
            println!(
                "  🔄 '{}' has {} elements",
                process.name,
                process.elements.len()
            );
        }
    }
}

fn print_short_format(report: &DiagnosticReport) {
    for error in &report.errors {
        let span = error.span();
        println!(
            "{}:{}:{}: {}: {}",
            span.file.display(),
            span.line,
            span.column,
            error.severity(),
            error
        );
    }
}

fn print_ast_debug_info(ast: &bpmncode::parser::ast::AstDocument, use_colors: bool) {
    if use_colors {
        println!("{} AST structure:", "Debug:".yellow().bold());
    } else {
        println!("Debug: AST structure:");
    }
    print_ast_summary(ast, use_colors);
}

#[allow(clippy::unnecessary_wraps)]
fn print_summary(
    total_errors: usize,
    total_warnings: usize,
    use_colors: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if total_errors == 0 && total_warnings == 0 {
        if use_colors {
            println!("\n{} All checks passed", "✓".green().bold());
        } else {
            println!("\n✓ All checks passed");
        }
    } else {
        let summary = match (total_errors, total_warnings) {
            (0, w) => format!("{w} warnings"),
            (e, 0) => format!("{e} errors"),
            (e, w) => format!("{e} errors, {w} warnings"),
        };

        if use_colors {
            println!("\n{} Found {}", "Summary:".bold(), summary);
        } else {
            println!("\nSummary: Found {summary}");
        }
    }
    Ok(())
}

fn print_ast_summary(ast: &bpmncode::parser::ast::AstDocument, use_colors: bool) {
    println!("  📄 Imports: {}", ast.imports.len());
    for import in &ast.imports {
        if let Some(alias) = &import.alias {
            println!("    - {} as {}", import.path, alias);
        } else {
            println!("    - {} (items: {})", import.path, import.items.join(", "));
        }
    }

    println!("  🔄 Processes: {}", ast.processes.len());
    for process in &ast.processes {
        println!(
            "    - {} ({} elements, {} flows)",
            process.name,
            process.elements.len(),
            process.flows.len()
        );

        let mut element_counts = std::collections::HashMap::new();
        for element in &process.elements {
            let element_type = match element {
                ProcessElement::StartEvent { .. } => "start",
                ProcessElement::EndEvent { .. } => "end",
                ProcessElement::Task { task_type, .. } => match task_type {
                    bpmncode::parser::ast::TaskType::Generic => "task",
                    bpmncode::parser::ast::TaskType::User => "user",
                    bpmncode::parser::ast::TaskType::Service => "service",
                    bpmncode::parser::ast::TaskType::Script => "script",
                },
                ProcessElement::Gateway { gateway_type, .. } => match gateway_type {
                    bpmncode::parser::ast::GatewayType::Exclusive => "xor",
                    bpmncode::parser::ast::GatewayType::Parallel => "and",
                },
                ProcessElement::IntermediateEvent { .. } => "event",
                ProcessElement::Subprocess { .. } => "subprocess",
                ProcessElement::CallActivity { .. } => "call",
                ProcessElement::Pool { .. } => "pool",
                ProcessElement::Group { .. } => "group",
                ProcessElement::Annotation { .. } => "note",
            };
            *element_counts.entry(element_type).or_insert(0) += 1;
        }

        for (element_type, count) in element_counts {
            if use_colors {
                println!("      {} {}: {}", "•".blue(), element_type, count);
            } else {
                println!("      • {element_type}: {count}");
            }
        }
    }

    if !ast.errors.is_empty() {
        println!("  ❌ Errors: {}", ast.errors.len());
        for error in &ast.errors {
            println!("    - {}", error.message);
        }
    }
}

fn info_command(version: bool, syntax: bool, examples: bool) {
    if version {
        show_version();
        return;
    }

    if syntax {
        show_syntax();
        return;
    }

    if examples {
        show_examples();
        return;
    }

    show_general_info();
}

fn show_version() {
    println!("{} {}", "BPMNCode".blue().bold(), env!("CARGO_PKG_VERSION"));
    println!(
        "Build: {} ({})",
        option_env!("BUILD_HASH").unwrap_or("dev"),
        option_env!("BUILD_DATE").unwrap_or("unknown")
    );
}

fn show_syntax() {
    println!("{}", "BPMNCode Syntax Reference".blue().bold());
    println!();

    println!("{}", "Process Definition:".green().bold());
    println!("  process ProcessName @version \"1.0\" {{ ... }}");
    println!();

    println!("{}", "Elements:".green().bold());

    println!("  start                    - Start event");
    println!("  end                      - End event");
    println!("  task TaskName            - Generic task");
    println!("  user TaskName            - User task");
    println!("  service TaskName         - Service task");
    println!("  script TaskName          - Script task");
    println!();

    println!("{}", "Gateways:".green().bold());
    println!("  xor GatewayName? {{ ... }}  - Exclusive gateway");
    println!("  and GatewayName {{ ... }}   - Parallel gateway");
    println!();

    println!("{}", "Flows:".green().bold());
    println!("  ->                       - Sequence flow");
    println!("  -->                      - Message flow");
    println!("  =>                       - Default flow");
    println!("  ..>                      - Association");
    println!();

    println!("{}", "Containers:".green().bold());
    println!("  pool PoolName {{ ... }}    - Pool");
    println!("  lane LaneName {{ ... }}    - Lane");
    println!("  subprocess Name {{ ... }}  - Subprocess");
    println!();

    println!("{}", "Imports:".green().bold());
    println!("  import \"file.bpmn\" as alias");
    println!("  import element from \"file.bpmn\"");
    println!();

    println!("{}", "Attributes:".green().bold());
    println!("  task Name (async=true retries=3)");
    println!("  @version \"1.0\" @author \"Developer\"");
}

fn show_examples() {
    println!("{}", "BPMNCode Examples".blue().bold());
    println!();

    println!("{}", "Simple Process:".green().bold());
    println!("  process OrderFlow {{");
    println!("      start");
    println!("      task ValidateOrder");
    println!("      task ProcessPayment");
    println!("      end");
    println!("  }}");
    println!();

    println!("{}", "Process with Gateway:".green().bold());
    println!("  process OrderFlow {{");
    println!("      start");
    println!("      task ValidateOrder");
    println!("      xor OrderValid? {{");
    println!("          yes -> ProcessOrder");
    println!("          no -> RejectOrder");
    println!("      }}");
    println!("      ProcessOrder -> end");
    println!("      RejectOrder -> end");
    println!("  }}");
    println!();

    println!("{}", "Multi-file Process:".green().bold());
    println!("  import \"validation.bpmn\" as validation");
    println!("  process MainFlow {{");
    println!("      start");
    println!("      call validation::ValidateOrder");
    println!("      end");
    println!("  }}");
}

fn show_general_info() {
    println!("{}", "BPMNCode - Textual DSL for BPMN 2.0".blue().bold());
    println!();
    println!("Write business processes as code and generate BPMN diagrams.");
    println!();

    println!("{}", "Available Commands:".green().bold());
    println!("  {}    Check source files for errors", "check".cyan());
    println!("  {}      Show information and help", "info".cyan());
    println!();

    println!(
        "Use {} for detailed help on any command.",
        "bpmncode <command> --help".cyan()
    );
    println!(
        "Use {} for syntax reference.",
        "bpmncode info --syntax".cyan()
    );
    println!("Use {} for examples.", "bpmncode info --examples".cyan());
}
