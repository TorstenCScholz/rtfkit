use crate::support::cli::{run_cli_to_ir, run_rtfkit_multiple_times};
use crate::support::fs::fixture_dir;
use tempfile::TempDir;

// This module exists solely to document known variability.
// No tests are needed here - the documentation above serves as the record.

/// Verify that IR JSON does not contain timestamps.
/// This test ensures we don't accidentally introduce non-determinism.
#[test]
fn ir_json_contains_no_timestamps() {
    let fixture = "text_simple_paragraph.rtf";
    let temp_dir = TempDir::new().unwrap();
    let ir_path = run_cli_to_ir(fixture, &temp_dir, "timestamp_check");

    let content = std::fs::read_to_string(ir_path).expect("Failed to read IR file");

    // Check for common timestamp field names
    assert!(
        !content.contains("\"timestamp\""),
        "IR JSON should not contain timestamp fields"
    );
    assert!(
        !content.contains("\"created_at\""),
        "IR JSON should not contain created_at fields"
    );
    assert!(
        !content.contains("\"modified_at\""),
        "IR JSON should not contain modified_at fields"
    );
    assert!(
        !content.contains("\"date\""),
        "IR JSON should not contain date fields"
    );
    assert!(
        !content.contains("\"time\""),
        "IR JSON should not contain time fields"
    );
}

/// Verify that Report JSON does not contain timestamps.
#[test]
fn report_json_contains_no_timestamps() {
    let fixture = "text_simple_paragraph.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 1);
    let content = String::from_utf8_lossy(&outputs[0]);

    // Check for common timestamp field names
    assert!(
        !content.contains("\"timestamp\""),
        "Report JSON should not contain timestamp fields"
    );
    assert!(
        !content.contains("\"created_at\""),
        "Report JSON should not contain created_at fields"
    );
    assert!(
        !content.contains("\"modified_at\""),
        "Report JSON should not contain modified_at fields"
    );
    // Note: "date" and "time" might appear in warning reasons, so we don't check those
}
