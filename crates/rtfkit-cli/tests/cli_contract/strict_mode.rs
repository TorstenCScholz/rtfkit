use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn strict_mode_fails_even_if_warning_cap_is_hit_first() {
    // Test that strict-mode signal is preserved even when warnings are capped
    let dir = tempdir().unwrap();
    let input = dir.path().join("warning_cap_then_drop.rtf");

    let mut rtf = String::from(r"{\rtf1\ansi ");
    for i in 0..1100 {
        rtf.push_str(&format!("\\unknown{i} "));
    }
    rtf.push_str(r"{\*\pict hidden} visible}");
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
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
fn warning_cap_preserves_strict_mode_signal() {
    // Test that strict-mode signal is preserved even when warnings are capped
    let dir = tempdir().unwrap();
    let input = dir.path().join("warning_cap_strict.rtf");

    // Create RTF with many unknown keywords (to hit warning cap) AND dropped content
    let mut rtf = String::from(r"{\rtf1\ansi ");
    for i in 0..1100 {
        rtf.push_str(&format!("\\unknown{i} "));
    }
    // Add unresolved list reference that causes DroppedContent
    rtf.push_str(r"\ls999 dropped list item}");
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    // Should fail with exit code 4 (strict violation), not 0 (success)
    cmd.assert()
        .failure()
        .code(4)
        .stderr(contains("Strict mode violated"));
}

#[test]
fn dropped_content_always_causes_strict_failure() {
    // Ensure DroppedContent warnings always cause strict-mode failure regardless of count
    let dir = tempdir().unwrap();
    let input = dir.path().join("multiple_dropped.rtf");

    // Create RTF with multiple dropped content scenarios
    let rtf = r#"{\rtf1\ansi {\*\unknown1 hidden1} {\*\unknown2 hidden2} \ls999 list item}"#;
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
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
fn strict_mode_fails_on_dropped_content_from_list() {
    // Test that DroppedContent warning causes strict-mode failure
    let dir = tempdir().unwrap();
    let input = dir.path().join("dropped_list_content.rtf");
    // Create RTF with unresolved list reference that causes content to be dropped
    fs::write(&input, r#"{\rtf1\ansi \ls999 Item with dropped content}"#).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
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
fn non_strict_mode_succeeds_with_warnings_on_unresolved_ls() {
    // Test that non-strict mode succeeds but emits warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_list_unresolved_ls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Warnings are in JSON output (stdout), not stderr
    cmd.assert().success().stdout(contains("dropped_content"));
}

#[test]
fn table_merge_controls_non_strict_succeeds() {
    // Degraded merge controls should succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_merge_controls_degraded.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Should succeed (exit code 0) with warnings in output
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn table_missing_cell_terminator_non_strict_succeeds() {
    // Missing cell terminator should succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Should succeed (exit code 0) with warnings in output
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn merge_conflict_non_strict_succeeds_with_warnings() {
    // Test that merge conflicts succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_merge_continuation.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    // Should succeed with warnings in output
    cmd.assert()
        .success()
        .stdout(contains("merge_conflict"))
        .stdout(contains("dropped_content"));
}

#[test]
fn table_geometry_conflict_causes_strict_mode_failure() {
    // Test that table geometry conflicts (span exceeding bounds) cause strict mode failure
    let dir = tempdir().unwrap();
    let input = dir.path().join("geometry_conflict.rtf");

    // Create RTF with merge span exceeding available cells
    // This should trigger TableGeometryConflict + DroppedContent
    // Note: clmrg must come before cellx in the row definition
    let rtf = r#"{\rtf1\ansi
\trowd\clmgf\cellx2880\clmrg\cellx5760\clmrg\cellx8640\clmrg\cellx11520
\intbl Cell 1\cell \cell \cell\row
}"#;
    fs::write(&input, rtf).unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        input.to_str().unwrap(),
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
fn horizontal_merge_valid_strict_succeeds() {
    // Valid horizontal merge should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_horizontal_merge_valid.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn vertical_merge_valid_strict_succeeds() {
    // Valid vertical merge should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_vertical_merge_valid.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn mixed_merge_strict_succeeds() {
    // Mixed merge should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_mixed_merge.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn conflicting_merge_strict_succeeds() {
    // Conflicting merge chains should succeed (they are resolved deterministically)
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_conflicting_merge.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}

#[test]
fn non_monotonic_cellx_non_strict_succeeds() {
    // Non-monotonic cellx should succeed in non-strict mode with warnings
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_non_monotonic_cellx.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    cmd.assert().success().stdout(contains("warnings"));
}

#[test]
fn strict_mode_allows_mergefield_when_result_text_preserved() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/field_mergefield_preserve_result.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

#[test]
fn strict_mode_allows_ref_when_fallback_text_preserved() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/field_ref_unresolved_target.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success().code(0);
}

#[test]
fn prose_interleave_strict_succeeds() {
    // Prose/table interleave should succeed in strict mode
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/table_prose_interleave.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args([
        "convert",
        fixture.to_str().unwrap(),
        "--strict",
        "--format",
        "json",
    ]);
    cmd.assert().success();
}
