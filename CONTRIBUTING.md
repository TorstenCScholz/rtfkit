# Contributing

Thanks for your interest in contributing to rtfkit!

## Table of Contents

- [Workflow](#workflow)
- [Fixture-First Development](#fixture-first-development)
- [Adding a Fixture](#adding-a-fixture)
- [Testing Guidelines](#testing-guidelines)
- [Code Style](#code-style)
- [Documentation Updates](#documentation-updates)
- [Safety-Related Changes](#safety-related-changes)

## Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b my-feature`)
3. Make your changes
4. Run checks locally:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```
5. Commit and push
6. Open a Pull Request

## Fixture-First Development

rtfkit follows a **fixture-first development** workflow. This means:

1. **Add a fixture** - Create an RTF file that demonstrates the behavior
2. **Add a failing test** - Write a test that captures the expected behavior
3. **Implement the fix** - Make the test pass
4. **Update docs/changelog** - Document the change

This approach ensures:
- All features have test coverage
- Behavior is documented through tests
- Regressions are caught early
- Golden snapshots stay up-to-date

## Adding a Fixture

### Step 1: Create the RTF File

Place your `.rtf` file in the `fixtures/` directory with a descriptive name:

| Prefix | Purpose |
|--------|---------|
| `text_*` | Text and formatting tests |
| `list_*` | List structure tests |
| `table_*` | Table structure tests |
| `mixed_*` | Combined content tests |
| `malformed_*` | Error recovery tests |
| `limits_*` | Limit boundary tests |

Example: `table_with_merged_cells.rtf`

### Step 2: Generate the Golden File

Run the golden test with the update flag:

```sh
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests
```

This generates `golden/<fixture_name>.json` with the IR snapshot.

### Step 3: Review the Golden File

Open the generated JSON file and verify:

1. The IR structure is correct
2. Text content is preserved
3. Formatting is captured
4. No unexpected warnings

### Step 4: Commit Both Files

```sh
git add fixtures/<name>.rtf golden/<name>.json
git commit -m "Add fixture for <feature>"
```

### Step 5: Add Tests

Add tests in the appropriate test file:

- `crates/rtfkit-cli/tests/cli_contract_tests.rs` - Exit codes, strict mode
- `crates/rtfkit-cli/tests/determinism_tests.rs` - Output stability
- `crates/rtfkit-cli/tests/limits_tests.rs` - Limit behavior
- `crates/rtfkit-cli/tests/docx_integration_tests.rs` - DOCX output

## Testing Guidelines

### Test Categories

| Category | File | Purpose |
|----------|------|---------|
| Contract tests | `cli_contract_tests.rs` | Exit codes, strict mode, warnings |
| Determinism tests | `determinism_tests.rs` | Output stability |
| Limits tests | `limits_tests.rs` | Safety and resource protection |
| Golden tests | `golden_tests.rs` | IR snapshot validation |
| DOCX integration | `docx_integration_tests.rs` | End-to-end conversion |

### Test Naming

Use descriptive names that indicate what's being tested:

```rust
#[test]
fn strict_mode_fails_on_dropped_content() { ... }

#[test]
fn deterministic_output_for_nested_lists() { ... }

#[test]
fn table_row_limit_violation_returns_parse_exit_code() { ... }
```

### Test Structure

Follow the Arrange-Act-Assert pattern:

```rust
#[test]
fn test_name() {
    // Arrange - set up the test
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().parent().unwrap();
    let fixture = project_root.join("fixtures/example.rtf");

    // Act - run the code under test
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(["convert", fixture.to_str().unwrap(), "--format", "json"]);

    // Assert - verify the result
    cmd.assert().success().code(0);
}
```

### Running Tests

```sh
# Run all tests
cargo test --all

# Run specific test file
cargo test --test cli_contract_tests

# Run specific test
cargo test test_name

# Update golden snapshots
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests
```

## Code Style

- Run `cargo fmt` before committing
- All clippy warnings must be resolved (`-D warnings`)
- Keep dependencies minimal
- Document public APIs with doc comments

### Documentation Comments

```rust
/// Creates a new paragraph from a vector of runs.
///
/// # Example
/// ```
/// use rtfkit_core::{Paragraph, Run};
/// let para = Paragraph::from_runs(vec![Run::new("Hello")]);
/// ```
pub fn from_runs(runs: Vec<Run>) -> Self {
    Self {
        alignment: Alignment::default(),
        runs,
    }
}
```

## Documentation Updates

When adding or changing features, update the relevant documentation:

| Change | Documentation to Update |
|--------|------------------------|
| New feature | README.md, Feature Support Matrix |
| New warning type | Warning Reference, Architecture |
| New limit | Limits Policy, Architecture |
| Bug fix | CHANGELOG.md |
| Breaking change | CHANGELOG.md with migration notes |

### Documentation Files

- `README.md` - Project overview and usage
- `CHANGELOG.md` - Version history
- `docs/arch/README.md` - Architecture overview
- `docs/feature-support.md` - Feature support matrix
- `docs/warning-reference.md` - Warning documentation
- `docs/limits-policy.md` - Parser limits policy

## Safety-Related Changes

Changes that affect safety or limits require extra attention:

### Before Making Changes

1. Review [Limits Policy](docs/limits-policy.md)
2. Understand the fail-closed philosophy
3. Consider impact on existing limits

### When Adding New Limits

1. Define the limit in `crates/rtfkit-core/src/limits.rs`
2. Add enforcement in the interpreter
3. Add tests for:
   - Near-limit success
   - Over-limit failure
   - No partial output after failure
4. Update documentation

### When Modifying Limits

1. Consider backward compatibility
2. Ensure fail-closed behavior is preserved
3. Update tests and documentation
4. Add migration notes if breaking

## Pull Request Checklist

Before submitting a PR:

- [ ] Code is formatted (`cargo fmt --all`)
- [ ] No clippy warnings (`cargo clippy --all-targets -- -D warnings`)
- [ ] All tests pass (`cargo test --all`)
- [ ] New features have fixtures
- [ ] Documentation is updated
- [ ] CHANGELOG.md has entry (if user-facing)

## Questions?

Open an issue for discussion before starting significant work.
