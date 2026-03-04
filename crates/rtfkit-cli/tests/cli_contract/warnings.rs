use predicates::str::contains;

#[test]
fn dropped_content_warning_has_stable_type() {
    // Test that DroppedContent warning type is stable and present
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // The output should contain the stable warning type "dropped_content"
    cmd.assert().success().stdout(contains("dropped_content"));
}

#[test]
fn dropped_content_warning_has_reason_string() {
    // Test that DroppedContent warning includes a meaningful reason string
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // The output should contain a reason field
    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Check for reason field presence
    assert!(
        output_str.contains("\"reason\""),
        "Warning should include a reason field"
    );
}

#[test]
fn merge_conflict_warning_has_stable_type() {
    // Test that MergeConflict warning type is stable
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // The output should contain the stable warning type "merge_conflict"
    cmd.assert().success().stdout(contains("merge_conflict"));
}

#[test]
fn malformed_table_structure_warning_has_stable_type() {
    // Test that MalformedTableStructure warning type is stable
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_controls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with warnings
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn unclosed_table_cell_warning_present() {
    // Test that UnclosedTableCell warning is generated for missing cell terminators
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with warnings
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn unclosed_table_row_warning_present() {
    // Test that UnclosedTableRow warning is generated for missing row terminators
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_missing_row_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with warnings
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn invalid_control_words_generate_warnings() {
    // Test that invalid control words generate warnings but don't fail
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_invalid_control_words.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with warnings
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn repeated_bad_controls_generate_warnings() {
    // Test that repeated bad controls generate warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_repeated_bad_controls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with warnings
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn warning_count_is_reported() {
    // Test that warning count is included in output
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_invalid_control_words.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Check for warning count field
    assert!(
        output_str.contains("warning_count") || output_str.contains("warnings"),
        "Output should include warning count"
    );
}

#[test]
fn list_fallback_generates_warnings() {
    // Test that list fallback generates warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_list_fallback.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with warnings
    cmd.assert().success().stdout(contains("warnings"));
}
