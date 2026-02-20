use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rtfkit_core::{Document, Interpreter, Report, Warning};
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

        /// Output file path (reserved for future DOCX writer)
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
        } => handle_convert(
            input,
            output.as_ref(),
            &to,
            &format,
            emit_ir.as_ref(),
            strict,
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
) -> Result<ExitCode> {
    if output.is_some() {
        bail!("--output is not supported yet in v0.1 (DOCX writer is planned for a later phase)");
    }

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

    // Output report to stdout
    match format {
        "json" => print_report_json(&report)?,
        "text" => print_report_text(&report),
        _ => unreachable!("clap validates format"),
    }

    Ok(ExitCode::from(EXIT_SUCCESS))
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
