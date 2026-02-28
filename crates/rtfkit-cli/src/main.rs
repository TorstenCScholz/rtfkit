use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::DateTime;
use clap::{Parser, Subcommand, ValueEnum};
use rtfkit_core::{Document, Report, Warning, parse};
use rtfkit_docx::write_docx;
use rtfkit_html::{CssMode, HtmlWriterOptions, document_to_html_with_warnings};
use rtfkit_render_typst::{
    DeterminismOptions, Margins, PageSize, RenderOptions, document_to_pdf_with_warnings,
};
use rtfkit_style_tokens::StyleProfileName;
use tracing::debug;

/// RTF conversion toolkit - convert RTF files to various formats.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Enable verbose (debug) logging
    #[arg(long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

/// CSS mode argument for CLI parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CssModeArg {
    /// Embed the built-in polished stylesheet (default).
    Default,
    /// Omit built-in CSS, emit semantic HTML only.
    None,
}

impl From<CssModeArg> for CssMode {
    fn from(arg: CssModeArg) -> Self {
        match arg {
            CssModeArg::Default => CssMode::Default,
            CssModeArg::None => CssMode::None,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Docx,
    Html,
    Pdf,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum ReportFormat {
    Json,
    Text,
}

/// Infer target format from output file extension.
/// Returns None if the extension doesn't match a known format.
fn infer_target_from_extension(path: &Path) -> Option<Target> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    match extension.as_str() {
        "docx" => Some(Target::Docx),
        "html" | "htm" => Some(Target::Html),
        "pdf" => Some(Target::Pdf),
        _ => None,
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert an RTF file to another format
    Convert {
        /// Input RTF file to convert
        #[arg(required = true)]
        input: PathBuf,

        /// Output file path (e.g., output.docx, output.html, output.pdf)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output target format (docx, html, or pdf)
        #[arg(long, value_enum)]
        to: Option<Target>,

        /// Output format for conversion report
        #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
        format: ReportFormat,

        /// Serialize IR to JSON file for analysis/debugging
        #[arg(long, value_name = "FILE")]
        emit_ir: Option<PathBuf>,

        /// Fail on unsupported features (DroppedContent warnings)
        #[arg(long)]
        strict: bool,

        /// Overwrite existing output file without prompting
        #[arg(long)]
        force: bool,

        /// CSS output mode for HTML output
        #[arg(long, value_name = "MODE", value_enum)]
        html_css: Option<CssModeArg>,

        /// Path to custom CSS file to append after built-in CSS
        #[arg(long, value_name = "FILE")]
        html_css_file: Option<PathBuf>,

        /// PDF page size (default: a4)
        #[arg(long, value_name = "a4|letter")]
        pdf_page_size: Option<String>,

        /// Fixed RFC3339 timestamp for deterministic PDF metadata
        #[arg(long, value_name = "RFC3339")]
        fixed_timestamp: Option<String>,

        /// Style profile for HTML and PDF output (default: report)
        #[arg(long, value_name = "PROFILE")]
        style_profile: Option<String>,
    },
}

/// Exit codes as defined in PHASE1.md
const EXIT_SUCCESS: u8 = 0;
const EXIT_PARSE_ERROR: u8 = 2;
const EXIT_CONVERSION_ERROR: u8 = 3;
const EXIT_STRICT_MODE: u8 = 4;
const MAX_CUSTOM_CSS_BYTES: u64 = 1024 * 1024; // 1 MiB

/// Parse and validate a style profile name.
/// Returns the resolved StyleProfileName or an error message.
fn parse_style_profile(input: &Option<String>) -> Result<StyleProfileName, String> {
    match input {
        None => Ok(StyleProfileName::default()), // Report
        Some(s) => match s.to_lowercase().as_str() {
            "classic" => Ok(StyleProfileName::Classic),
            "report" => Ok(StyleProfileName::Report),
            "compact" => Ok(StyleProfileName::Compact),
            _ => Err(format!(
                "Unknown style profile: '{}'. Valid options: classic, report, compact",
                s
            )),
        },
    }
}

struct ConvertRequest {
    input: PathBuf,
    output: Option<PathBuf>,
    to: Option<Target>,
    format: ReportFormat,
    emit_ir: Option<PathBuf>,
    strict: bool,
    force: bool,
    verbose: bool,
    html_css: Option<CssModeArg>,
    html_css_file: Option<PathBuf>,
    pdf_page_size: Option<String>,
    fixed_timestamp: Option<String>,
    style_profile: Option<String>,
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    debug!("Parsed CLI args: {:?}", cli);

    match cli.command {
        Commands::Convert {
            input,
            output,
            to,
            format,
            emit_ir,
            strict,
            force,
            html_css,
            html_css_file,
            pdf_page_size,
            fixed_timestamp,
            style_profile,
        } => handle_convert(ConvertRequest {
            input,
            output,
            to,
            format,
            emit_ir,
            strict,
            force,
            verbose: cli.verbose,
            html_css,
            html_css_file,
            pdf_page_size,
            fixed_timestamp,
            style_profile,
        }),
    }
}

fn handle_convert(request: ConvertRequest) -> Result<ExitCode> {
    let ConvertRequest {
        input,
        output,
        to,
        format,
        emit_ir,
        strict,
        force,
        verbose,
        html_css,
        html_css_file,
        pdf_page_size,
        fixed_timestamp,
        style_profile,
    } = request;

    // Parse and validate style profile if provided
    let resolved_style_profile = match parse_style_profile(&style_profile) {
        Ok(profile) => profile,
        Err(error_msg) => {
            eprintln!("Error: {}", error_msg);
            return Ok(ExitCode::from(EXIT_PARSE_ERROR));
        }
    };

    // Resolve target format:
    // - Explicit --to takes precedence.
    // - When --to is omitted, infer from output extension if known.
    // - Fall back to docx.
    let extension_target = output
        .as_ref()
        .and_then(|output_path| infer_target_from_extension(output_path));
    let resolved_to = match (to, extension_target) {
        (Some(explicit), Some(inferred)) if explicit != inferred => {
            eprintln!(
                "Error: target format mismatch: --to {} conflicts with output extension '{}'",
                match explicit {
                    Target::Docx => "docx",
                    Target::Html => "html",
                    Target::Pdf => "pdf",
                },
                match inferred {
                    Target::Docx => "docx",
                    Target::Html => "html",
                    Target::Pdf => "pdf",
                }
            );
            eprintln!("       Use matching values or omit --to to infer from extension.");
            return Ok(ExitCode::from(EXIT_PARSE_ERROR));
        }
        (Some(explicit), _) => explicit,
        (None, Some(inferred)) => inferred,
        (None, None) => Target::Docx,
    };

    debug!(
        "Target format resolved: {:?} (requested: {:?}, extension: {:?})",
        resolved_to, to, extension_target
    );

    // HTML-specific flags are only valid with --to html.
    if resolved_to != Target::Html && (html_css.is_some() || html_css_file.is_some()) {
        eprintln!("Error: --html-css and --html-css-file are only valid with --to html");
        return Ok(ExitCode::from(EXIT_PARSE_ERROR));
    }

    // PDF-specific flags are only valid with --to pdf.
    if resolved_to != Target::Pdf && (pdf_page_size.is_some() || fixed_timestamp.is_some()) {
        eprintln!("Error: --pdf-page-size and --fixed-timestamp are only valid with --to pdf");
        return Ok(ExitCode::from(EXIT_PARSE_ERROR));
    }

    // Style profiles are not supported for DOCX output (MVP).
    if resolved_to == Target::Docx && style_profile.is_some() {
        eprintln!("Error: Style profiles are not supported for DOCX output");
        return Ok(ExitCode::from(EXIT_PARSE_ERROR));
    }

    // Validate pdf_page_size value if provided
    if let Some(ref page_size) = pdf_page_size {
        let page_size_lower = page_size.to_lowercase();
        if page_size_lower != "a4" && page_size_lower != "letter" {
            eprintln!(
                "Error: Invalid PDF page size '{}'. Valid values: a4, letter",
                page_size
            );
            return Ok(ExitCode::from(EXIT_PARSE_ERROR));
        }
    }

    // Validate fixed_timestamp value if provided
    if let Some(ref timestamp) = fixed_timestamp
        && DateTime::parse_from_rfc3339(timestamp).is_err()
    {
        eprintln!(
            "Error: Invalid timestamp '{}'. Use RFC3339 format, e.g. 2024-01-01T00:00:00Z",
            timestamp
        );
        return Ok(ExitCode::from(EXIT_PARSE_ERROR));
    }

    // Read input file
    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read file: {}", input.display()))?;

    // Parse RTF using the new API
    let (document, report) = match parse(&content) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Parse error: {e}");
            return Ok(ExitCode::from(EXIT_PARSE_ERROR));
        }
    };

    debug!("Parsed document with {} blocks", document.blocks.len());
    debug!("Report: {} warnings", report.warnings.len());

    // Emit IR if requested
    if let Some(ir_path) = emit_ir.as_ref() {
        emit_ir_to_file(&document, ir_path)?;
    }

    // Strict mode for parser/interpreter warnings.
    if strict {
        let dropped_reasons = dropped_content_reasons(&report.warnings);
        if !dropped_reasons.is_empty() {
            print_strict_mode_violation(&dropped_reasons);
            return Ok(ExitCode::from(EXIT_STRICT_MODE));
        }
    }

    // Handle output if --output is specified
    if let Some(output_path) = output.as_ref() {
        return match resolved_to {
            Target::Docx => handle_docx_output(&document, output_path, force, verbose),
            Target::Html => handle_html_output(
                &document,
                output_path,
                force,
                verbose,
                strict,
                html_css,
                html_css_file.as_ref(),
                resolved_style_profile,
            ),
            Target::Pdf => handle_pdf_output(
                &document,
                output_path,
                force,
                verbose,
                strict,
                pdf_page_size.as_deref(),
                fixed_timestamp.as_deref(),
                resolved_style_profile,
            ),
        };
    }

    // Output report to stdout (when no --output specified)
    match format {
        ReportFormat::Json => print_report_json(&report)?,
        ReportFormat::Text => print_report_text(&report),
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
}

fn validate_output_path(
    output_path: &Path,
    force: bool,
    verbose: bool,
    format_name: &str,
    valid_extensions: &[&str],
    suggested_extension: &str,
) -> Option<ExitCode> {
    let extension = output_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());

    match extension.as_deref() {
        Some(ext) if valid_extensions.iter().any(|candidate| candidate == &ext) => {}
        Some(other) => {
            if verbose {
                eprintln!(
                    "Warning: Output file has '.{other}' extension, but {format_name} format will be written."
                );
                eprintln!("         Consider using '{suggested_extension}' extension for clarity.");
            }
        }
        None => {
            if verbose {
                eprintln!(
                    "Warning: Output file has no extension. {format_name} format will be written."
                );
                eprintln!("         Consider using '{suggested_extension}' extension for clarity.");
            }
        }
    }

    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            eprintln!(
                "Error: Output directory does not exist: {}",
                parent.display()
            );
            return Some(ExitCode::from(EXIT_CONVERSION_ERROR));
        }

        // Check directory writability using a unique probe filename.
        if let Err(e) = check_directory_writable(parent) {
            eprintln!(
                "Error: Output directory is not writable: {}",
                parent.display()
            );
            eprintln!("  {e}");
            return Some(ExitCode::from(EXIT_CONVERSION_ERROR));
        }
    }

    if output_path.exists() && !force {
        eprintln!(
            "Error: Output file already exists: {}",
            output_path.display()
        );
        eprintln!("       Use --force to overwrite existing files.");
        return Some(ExitCode::from(EXIT_CONVERSION_ERROR));
    }

    None
}

/// Handle writing DOCX output with validation and error handling
fn handle_docx_output(
    document: &Document,
    output_path: &Path,
    force: bool,
    verbose: bool,
) -> Result<ExitCode> {
    if let Some(code) =
        validate_output_path(output_path, force, verbose, "DOCX", &["docx"], ".docx")
    {
        return Ok(code);
    }

    // Write DOCX file
    debug!("Writing DOCX to: {}", output_path.display());
    if let Err(e) = write_docx(document, output_path) {
        eprintln!("Error writing DOCX file: {}", output_path.display());
        eprintln!("  {e}");
        return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
    }

    eprintln!("DOCX written to: {}", output_path.display());
    Ok(ExitCode::from(EXIT_SUCCESS))
}

/// Handle writing HTML output with validation and error handling
#[allow(clippy::too_many_arguments)]
fn handle_html_output(
    document: &Document,
    output_path: &Path,
    force: bool,
    verbose: bool,
    strict: bool,
    html_css: Option<CssModeArg>,
    html_css_file: Option<&PathBuf>,
    style_profile: StyleProfileName,
) -> Result<ExitCode> {
    if let Some(code) = validate_output_path(
        output_path,
        force,
        verbose,
        "HTML",
        &["html", "htm"],
        ".html",
    ) {
        return Ok(code);
    }

    // Build HTML writer options
    let mut html_options = HtmlWriterOptions {
        css_mode: html_css.unwrap_or(CssModeArg::Default).into(),
        style_profile,
        ..Default::default()
    };

    // Load custom CSS file if provided
    if let Some(css_file) = html_css_file {
        let css_size = match fs::metadata(css_file) {
            Ok(metadata) => metadata.len(),
            Err(e) => {
                eprintln!("Error reading CSS file metadata: {}", css_file.display());
                eprintln!("  {e}");
                return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
            }
        };
        if css_size > MAX_CUSTOM_CSS_BYTES {
            eprintln!(
                "Error: CSS file too large: {} bytes exceeds limit of {} bytes",
                css_size, MAX_CUSTOM_CSS_BYTES
            );
            return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
        }

        match fs::read_to_string(css_file) {
            Ok(css) => html_options.custom_css = Some(css),
            Err(e) => {
                eprintln!("Error reading CSS file: {}", css_file.display());
                eprintln!("  {e}");
                return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
            }
        }
    }

    // Generate HTML
    debug!("Writing HTML to: {}", output_path.display());
    let output = match document_to_html_with_warnings(document, &html_options) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Error generating HTML: {}", output_path.display());
            eprintln!("  {e}");
            return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
        }
    };

    // Strict mode for HTML includes writer-level drops.
    // Parser/interpreter drops are checked in handle_convert.
    if strict {
        let dropped_reasons = output.dropped_content_reasons.clone();
        if !dropped_reasons.is_empty() {
            print_strict_mode_violation(&dropped_reasons);
            return Ok(ExitCode::from(EXIT_STRICT_MODE));
        }
    }

    // Write HTML file
    if let Err(e) = fs::write(output_path, output.html) {
        eprintln!("Error writing HTML file: {}", output_path.display());
        eprintln!("  {e}");
        return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
    }

    eprintln!("HTML written to: {}", output_path.display());
    Ok(ExitCode::from(EXIT_SUCCESS))
}

/// Handle writing PDF output with validation and error handling
#[allow(clippy::too_many_arguments)]
fn handle_pdf_output(
    document: &Document,
    output_path: &Path,
    force: bool,
    verbose: bool,
    strict: bool,
    pdf_page_size: Option<&str>,
    fixed_timestamp: Option<&str>,
    style_profile: StyleProfileName,
) -> Result<ExitCode> {
    if let Some(code) = validate_output_path(output_path, force, verbose, "PDF", &["pdf"], ".pdf") {
        return Ok(code);
    }

    // Build PDF render options
    let options = RenderOptions {
        page_size: match pdf_page_size.map(|s| s.to_lowercase()).as_deref() {
            Some("letter") => PageSize::Letter,
            _ => PageSize::A4,
        },
        margins: Margins::default(),
        determinism: DeterminismOptions {
            fixed_timestamp: fixed_timestamp.map(str::to_owned),
            normalize_metadata: fixed_timestamp.is_some(),
        },
        style_profile,
    };

    // Generate PDF using in-process Typst renderer
    debug!("Writing PDF to: {}", output_path.display());
    let output = match document_to_pdf_with_warnings(document, &options) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Error generating PDF: {}", output_path.display());
            eprintln!("  {e}");
            return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
        }
    };

    // Strict mode for PDF includes writer-level dropped semantic content.
    // Parser/interpreter drops are checked in handle_convert.
    if strict {
        let dropped_reasons: Vec<String> = output
            .warnings
            .iter()
            .filter_map(|w| {
                if matches!(w.kind, rtfkit_render_typst::WarningKind::DroppedContent) {
                    Some(w.message.clone())
                } else {
                    None
                }
            })
            .collect();
        if !dropped_reasons.is_empty() {
            print_strict_mode_violation(&dropped_reasons);
            return Ok(ExitCode::from(EXIT_STRICT_MODE));
        }
    }

    // Write PDF file
    if let Err(e) = fs::write(output_path, output.pdf_bytes) {
        eprintln!("Error writing PDF file: {}", output_path.display());
        eprintln!("  {e}");
        return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
    }

    eprintln!("PDF written to: {}", output_path.display());
    Ok(ExitCode::from(EXIT_SUCCESS))
}

fn dropped_content_reasons(warnings: &[Warning]) -> Vec<String> {
    warnings
        .iter()
        .filter_map(|warning| {
            if let Warning::DroppedContent { reason, .. } = warning {
                Some(reason.clone())
            } else {
                None
            }
        })
        .collect()
}

fn print_strict_mode_violation(dropped_reasons: &[String]) {
    eprintln!(
        "Strict mode violated: {} dropped content warning(s)",
        dropped_reasons.len()
    );
    for reason in dropped_reasons {
        eprintln!("  - {reason}");
    }
}

fn check_directory_writable(dir: &Path) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::ErrorKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = std::process::id();
    for attempt in 0..16_u32 {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let probe_path = dir.join(format!(".rtfkit_write_test.{pid}.{timestamp}.{attempt}"));

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe_path)
        {
            Ok(_) => {
                let _ = fs::remove_file(&probe_path);
                return Ok(());
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "Failed to create unique probe file for write check",
    ))
}

/// Serialize IR Document to JSON file
fn emit_ir_to_file(document: &Document, path: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(document).context("Failed to serialize IR to JSON")?;

    fs::write(path, json)
        .with_context(|| format!("Failed to write IR file: {}", path.display()))?;

    eprintln!("IR written to: {}", path.display());
    Ok(())
}

/// Print report in JSON format to stdout
fn print_report_json(report: &Report) -> Result<()> {
    let json =
        serde_json::to_string_pretty(report).context("Failed to serialize report to JSON")?;
    println!("{json}");
    Ok(())
}

/// Print report in human-readable text format to stdout
fn print_report_text(report: &Report) {
    println!("=== Conversion Report ===");
    println!();
    println!("Statistics:");
    println!("  Paragraphs:  {}", report.stats.paragraph_count);
    println!("  Runs:        {}", report.stats.run_count);
    println!("  Bytes:       {}", report.stats.bytes_processed);
    println!("  Duration:    {}ms", report.stats.duration_ms);
    println!();

    if report.warnings.is_empty() {
        println!("Warnings: None");
    } else {
        println!("Warnings ({}):", report.warnings.len());
        for warning in &report.warnings {
            match warning {
                Warning::UnsupportedControlWord {
                    word, parameter, ..
                } => {
                    if let Some(param) = parameter {
                        println!("  - Unsupported control word: \\{}{}", word, param);
                    } else {
                        println!("  - Unsupported control word: \\{}", word);
                    }
                }
                Warning::UnknownDestination { destination, .. } => {
                    println!("  - Unknown destination: {}", destination);
                }
                Warning::DroppedContent {
                    reason, size_hint, ..
                } => {
                    if let Some(size) = size_hint {
                        println!("  - Dropped content: {} ({} bytes)", reason, size);
                    } else {
                        println!("  - Dropped content: {}", reason);
                    }
                }
                Warning::UnsupportedListControl { control_word, .. } => {
                    println!("  - Unsupported list control: \\{}", control_word);
                }
                Warning::UnresolvedListOverride { ls_id, .. } => {
                    println!("  - Unresolved list override: ls_id={}", ls_id);
                }
                Warning::UnsupportedNestingLevel { level, max, .. } => {
                    println!(
                        "  - Unsupported nesting level: {} (max supported: {})",
                        level, max
                    );
                }
                Warning::UnsupportedTableControl { control_word, .. } => {
                    println!("  - Unsupported table control: \\{}", control_word);
                }
                Warning::MalformedTableStructure { reason, .. } => {
                    println!("  - Malformed table structure: {}", reason);
                }
                Warning::UnclosedTableCell { .. } => {
                    println!("  - Unclosed table cell");
                }
                Warning::UnclosedTableRow { .. } => {
                    println!("  - Unclosed table row");
                }
                Warning::MergeConflict { reason, .. } => {
                    println!("  - Merge conflict: {}", reason);
                }
                Warning::TableGeometryConflict { reason, .. } => {
                    println!("  - Table geometry conflict: {}", reason);
                }
                Warning::UnsupportedField { reason, .. } => {
                    println!("  - Unsupported field (result preserved): {}", reason);
                }
            }
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::from(EXIT_CONVERSION_ERROR)
        }
    }
}
