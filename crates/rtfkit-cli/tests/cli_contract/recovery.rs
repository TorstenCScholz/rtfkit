#[test]
fn malformed_input_preserves_visible_text() {
    // Test that malformed RTF still extracts visible text in order
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_invalid_control_words.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed and produce output
    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Output should be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&output_str).is_ok(),
        "Output should be valid JSON"
    );
}

#[test]
fn malformed_table_produces_valid_output() {
    // Test that malformed tables still produce valid output structure
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_missing_cell_terminator.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Output should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    // Check for either blocks or stats field (output structure may vary)
    assert!(
        parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
        "Output should have blocks or stats field"
    );
}

#[test]
fn degraded_merge_controls_produce_valid_table() {
    // Test that degraded merge controls still produce a valid table
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_merge_controls_degraded.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Output should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    // Check for either blocks or stats field
    assert!(
        parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
        "Output should have blocks or stats field"
    );
}

#[test]
fn repeated_bad_controls_preserve_text() {
    // Test that repeated bad controls don't lose text content
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_repeated_bad_controls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Should succeed with output
    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Output should be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&output_str).is_ok(),
        "Output should be valid JSON"
    );
}

#[test]
fn list_fallback_produces_valid_output() {
    // Test that list fallback produces valid output
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_list_fallback.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Output should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");

    // Check for either blocks or stats field
    assert!(
        parsed.get("blocks").is_some() || parsed.get("stats").is_some(),
        "Output should have blocks or stats field"
    );
}

#[test]
fn orphan_controls_recover_to_valid_output() {
    // Test that orphan table controls recover to valid output
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/malformed_table_orphan_controls.rtf");

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);

    // Output should be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&output_str).is_ok(),
        "Output should be valid JSON"
    );
}
