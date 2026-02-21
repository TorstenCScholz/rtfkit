//! Limits and Safety Test Matrix
//!
//! This test module validates the fail-closed behavior and resource protection
//! limits of rtfkit. Tests are organized into:
//!
//! - `near_limit_success`: Tests that inputs just under limits succeed
//! - `over_limit_failure`: Tests that inputs exceeding limits fail with exit code 2
//! - `no_partial_output`: Tests that no output is emitted after limit failure
//! - `safety_tests`: Tests for no panic/unbounded loops under malformed input
//!
//! ## Limits Under Test
//!
//! | Limit | Default | Description |
//! |-------|---------|-------------|
//! | `max_input_bytes` | 10 MB | Maximum input file size |
//! | `max_group_depth` | 256 | Maximum nesting depth |
//! | `max_warning_count` | 1000 | Maximum warnings before stopping |
//! | `max_rows_per_table` | 10,000 | Maximum rows per table |
//! | `max_cells_per_row` | 1,000 | Maximum cells per row |
//! | `max_merge_span` | 1,000 | Maximum merge span |

use std::fs;

use predicates::str::contains;
use tempfile::tempdir;

// =============================================================================
// NEAR-LIMIT SUCCESS TESTS
// =============================================================================
//
// Tests that inputs just under limits succeed with exit code 0.
// These verify that the limits don't trigger prematurely.

mod near_limit_success {
    use super::*;

    // -------------------------------------------------------------------------
    // max_cells_per_row Tests
    // -------------------------------------------------------------------------

    #[test]
    fn cells_near_limit_succeeds() {
        // Test that a table with cells just under max_cells_per_row (1000) succeeds
        let dir = tempdir().unwrap();
        let input = dir.path().join("near_cells.rtf");

        // Create RTF with 999 cells (just under limit of 1000)
        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=999 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=999 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    #[test]
    fn cells_near_limit_fixture_succeeds() {
        // Test using the fixture file
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_cells_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    // -------------------------------------------------------------------------
    // max_rows_per_table Tests
    // -------------------------------------------------------------------------

    #[test]
    fn rows_near_limit_succeeds() {
        // Test that a table with rows just under max_rows_per_table (10000) succeeds
        // Note: We use a smaller count for test performance, but the principle is validated
        let dir = tempdir().unwrap();
        let input = dir.path().join("near_rows.rtf");

        // Create RTF with 100 rows (representative test - actual near-limit would be 9999)
        let mut rtf = String::from("{\\rtf1\\ansi\n");
        for i in 1..=100 {
            rtf.push_str(&format!("\\trowd\\cellx1000\\intbl R{}\\cell\\row\n", i));
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    #[test]
    fn rows_near_limit_fixture_succeeds() {
        // Test using the fixture file
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_rows_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    // -------------------------------------------------------------------------
    // max_group_depth Tests
    // -------------------------------------------------------------------------

    #[test]
    fn depth_near_limit_succeeds() {
        // Test that a document with group depth just under max_group_depth (256) succeeds
        let dir = tempdir().unwrap();
        let input = dir.path().join("near_depth.rtf");

        // Create RTF with 255 nested groups (just under limit of 256)
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..255 {
            rtf.push('{');
        }
        rtf.push_str("deeply nested text");
        for _ in 0..255 {
            rtf.push('}');
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    #[test]
    fn depth_near_limit_fixture_succeeds() {
        // Test using the fixture file
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_depth_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    // -------------------------------------------------------------------------
    // max_merge_span Tests
    // -------------------------------------------------------------------------

    #[test]
    fn merge_near_limit_succeeds() {
        // Test that a table with merge span just under max_merge_span (1000) succeeds
        // Note: We use a smaller count for test practicality
        let dir = tempdir().unwrap();
        let input = dir.path().join("near_merge.rtf");

        // Create RTF with a 10-cell merge (representative test)
        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd\\clmgf");
        for i in 1..=10 {
            rtf.push_str(&format!("\\cellx{}", i * 1000));
            if i > 1 {
                rtf.push_str("\\clmrg");
            }
        }
        rtf.push_str("\n\\intbl Merged\\cell");
        for _ in 1..10 {
            rtf.push_str("\\cell");
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    #[test]
    fn merge_near_limit_fixture_succeeds() {
        // Test using the fixture file
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_merge_near_max.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    // -------------------------------------------------------------------------
    // max_input_bytes Tests
    // -------------------------------------------------------------------------

    #[test]
    fn input_size_under_limit_succeeds() {
        // Test that a reasonably sized input succeeds
        let dir = tempdir().unwrap();
        let input = dir.path().join("normal_size.rtf");

        // Create a normal-sized RTF (well under 10 MB limit)
        let rtf = format!("{{\\rtf1\\ansi {} }}", "Hello, World! ".repeat(1000));
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    // -------------------------------------------------------------------------
    // max_warning_count Tests
    // -------------------------------------------------------------------------

    #[test]
    fn warnings_under_limit_succeeds() {
        // Test that a document with warnings under max_warning_count (1000) succeeds
        let dir = tempdir().unwrap();
        let input = dir.path().join("some_warnings.rtf");

        // Create RTF with 100 unknown control words (well under limit of 1000)
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for i in 0..100 {
            rtf.push_str(&format!("\\unknown{} ", i));
        }
        rtf.push_str("text}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }

    // -------------------------------------------------------------------------
    // Stress Tests
    // -------------------------------------------------------------------------

    #[test]
    fn large_table_stress_succeeds() {
        // Test that a large table (20 rows, 5 cells each) processes successfully
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap().parent().unwrap();
        let fixture = project_root.join("fixtures/limits_table_stress.rtf");

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);
        cmd.assert().success().code(0);
    }
}

// =============================================================================
// OVER-LIMIT FAILURE TESTS
// =============================================================================
//
// Tests that inputs exceeding limits fail with exit code 2 (parse error).
// These verify that the limits are enforced correctly.

mod over_limit_failure {
    use super::*;

    // -------------------------------------------------------------------------
    // max_cells_per_row Tests
    // -------------------------------------------------------------------------

    #[test]
    fn cells_exceed_limit_fails() {
        // Test that a table with cells exceeding max_cells_per_row (1000) fails
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_many_cells.rtf");

        // Create RTF with 1001 cells (exceeds limit of 1000)
        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("Parse error"));
    }

    // -------------------------------------------------------------------------
    // max_rows_per_table Tests
    // -------------------------------------------------------------------------

    #[test]
    fn rows_exceed_limit_fails() {
        // Test that a table with rows exceeding max_rows_per_table (10000) fails
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_many_rows.rtf");

        // Create RTF with 10001 rows (exceeds limit of 10000)
        let mut rtf = String::from("{\\rtf1\\ansi\n");
        for i in 1..=10001 {
            rtf.push_str(&format!("\\trowd\\cellx1000\\intbl R{}\\cell\\row\n", i));
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("Parse error"));
    }

    // -------------------------------------------------------------------------
    // max_group_depth Tests
    // -------------------------------------------------------------------------

    #[test]
    fn depth_exceed_limit_fails() {
        // Test that a document with group depth exceeding max_group_depth (256) fails
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_deep.rtf");

        // Create RTF with 257 nested groups (exceeds limit of 256)
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..257 {
            rtf.push('{');
        }
        rtf.push_str("too deep");
        for _ in 0..257 {
            rtf.push('}');
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("Parse error"));
    }

    // -------------------------------------------------------------------------
    // max_input_bytes Tests
    // -------------------------------------------------------------------------

    #[test]
    fn input_size_exceed_limit_fails() {
        // Test that input exceeding max_input_bytes fails.
        // Default is 10 MiB (10,485,760 bytes), so write a slightly larger file.
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_large.rtf");

        const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;
        let payload_len = MAX_INPUT_BYTES + 1;
        let oversized_rtf = format!("{{\\rtf1\\ansi {} }}", "A".repeat(payload_len));
        fs::write(&input, oversized_rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert()
            .failure()
            .code(2)
            .stderr(contains("Input too large"));
    }

    // -------------------------------------------------------------------------
    // Error Message Quality Tests
    // -------------------------------------------------------------------------

    #[test]
    fn cells_limit_error_message_is_clear() {
        // Verify that the error message indicates which limit was exceeded
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_many_cells.rtf");

        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().failure().code(2).stderr(contains("cells")); // Error message should mention cells
    }

    #[test]
    fn depth_limit_error_message_is_clear() {
        // Verify that the error message indicates which limit was exceeded
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_deep.rtf");

        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..257 {
            rtf.push('{');
        }
        rtf.push_str("too deep");
        for _ in 0..257 {
            rtf.push('}');
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().failure().code(2).stderr(contains("depth")); // Error message should mention depth
    }
}

// =============================================================================
// NO PARTIAL OUTPUT TESTS
// =============================================================================
//
// Tests that verify no partial output is emitted after fatal limit failures.
// This ensures fail-closed behavior.

mod no_partial_output {
    use super::*;

    #[test]
    fn no_output_after_cells_limit_failure() {
        // Verify that no partial output is emitted after cells limit failure
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_wide.rtf");

        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
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

    #[test]
    fn no_output_after_rows_limit_failure() {
        // Verify that no partial output is emitted after rows limit failure
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_many_rows.rtf");

        let mut rtf = String::from("{\\rtf1\\ansi\n");
        for i in 1..=10001 {
            rtf.push_str(&format!("\\trowd\\cellx1000\\intbl R{}\\cell\\row\n", i));
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        cmd.assert().failure().code(2);

        let output = cmd.assert().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Should not emit partial blocks
        if !output_str.is_empty()
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output_str)
        {
            assert!(
                parsed.get("blocks").is_none(),
                "Should not emit partial blocks after rows limit failure"
            );
        }
    }

    #[test]
    fn no_output_after_depth_limit_failure() {
        // Verify that no partial output is emitted after depth limit failure
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_deep.rtf");

        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..257 {
            rtf.push('{');
        }
        rtf.push_str("too deep");
        for _ in 0..257 {
            rtf.push('}');
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        cmd.assert().failure().code(2);

        let output = cmd.assert().get_output().stdout.clone();
        let output_str = String::from_utf8_lossy(&output);

        // Should not emit partial blocks
        if !output_str.is_empty()
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output_str)
        {
            assert!(
                parsed.get("blocks").is_none(),
                "Should not emit partial blocks after depth limit failure"
            );
        }
    }

    #[test]
    fn no_docx_output_after_limit_failure() {
        // Verify that no DOCX output file is created after limit failure
        let dir = tempdir().unwrap();
        let input = dir.path().join("too_wide.rtf");
        let output = dir.path().join("out.docx");

        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args([
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ]);

        cmd.assert().failure().code(2);

        // Output file should not exist
        assert!(
            !output.exists(),
            "No DOCX output should be created after limit failure"
        );
    }
}

// =============================================================================
// SAFETY TESTS
// =============================================================================
//
// Tests for no panic/unbounded loops under malformed input.
// These verify robustness against denial-of-service attempts.

mod safety_tests {
    use super::*;

    #[test]
    fn no_panic_on_malformed_high_volume_input() {
        // Verify no panic under malformed high-volume input
        let dir = tempdir().unwrap();
        let input = dir.path().join("malformed.rtf");

        // Create RTF with many malformed control words
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for i in 0..1000 {
            rtf.push_str(&format!("\\malformed{} ", i));
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should succeed (malformed controls are warnings, not errors)
        // or fail gracefully, but never panic
        cmd.assert().success();
    }

    #[test]
    fn no_panic_on_deeply_nested_malformed() {
        // Verify no panic on deeply nested malformed structures
        let dir = tempdir().unwrap();
        let input = dir.path().join("nested_malformed.rtf");

        // Create RTF with deeply nested but malformed content
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for i in 0..50 {
            rtf.push_str(&format!("{{\\invalid{} ", i));
        }
        rtf.push_str("content");
        for _ in 0..50 {
            rtf.push('}');
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should not panic
        let result = cmd.assert();
        // Either success or graceful failure is acceptable
        let code = result.get_output().status.code().unwrap();
        assert!(code == 0 || code == 2 || code == 4, "Should not panic");
    }

    #[test]
    fn no_unbounded_loop_on_repeated_groups() {
        // Verify no unbounded loop on repeated group open/close
        let dir = tempdir().unwrap();
        let input = dir.path().join("repeated_groups.rtf");

        // Create RTF with many group open/close
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..1000 {
            rtf.push_str("{}");
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should complete in reasonable time (no infinite loop)
        cmd.assert().success();
    }

    #[test]
    fn no_unbounded_loop_on_unclosed_groups() {
        // Verify behavior on unclosed groups
        let dir = tempdir().unwrap();
        let input = dir.path().join("unclosed.rtf");

        // Create RTF with unclosed groups
        let rtf = "{\\rtf1\\ansi {\\b {\\i {\\ul text}}}";
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should complete (either succeed with recovery or fail gracefully)
        let result = cmd.assert();
        let code = result.get_output().status.code().unwrap();
        assert!(code == 0 || code == 2, "Should not panic or hang");
    }

    #[test]
    fn no_panic_on_empty_input() {
        // Verify no panic on empty input
        let dir = tempdir().unwrap();
        let input = dir.path().join("empty.rtf");
        fs::write(&input, "").unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should fail gracefully with parse error
        cmd.assert().failure().code(2);
    }

    #[test]
    fn no_panic_on_binary_garbage() {
        // Verify no panic on binary garbage input
        let dir = tempdir().unwrap();
        let input = dir.path().join("garbage.rtf");

        // Create file with random binary data
        let garbage: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        fs::write(&input, garbage).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should fail gracefully - binary garbage returns exit code 3 (IO failure)
        // because the file cannot be read as valid UTF-8
        let result = cmd.assert();
        let code = result.get_output().status.code().unwrap();
        // Exit code 2 (parse error) or 3 (IO failure) are both acceptable
        assert!(
            code == 2 || code == 3,
            "Should not panic, got exit code {}",
            code
        );
    }

    #[test]
    fn no_panic_on_null_bytes() {
        // Verify no panic on input with null bytes
        let dir = tempdir().unwrap();
        let input = dir.path().join("nulls.rtf");

        // Create file with null bytes embedded
        let content = b"{\\rtf1\0\\ansi test}";
        fs::write(&input, content).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should fail gracefully
        let result = cmd.assert();
        let code = result.get_output().status.code().unwrap();
        assert!(code == 0 || code == 2, "Should not panic");
    }

    #[test]
    fn no_panic_on_very_long_control_word() {
        // Verify no panic on very long control word
        let dir = tempdir().unwrap();
        let input = dir.path().join("long_control.rtf");

        // Create RTF with a very long control word
        let long_control = "a".repeat(10000);
        let rtf = format!("{{\\rtf1\\ansi \\{} text}}", long_control);
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should handle gracefully
        let result = cmd.assert();
        let code = result.get_output().status.code().unwrap();
        assert!(code == 0 || code == 2, "Should not panic");
    }

    #[test]
    fn no_panic_on_deeply_nested_table_controls() {
        // Verify no panic on deeply nested table controls
        let dir = tempdir().unwrap();
        let input = dir.path().join("nested_table.rtf");

        // Create RTF with many nested table controls
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..100 {
            rtf.push_str("\\trowd\\cellx1000\\intbl cell\\cell\\row ");
        }
        rtf.push('}');
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should succeed
        cmd.assert().success();
    }

    #[test]
    fn no_panic_on_mixed_valid_invalid_controls() {
        // Verify no panic on mix of valid and invalid controls
        let dir = tempdir().unwrap();
        let input = dir.path().join("mixed.rtf");

        let rtf = "{\\rtf1\\ansi \\b valid\\invalid \\i more\\bogus text}";
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);

        // Should succeed with warnings
        cmd.assert().success();
    }
}

// =============================================================================
// LIMIT BREACH BEHAVIOR TESTS
// =============================================================================
//
// Tests that verify all limit breaches map to parse errors (exit code 2)
// and that error messages are clear.

mod limit_breach_behavior {
    use super::*;

    #[test]
    fn all_limit_breaches_return_exit_code_2() {
        // Verify that all limit breaches return exit code 2 (parse error)
        // This is a meta-test that documents the expected behavior

        // Cells limit
        let dir = tempdir().unwrap();
        let input = dir.path().join("cells.rtf");
        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        cmd.assert().failure().code(2);

        // Depth limit
        let input2 = dir.path().join("depth.rtf");
        let mut rtf = String::from("{\\rtf1\\ansi ");
        for _ in 0..257 {
            rtf.push('{');
        }
        rtf.push('x');
        for _ in 0..257 {
            rtf.push('}');
        }
        rtf.push('}');
        fs::write(&input2, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input2.to_str().unwrap(), "--format", "json"]);
        cmd.assert().failure().code(2);
    }

    #[test]
    fn error_messages_indicate_limit_type() {
        // Verify that error messages indicate which limit was exceeded
        let dir = tempdir().unwrap();

        // Test cells limit error message
        let input = dir.path().join("cells.rtf");
        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        let output = cmd.assert().failure().get_output().stderr.clone();
        let stderr = String::from_utf8_lossy(&output);
        assert!(
            stderr.contains("cell") || stderr.contains("limit"),
            "Error message should indicate limit type: {}",
            stderr
        );
    }

    #[test]
    fn limit_failures_are_deterministic() {
        // Verify that limit failures are deterministic
        let dir = tempdir().unwrap();
        let input = dir.path().join("cells.rtf");

        let mut rtf = String::from("{\\rtf1\\ansi\n\\trowd");
        for i in 1..=1001 {
            rtf.push_str(&format!("\\cellx{}", i * 100));
        }
        rtf.push_str("\n\\intbl ");
        for i in 1..=1001 {
            rtf.push_str(&format!("C{}\\cell ", i));
        }
        rtf.push_str("\\row\n}");
        fs::write(&input, rtf).unwrap();

        // Run twice and verify same error
        let mut cmd1 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd1.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        let stderr1 = cmd1.assert().failure().get_output().stderr.clone();

        let mut cmd2 = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
        cmd2.args(["convert", input.to_str().unwrap(), "--format", "json"]);
        let stderr2 = cmd2.assert().failure().get_output().stderr.clone();

        assert_eq!(stderr1, stderr2, "Limit failures should be deterministic");
    }
}
