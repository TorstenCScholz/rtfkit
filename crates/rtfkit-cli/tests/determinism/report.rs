use crate::support::cli::{run_rtfkit_multiple_times, verify_identical_outputs};
use crate::support::fs::fixture_dir;

/// Test Report JSON determinism for simple text.
/// Simple case - report should be consistent for simple input.
#[test]
fn report_simple_text_is_deterministic() {
    let fixture = "text_simple_paragraph.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for simple text should be byte-identical across runs"
    );
}

/// Test Report JSON determinism for malformed input with warnings.
/// Warning ordering and content must be stable.
#[test]
fn report_malformed_input_warnings_are_stable() {
    let fixture = "malformed_table_merge_controls_degraded.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for malformed input should have stable warning ordering"
    );
}

/// Test Report JSON determinism for invalid control words.
/// Multiple warnings must be consistently ordered.
#[test]
fn report_invalid_control_words_warnings_are_stable() {
    let fixture = "malformed_invalid_control_words.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for invalid control words should have stable warning ordering"
    );
}

/// Test Report JSON determinism for repeated bad controls.
/// Large numbers of warnings must be consistently ordered.
#[test]
fn report_repeated_bad_controls_warnings_are_stable() {
    let fixture = "malformed_repeated_bad_controls.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for repeated bad controls should have stable warning ordering"
    );
}

/// Test Report JSON determinism for orphan controls.
/// Table-related warnings must be consistently ordered.
#[test]
fn report_orphan_controls_warnings_are_stable() {
    let fixture = "malformed_table_orphan_controls.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for orphan controls should have stable warning ordering"
    );
}

/// Test Report JSON determinism for list fallback.
/// List-related warnings must be consistently ordered.
#[test]
fn report_list_fallback_warnings_are_stable() {
    let fixture = "malformed_list_fallback.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for list fallback should have stable warning ordering"
    );
}

/// Test Report JSON determinism for missing cell terminator.
/// Table structure warnings must be consistently ordered.
#[test]
fn report_missing_cell_terminator_warnings_are_stable() {
    let fixture = "malformed_table_missing_cell_terminator.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for missing cell terminator should have stable warning ordering"
    );
}

/// Test Report JSON determinism for missing row terminator.
/// Table structure warnings must be consistently ordered.
#[test]
fn report_missing_row_terminator_warnings_are_stable() {
    let fixture = "malformed_table_missing_row_terminator.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for missing row terminator should have stable warning ordering"
    );
}

/// Test Report JSON determinism for conflicting merge.
/// Merge conflict warnings must be consistently ordered.
#[test]
fn report_conflicting_merge_warnings_are_stable() {
    let fixture = "malformed_table_conflicting_merge.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for conflicting merge should have stable warning ordering"
    );
}

/// Test Report JSON determinism for mixed complex content.
/// Complex documents must have stable report output.
#[test]
fn report_mixed_complex_is_deterministic() {
    let fixture = "mixed_complex.rtf";
    let fixture_path = fixture_dir().join(fixture);
    let args = [
        "convert",
        fixture_path.to_str().unwrap(),
        "--format",
        "json",
    ];

    let outputs = run_rtfkit_multiple_times(&args, 3);

    assert!(
        verify_identical_outputs(&outputs),
        "Report JSON for mixed complex content should be byte-identical across runs"
    );
}
