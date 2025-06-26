use bpmncode::lexer::TokenKind;
use bpmncode::lexer::multi_file::MultiFileLexer;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "BPMNCode")]
#[command(about = "A textual DSL for BPMN 2.0 processes")]
#[command(version = "0.1.0")]
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

        /// Check all imported files recursively
        #[arg(short, long)]
        recursive: bool,

        /// Show detailed error information
        #[arg(short, long)]
        verbose: bool,

        /// Output format for diagnostics
        #[arg(long, default_value = "human")]
        format: DiagnosticFormat,
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
    /// Human-readable format
    Human,
    /// Short format
    Short,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Check {
            input,
            recursive: _,
            verbose,
            format,
        } => check_command(input, verbose, &format),
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut total_errors = 0;

    for input in inputs {
        if verbose {
            println!("{} {}", "Checking:".blue().bold(), input.display());
        }

        let base_dir = std::env::current_dir()?;
        let mut lexer = MultiFileLexer::new(base_dir);
        let tokens = lexer.tokenize_file(&input)?;

        for token in tokens.clone() {
            println!("{token:?}");
        }

        let errors: Vec<_> = tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Unknown))
            .collect();

        total_errors += errors.len();

        match format {
            DiagnosticFormat::Human => {
                if errors.is_empty() {
                    println!("{} {} - no issues found", "✓".green(), input.display());
                } else {
                    println!(
                        "{} {} - {} errors found",
                        "✗".red(),
                        input.display(),
                        errors.len()
                    );

                    for error in errors {
                        println!(
                            "  {} {}:{}:{} Unknown token '{}'",
                            "error:".red().bold(),
                            error.span.file.display(),
                            error.span.line,
                            error.span.column,
                            error.text
                        );
                    }
                }
            }
            DiagnosticFormat::Short => {
                if !errors.is_empty() {
                    for error in errors {
                        println!(
                            "{}:{}:{}: error: Unknown token '{}'",
                            error.span.file.display(),
                            error.span.line,
                            error.span.column,
                            error.text
                        );
                    }
                }
            }
        }
    }

    if total_errors == 0 {
        println!("\n{} All checks passed", "✓".green().bold());
    } else {
        println!("\n{} Found {} errors", "Summary:".bold(), total_errors,);
        return Err("Check failed".into());
    }

    Ok(())
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
