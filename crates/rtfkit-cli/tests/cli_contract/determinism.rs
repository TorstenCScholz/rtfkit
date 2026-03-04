use serde_json::Value;

fn project_root() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn canonical_report_json(fixture_name: &str) -> Value {
    let fixture = project_root().join("fixtures").join(fixture_name);

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
    let output = cmd.assert().success().get_output().stdout.clone();

    let mut parsed: Value = serde_json::from_slice(&output).expect("CLI JSON output should parse");
    if let Some(stats) = parsed.get_mut("stats").and_then(|v| v.as_object_mut()) {
        // duration_ms depends on runtime scheduling and is not semantic output.
        stats.insert("duration_ms".to_string(), serde_json::json!(0));
    }
    parsed
}

fn assert_deterministic_report_json(fixture_name: &str, label: &str) {
    let output1 = canonical_report_json(fixture_name);
    let output2 = canonical_report_json(fixture_name);
    assert_eq!(output1, output2, "{label}");
}

#[test]
fn deterministic_output_for_simple_text() {
    assert_deterministic_report_json(
        "text_simple_paragraph.rtf",
        "JSON output should be deterministic for simple text",
    );
}

#[test]
fn deterministic_output_for_merge_tables() {
    assert_deterministic_report_json(
        "table_horizontal_merge_valid.rtf",
        "JSON output should be deterministic for merge tables",
    );
}

#[test]
fn deterministic_output_for_vertical_merge_tables() {
    assert_deterministic_report_json(
        "table_vertical_merge_valid.rtf",
        "JSON output should be deterministic for vertical merge tables",
    );
}

#[test]
fn deterministic_output_for_nested_lists() {
    assert_deterministic_report_json(
        "list_nested_two_levels.rtf",
        "JSON output should be deterministic for nested lists",
    );
}

#[test]
fn deterministic_output_for_mixed_content() {
    assert_deterministic_report_json(
        "mixed_complex.rtf",
        "JSON output should be deterministic for mixed content",
    );
}

#[test]
fn deterministic_output_for_degraded_content() {
    assert_deterministic_report_json(
        "malformed_table_merge_controls_degraded.rtf",
        "JSON output should be deterministic for degraded content",
    );
}
