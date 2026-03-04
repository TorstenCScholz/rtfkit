use std::fs;

use tempfile::tempdir;

#[test]
fn near_limit_cells_succeeds() {
    // Test that a table near the cell limit succeeds
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_cells_near_max.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();
}

#[test]
fn near_limit_depth_succeeds() {
    // Test that a document near the depth limit succeeds
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_depth_near_max.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();
}

#[test]
fn near_limit_rows_succeeds() {
    // Test that a table near the row limit succeeds
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_rows_near_max.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();
}

#[test]
fn near_limit_merge_succeeds() {
    // Test that a table near the merge span limit succeeds
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_merge_near_max.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();
}

#[test]
fn large_table_within_limits() {
    // Test that a large table (20 rows, 5 cells each) processes successfully
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_table_stress.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();
}

#[test]
fn large_table_produces_correct_structure() {
    // Test that a large table produces the correct number of rows and cells
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/limits_table_stress.rtf");

    let dir = tempdir().unwrap();
    let ir_output = dir.path().join("ir.json");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);
    cmd.assert().success();

    // Parse IR JSON and verify structure
    let json = fs::read_to_string(&ir_output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Check that we have a table block
    let blocks = parsed.get("blocks").unwrap().as_array().unwrap();
    assert_eq!(blocks.len(), 1, "Should have exactly one block");

    let table = blocks[0].get("rows").unwrap().as_array().unwrap();
    assert_eq!(table.len(), 20, "Should have 20 rows");

    // Check first and last row have 5 cells each
    let first_row = table[0].get("cells").unwrap().as_array().unwrap();
    assert_eq!(first_row.len(), 5, "First row should have 5 cells");

    let last_row = table[19].get("cells").unwrap().as_array().unwrap();
    assert_eq!(last_row.len(), 5, "Last row should have 5 cells");
}

#[test]
fn no_partial_output_after_limit_failure() {
    // Verify that no partial output is emitted after fatal limit failure
    let dir = tempdir().unwrap();
    let input = dir.path().join("too_wide.rtf");

    // Create RTF that exceeds cell limit
    let mut rtf = String::from("{\\rtf1\\ansi\\n\\trowd");
    for i in 1..=1001 {
        rtf.push_str(&format!("\\cellx{}", i * 1000));
    }
    rtf.push_str("\\n\\intbl ");
    for i in 1..=1001 {
        rtf.push_str(&format!("C{}\\cell ", i));
    }
    rtf.push_str("\\row\\n}");
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

    // Should fail with exit code 2
    cmd.assert().failure().code(2);

    // Stdout should be empty or contain only error info, not partial JSON
    let output = cmd.assert().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Should not be valid JSON with blocks (no partial output)
    if !output_str.is_empty() {
        // If there's output, it should not be a valid IR structure
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output_str) {
            assert!(
                parsed.get("blocks").is_none(),
                "Should not emit partial blocks after fatal limit failure"
            );
        }
    }
}
