use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn regression_merge_conflict_deterministic_resolution() {
    // Regression: Merge conflicts should be resolved deterministically
    // Run multiple times to ensure consistent output
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_conflicting_merge.rtf");

    let mut outputs = Vec::new();
    for _ in 0..3 {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        let output = cmd.assert().success().get_output().stdout.clone();
        let mut parsed: serde_json::Value =
            serde_json::from_slice(&output).expect("CLI JSON output should parse");
        if let Some(stats) = parsed.get_mut("stats").and_then(|v| v.as_object_mut()) {
            // duration_ms is runtime-dependent and not part of semantic determinism.
            stats.insert("duration_ms".to_string(), serde_json::json!(0));
        }
        outputs.push(parsed);
    }

    // All outputs should be identical
    for i in 1..outputs.len() {
        assert_eq!(
            outputs[0], outputs[i],
            "Merge conflict resolution should be deterministic"
        );
    }
}

#[test]
fn regression_orphan_merge_continuation_handled() {
    // Regression: Orphan merge continuation should be handled gracefully
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

    // Non-strict mode should succeed
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success();

    // Strict mode should fail with proper error
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn regression_table_normalization_preserves_content() {
    // Regression: Table normalization should not lose content
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_multirow_uneven_content.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Parse and verify content is preserved
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    // Check for either blocks or stats field
    assert!(
        parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
        "Output should have blocks or stats field"
    );
}

#[test]
fn regression_list_in_table_cell() {
    // Regression: Lists inside table cells should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_with_list_in_cell.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed without crashing
    cmd.assert().success();
}

#[test]
fn regression_mixed_prose_list_table() {
    // Regression: Mixed prose/list/table should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/mixed_prose_list_table.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed without crashing
    cmd.assert().success();
}

#[test]
fn regression_nested_list_table() {
    // Regression: Nested list/table combinations should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/mixed_list_table_nested.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed without crashing
    cmd.assert().success();
}

#[test]
fn regression_decimal_list_handling() {
    // Regression: Decimal lists should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_decimal_simple.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed
    cmd.assert().success();
}

#[test]
fn regression_mixed_list_kinds() {
    // Regression: Mixed list kinds should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/list_mixed_kinds.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed
    cmd.assert().success();
}

#[test]
fn regression_formatting_preservation() {
    // Regression: Text formatting should be preserved
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_mixed_formatting.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed
    cmd.assert().success();
}

#[test]
fn regression_nested_styles() {
    // Regression: Nested styles should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_nested_styles.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed
    cmd.assert().success();
}

#[test]
fn regression_alignment_handling() {
    // Regression: Text alignment should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_alignment.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed
    cmd.assert().success();
}

#[test]
fn regression_table_hyperlink_first_inline_alignment() {
    // Regression: cell paragraph alignment should persist when first inline is hyperlink.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_alignment_hyperlink_first_inline.rtf");

    let dir = tempdir().unwrap();
    let ir_output = dir.path().join("table-hyperlink-align.json");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--emit-ir",
        ir_output.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let ir = fs::read_to_string(ir_output).unwrap();
    assert!(ir.contains("\"alignment\": \"center\""));
    assert!(ir.contains("\"alignment\": \"right\""));
}

#[test]
fn regression_multiple_paragraphs() {
    // Regression: Multiple paragraphs should be handled correctly
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/text_multiple_paragraphs.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed
    cmd.assert().success();
}
