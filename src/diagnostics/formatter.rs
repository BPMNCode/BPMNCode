use super::{DiagnosticError, DiagnosticReport, Severity};
use colored::Colorize;
use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource};
use serde_json;

pub struct DiagnosticFormatter {
    use_colors: bool,
    show_source: bool,
}

impl DiagnosticFormatter {
    #[must_use]
    pub const fn new(use_colors: bool, show_source: bool) -> Self {
        Self {
            use_colors,
            show_source,
        }
    }

    #[allow(clippy::format_push_string)]
    #[must_use]
    pub fn format_cli(&self, report: &DiagnosticReport) -> String {
        if report.errors.is_empty() {
            return self.format_success_message(&report.file_path);
        }

        let mut output = String::new();

        if self.use_colors {
            output.push_str(&format!(
                "{} {}\n",
                "Checking:".blue().bold(),
                report.file_path.cyan()
            ));
        } else {
            output.push_str(&format!("Checking: {}\n", report.file_path));
        }

        for error in &report.errors {
            output.push_str(&self.format_error_cli(error, &report.source_code));
            output.push('\n');
        }

        let error_count = report.error_count();
        let warning_count = report.warning_count();

        if error_count > 0 {
            if self.use_colors {
                output.push_str(&format!(
                    "\n{} {} - {} found\n",
                    "✗".red().bold(),
                    report.file_path.cyan(),
                    self.format_count_text(error_count, warning_count).red()
                ));
            } else {
                output.push_str(&format!(
                    "\n✗ {} - {} found\n",
                    report.file_path,
                    self.format_count_text(error_count, warning_count)
                ));
            }
        }

        output
    }

    pub fn format_json(&self, report: &DiagnosticReport) -> Result<String, serde_json::Error> {
        #[derive(serde::Serialize)]
        struct JsonDiagnostic {
            file: String,
            errors: Vec<JsonError>,
            summary: JsonSummary,
        }

        #[derive(serde::Serialize)]
        struct JsonError {
            severity: String,
            message: String,
            line: usize,
            column: usize,
            start: usize,
            end: usize,
            suggestions: Vec<String>,
            code: Option<String>,
        }

        #[derive(serde::Serialize)]
        struct JsonSummary {
            error_count: usize,
            warning_count: usize,
            has_errors: bool,
        }

        let json_errors: Vec<JsonError> = report
            .errors
            .iter()
            .map(|error| {
                let span = error.span();
                JsonError {
                    severity: error.severity().to_string(),
                    message: error.to_string(),
                    line: span.line,
                    column: span.column,
                    start: span.start,
                    end: span.end,
                    suggestions: error.suggestions().to_vec(),
                    code: Some(self.extract_error_code(error)),
                }
            })
            .collect();

        let json_report = JsonDiagnostic {
            file: report.file_path.clone(),
            errors: json_errors,
            summary: JsonSummary {
                error_count: report.error_count(),
                warning_count: report.warning_count(),
                has_errors: report.has_errors(),
            },
        };

        serde_json::to_string_pretty(&json_report)
    }

    #[must_use]
    pub fn format_fancy(&self, report: &DiagnosticReport) -> String {
        if report.errors.is_empty() {
            return self.format_success_message(&report.file_path);
        }

        let mut output = String::new();
        let _source = NamedSource::new(&report.file_path, report.source_code.clone());

        let _handler = GraphicalReportHandler::new()
            .with_theme(if self.use_colors {
                GraphicalTheme::unicode()
            } else {
                GraphicalTheme::ascii()
            })
            .with_width(100);

        output.push_str(&self.format_cli(report));

        output
    }

    #[allow(clippy::format_push_string)]
    #[allow(clippy::uninlined_format_args)]
    fn format_error_cli(&self, error: &DiagnosticError, source: &str) -> String {
        let span = error.span();
        let severity_icon = match error.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };

        let location = format!("{}:{}:{}", span.file.display(), span.line, span.column);

        let mut output = if self.use_colors {
            format!(
                "  {}: {} {}",
                severity_icon.red().bold(),
                location.blue(),
                error
            )
        } else {
            format!("  {}: {} {}", severity_icon, location, error)
        };

        if self.show_source {
            if let Some(line) = self.get_source_line(source, span.line) {
                output.push('\n');
                output.push_str(&self.format_source_line(line, span.column, span.end - span.start));
            }
        }

        let suggestions = error.suggestions();
        if !suggestions.is_empty() {
            output.push('\n');
            if self.use_colors {
                output.push_str(&format!(
                    "    {}: {}",
                    "did you mean".cyan().bold(),
                    suggestions.join(", ").green()
                ));
            } else {
                output.push_str(&format!("    did you mean: {}", suggestions.join(", ")));
            }
        }

        output
    }

    #[allow(clippy::format_push_string)]
    fn format_source_line(&self, line: &str, column: usize, length: usize) -> String {
        let mut output = String::new();

        if self.use_colors {
            output.push_str(&format!("    {} | {}\n", "".blue(), line));
            output.push_str(&format!(
                "    {} | {}{}",
                "".blue(),
                " ".repeat(column.saturating_sub(1)),
                "^".repeat(length.max(1)).red().bold()
            ));
        } else {
            output.push_str(&format!("    | {line}\n"));
            output.push_str(&format!(
                "    | {}{}",
                " ".repeat(column.saturating_sub(1)),
                "^".repeat(length.max(1))
            ));
        }

        output
    }

    #[allow(clippy::unused_self)]
    fn get_source_line<'a>(&self, source: &'a str, line_number: usize) -> Option<&'a str> {
        source.lines().nth(line_number.saturating_sub(1))
    }

    #[allow(clippy::uninlined_format_args)]
    fn format_success_message(&self, file_path: &str) -> String {
        if self.use_colors {
            format!(
                "{} {} - no issues found\n",
                "✓".green().bold(),
                file_path.cyan()
            )
        } else {
            format!("✓ {} - no issues found\n", file_path)
        }
    }

    #[allow(clippy::unused_self)]
    fn format_count_text(&self, errors: usize, warnings: usize) -> String {
        match (errors, warnings) {
            (0, 0) => "no issues".to_string(),
            (0, w) => format!("{w} warnings"),
            (e, 0) => format!("{e} errors"),
            (e, w) => format!("{e} errors, {w} warnings"),
        }
    }

    #[allow(clippy::unused_self)]
    fn extract_error_code(&self, error: &DiagnosticError) -> String {
        match error {
            DiagnosticError::SyntaxError { .. } => "E001".to_string(),
            DiagnosticError::UnexpectedToken { .. } => "E002".to_string(),
            DiagnosticError::UndefinedReference { .. } => "E003".to_string(),
            DiagnosticError::DuplicateIdentifier { .. } => "E004".to_string(),
            DiagnosticError::InvalidAttribute { .. } => "E005".to_string(),
            DiagnosticError::MissingElement { .. } => "E006".to_string(),
            DiagnosticError::InvalidFlow { .. } => "E007".to_string(),
            DiagnosticError::ImportError { .. } => "E008".to_string(),
        }
    }
}

impl Default for DiagnosticFormatter {
    fn default() -> Self {
        Self::new(true, true)
    }
}
