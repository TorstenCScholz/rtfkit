# rtfkit Plan: CLI Test Suite Modularization

**Status: PLANNED**

## 1. Objective

Reduce maintenance cost and review friction in large CLI test files by splitting them into cohesive modules without changing behavior.

Primary targets:
- `crates/rtfkit-cli/tests/cli_contract_tests.rs` (~4.9k LOC)
- `crates/rtfkit-cli/tests/docx_integration_tests.rs` (~2.1k LOC)
- `crates/rtfkit-cli/tests/determinism_tests.rs` (~1.9k LOC)

## 2. Scope

### In scope

- Reorganize tests into topic-focused modules/files.
- Extract duplicated helper utilities into shared test support modules.
- Preserve test names, assertions, warning reason checks, and exit-code contracts.
- Keep fixture-first workflow unchanged.

### Out of scope

- New feature behavior.
- Loosening contract assertions.
- Renaming fixtures or changing golden expectations.

## 3. Proposed Structure

Use thin integration-test entrypoints plus internal modules:

- `crates/rtfkit-cli/tests/cli_contract_tests.rs` (entrypoint only)
- `crates/rtfkit-cli/tests/cli_contract/`
  - `mod.rs`
  - `exit_codes.rs`
  - `strict_mode.rs`
  - `warnings.rs`
  - `formats.rs` (docx/html/pdf flag contracts)
  - `io_and_overwrite.rs`
  - `fields_and_structure.rs`
  - `images_and_tables.rs`
- `crates/rtfkit-cli/tests/docx_integration_tests.rs` (entrypoint only)
- `crates/rtfkit-cli/tests/docx_integration/`
  - `mod.rs`
  - `paragraphs.rs`
  - `lists.rs`
  - `tables.rs`
  - `images.rs`
  - `structure.rs`
  - `style_profiles.rs`
- `crates/rtfkit-cli/tests/determinism_tests.rs` (entrypoint only)
- `crates/rtfkit-cli/tests/determinism/`
  - `mod.rs`
  - `ir_report.rs`
  - `docx_xml.rs`
  - `html.rs`
  - `pdf.rs`
  - `timestamps_and_metadata.rs`
- Shared helpers:
  - `crates/rtfkit-cli/tests/support/mod.rs`
  - `crates/rtfkit-cli/tests/support/cli.rs`
  - `crates/rtfkit-cli/tests/support/fs.rs`
  - `crates/rtfkit-cli/tests/support/docx_xml.rs`

## 4. Migration Strategy

1. Add support modules and move only helpers first (no test moves yet).
2. Split `cli_contract_tests.rs` by domain while keeping one green commit boundary.
3. Split `docx_integration_tests.rs` next.
4. Split `determinism_tests.rs` last (highest helper overlap).
5. Run full CLI test suite after each split step.

## 5. Risk Controls

- Keep all assertions byte-for-byte equivalent during moves.
- Do not collapse tests into parameterized loops when it would hide failure context.
- Preserve deterministic ordering where test output/fixtures depend on stable sort order.
- Keep strict-mode and warning reason string checks explicit.

## 6. Verification Plan

Required commands (from repo root):

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --test cli_contract_tests
cargo test --test docx_integration_tests
cargo test --test determinism_tests
cargo test --all
```

## 7. Acceptance Criteria

- No contract or output behavior changes.
- Existing CI jobs pass without relaxing checks.
- Each previously monolithic test file is reduced to a lightweight entrypoint + modules.
- New/updated tests can be added in domain files without touching unrelated areas.

## 8. Expected File Touches

- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/cli_contract_tests.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/docx_integration_tests.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/determinism_tests.rs`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/cli_contract/*`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/docx_integration/*`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/determinism/*`
- `/Users/torsten/Documents/Projects/rtfkit/crates/rtfkit-cli/tests/support/*`
