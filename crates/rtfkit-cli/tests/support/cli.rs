#![allow(dead_code)]

use std::path::PathBuf;

use tempfile::TempDir;

use super::fs::fixture_dir;

/// Run the CLI to convert an RTF file to DOCX.
/// Returns the path to the generated DOCX file.
pub fn run_cli_convert(fixture_name: &str, temp_dir: &TempDir) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join("output.docx");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);

    let output_result = cmd.output().expect("Failed to run CLI");

    if !output_result.status.success() {
        panic!(
            "CLI failed for fixture '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        );
    }

    assert!(output.exists(), "Output DOCX file should be created");
    output
}

/// Run the CLI to convert an RTF file to DOCX with a suffix for determinism tests.
/// Returns the path to the generated DOCX file.
pub fn run_cli_convert_determinism(
    fixture_name: &str,
    temp_dir: &TempDir,
    suffix: &str,
) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("output_{suffix}.docx"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);

    let output_result = cmd.output().expect("Failed to run CLI");

    if !output_result.status.success() {
        panic!(
            "CLI failed for fixture '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        );
    }

    assert!(output.exists(), "Output DOCX file should be created");
    output
}

/// Run the CLI to convert an RTF file to DOCX (determinism variant with suffix).
/// Returns the path to the generated DOCX file.
pub fn run_cli_to_docx(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("output_{suffix}.docx"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--force",
    ]);

    let output_result = cmd.output().expect("Failed to run CLI");

    if !output_result.status.success() {
        panic!(
            "CLI failed for fixture '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        );
    }

    assert!(output.exists(), "Output DOCX file should be created");
    output
}

/// Run the CLI to emit IR JSON to a file.
/// Returns the path to the generated IR JSON file.
pub fn run_cli_to_ir(fixture_name: &str, temp_dir: &TempDir, suffix: &str) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir.path().join(format!("ir_{suffix}.json"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--emit-ir",
        output.to_str().unwrap(),
    ]);

    let output_result = cmd.output().expect("Failed to run CLI");

    if !output_result.status.success() {
        panic!(
            "CLI failed for fixture '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            String::from_utf8_lossy(&output_result.stdout),
            String::from_utf8_lossy(&output_result.stderr)
        );
    }

    assert!(output.exists(), "IR JSON file should be created");
    output
}

/// Run the CLI to convert an RTF file to DOCX with a style profile.
/// Returns the path to the generated DOCX file.
pub fn run_cli_convert_with_profile(
    fixture_name: &str,
    temp_dir: &TempDir,
    profile: &str,
    suffix: &str,
) -> PathBuf {
    let input = fixture_dir().join(fixture_name);
    let output = temp_dir
        .path()
        .join(format!("output_{suffix}_{profile}.docx"));

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "-o",
        output.to_str().unwrap(),
        "--style-profile",
        profile,
        "--force",
    ]);

    let result = cmd.output().expect("Failed to run CLI");
    if !result.status.success() {
        panic!(
            "CLI failed for fixture '{}' with profile '{}':\nstdout: {}\nstderr: {}",
            fixture_name,
            profile,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    assert!(output.exists(), "Output DOCX file should be created");
    output
}

/// Run rtfkit multiple times with the given arguments and collect outputs.
/// Returns a vector of stdout bytes (one per run).
pub fn run_rtfkit_multiple_times(args: &[&str], runs: usize) -> Vec<Vec<u8>> {
    let mut outputs = Vec::with_capacity(runs);

    for _ in 0..runs {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(args);

        let output = cmd.output().expect("Failed to execute rtfkit");

        if !output.status.success() {
            panic!(
                "CLI failed with args {:?}:\nstdout: {}\nstderr: {}",
                args,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        outputs.push(output.stdout);
    }

    outputs
}

/// Verify that all outputs are byte-identical.
/// Returns true if all outputs match the first one.
pub fn verify_identical_outputs(outputs: &[Vec<u8>]) -> bool {
    if outputs.is_empty() {
        return true;
    }

    let first = &outputs[0];
    outputs.iter().all(|output| output == first)
}
