use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rtfkit_core::{Document, Interpreter, Report, Warning};
use rtfkit_docx::write_docx;
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

#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert an RTF file to another format
    Convert {
        /// Input RTF file to convert
        #[arg(required = true)]
        input: PathBuf,

        /// Output file path (e.g., output.docx)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output target (currently only docx is accepted)
        #[arg(long, default_value = "docx", value_parser = ["docx"])]
        to: String,

        /// Output format for conversion report
        #[arg(long, default_value = "text", value_parser = ["json", "text"])]
        format: String,

        /// Serialize IR to JSON file for analysis/debugging
        #[arg(long, value_name = "FILE")]
        emit_ir: Option<PathBuf>,

        /// Fail on unsupported features (DroppedContent warnings)
        #[arg(long)]
        strict: bool,

        /// Overwrite existing output file without prompting
        #[arg(long)]
        force: bool,
    },
}

/// Exit codes as defined in PHASE1.md
const EXIT_SUCCESS: u8 = 0;
const EXIT_PARSE_ERROR: u8 = 2;
const EXIT_CONVERSION_ERROR: u8 = 3;
const EXIT_STRICT_MODE: u8 = 4;

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
        } => handle_convert(
            input,
            output.as_ref(),
            &to,
            &format,
            emit_ir.as_ref(),
            strict,
            force,
        ),
    }
}

fn handle_convert(
    input: PathBuf,
    output: Option<&PathBuf>,
    to: &str,
    format: &str,
    emit_ir: Option<&PathBuf>,
    strict: bool,
    force: bool,
) -> Result<ExitCode> {
    debug!("Target format requested: {to}");

    // Read input file
    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read file: {}", input.display()))?;

    // Parse RTF using the Interpreter
    let (document, report) = match Interpreter::parse(&content) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Parse error: {e}");
            return Ok(ExitCode::from(EXIT_PARSE_ERROR));
        }
    };

    debug!("Parsed document with {} blocks", document.blocks.len());
    debug!("Report: {} warnings", report.warnings.len());

    // Check for strict mode violations
    if strict {
        let dropped_content: Vec<&Warning> = report
            .warnings
            .iter()
            .filter(|w| matches!(w, Warning::DroppedContent { .. }))
            .collect();

        if !dropped_content.is_empty() {
            eprintln!(
                "Strict mode violated: {} dropped content warning(s)",
                dropped_content.len()
            );
            for warning in &dropped_content {
                if let Warning::DroppedContent { reason, .. } = warning {
                    eprintln!("  - {reason}");
                }
            }
            return Ok(ExitCode::from(EXIT_STRICT_MODE));
        }
    }

    // Emit IR if requested
    if let Some(ir_path) = emit_ir {
        emit_ir_to_file(&document, ir_path)?;
    }

    // Handle DOCX output if --output is specified
    if let Some(output_path) = output {
        return handle_docx_output(&document, output_path, force);
    }

    // Output report to stdout (when no --output specified)
    match format {
        "json" => print_report_json(&report)?,
        "text" => print_report_text(&report),
        _ => unreachable!("clap validates format"),
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
}

/// Handle writing DOCX output with validation and error handling
fn handle_docx_output(document: &Document, output_path: &PathBuf, force: bool) -> Result<ExitCode> {
    // Validate .docx extension
    let extension = output_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase());

    match extension.as_deref() {
        Some("docx") => {}
        Some(other) => {
            eprintln!(
                "Warning: Output file has '.{other}' extension, but DOCX format will be written."
            );
            eprintln!("         Consider using '.docx' extension for clarity.");
        }
        None => {
            eprintln!("Warning: Output file has no extension. DOCX format will be written.");
            eprintln!("         Consider using '.docx' extension for clarity.");
        }
    }

    // Check if output directory exists and is writable
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            eprintln!(
                "Error: Output directory does not exist: {}",
                parent.display()
            );
            return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
        }

        // Check directory writability using a unique probe filename.
        match check_directory_writable(parent) {
            Ok(()) => {}
            Err(e) => {
                eprintln!(
                    "Error: Output directory is not writable: {}",
                    parent.display()
                );
                eprintln!("  {e}");
                return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
            }
        }
    }

    // Check if file exists and --force is not set
    if output_path.exists() && !force {
        eprintln!(
            "Error: Output file already exists: {}",
            output_path.display()
        );
        eprintln!("       Use --force to overwrite existing files.");
        return Ok(ExitCode::from(EXIT_CONVERSION_ERROR));
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
