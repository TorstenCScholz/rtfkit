# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.6.0] - Unreleased

### Added

#### Fixture Taxonomy
- 44 fixtures organized by category:
  - `text_*` (9 fixtures) - Text and formatting tests
  - `list_*` (4 fixtures) - List structure tests
  - `table_*` (7 fixtures) - Table structure tests
  - `mixed_*` (3 fixtures) - Combined content tests
  - `malformed_*` (12 fixtures) - Error recovery tests
  - `limits_*` (9 fixtures) - Limit boundary tests
- Golden IR snapshots for all semantic fixtures

#### Contract Tests
- 83 CLI contract tests covering:
  - Exit code 0 (success) paths for all content types
  - Exit code 2 (parse/validation failure) for invalid RTF and limit violations
  - Exit code 3 (writer/IO failure) for output file issues
  - Exit code 4 (strict-mode violation) for dropped content detection
- Warning semantics tests for stable warning types and reason strings
- Recovery behavior tests for malformed inputs

#### Determinism Tests
- 35 determinism tests covering:
  - Report JSON ordering and value stability
  - IR JSON byte stability for same input
  - `word/document.xml` stability (excluding ZIP metadata)
- Tests for representative fixtures: simple text, nested lists, merge-heavy tables, degraded input

#### Limits Tests
- 34 limits tests covering:
  - `max_input_bytes` (10 MB default)
  - `max_group_depth` (256 levels)
  - `max_warning_count` (1000 warnings)
  - `max_rows_per_table` (10,000 rows)
  - `max_cells_per_row` (1,000 cells)
  - `max_merge_span` (1,000 cells)
- Near-limit success tests and over-limit failure tests
- No-partial-output verification after fatal limit failure

#### CI/CD Improvements
- Cross-platform CI matrix (Ubuntu, macOS, Windows)
- Release automation with artifact verification
- Smoke test scripts for release validation

#### Documentation
- Feature support matrix (`docs/feature-support.md`)
- Warning reference documentation (`docs/warning-reference.md`)
- Updated limits policy with table-specific limits
- Updated architecture documentation for Phase 6
- Fixture-first contribution workflow in CONTRIBUTING.md

### Changed

- Warning cap now preserves `DroppedContent` signal for strict mode
- All limit violations now map consistently to exit code 2
- Documentation synchronized with implemented behavior

### Fixed

- Deterministic output for all supported content types
- Consistent exit code behavior across platforms
- Warning reason string stability for key warning types

### Test Summary

| Category | Count |
|----------|-------|
| Contract tests | 83 |
| Determinism tests | 35 |
| Limits tests | 34 |
| Golden fixtures | 44 |
| DOCX integration tests | 30+ |
| Core unit tests | 100+ |
| **Total** | **300+** |

## [0.5.0] - Unreleased

### Added

#### IR Extensions
- `CellMerge` enum for horizontal and vertical merge semantics
- `CellVerticalAlign` enum for cell content alignment (top/center/bottom)
- `RowAlignment` enum for row-level alignment (left/center/right)
- `RowProps` struct for row-level formatting properties
- `TableProps` struct for table-level properties (placeholder)
- Extended `TableCell` with `merge` and `v_align` fields
- Extended `TableRow` with `row_props` field
- Extended `TableBlock` with `table_props` field

#### Parser Support
- Horizontal merge controls: `\clmgf` (merge start), `\clmrg` (merge continuation)
- Vertical merge controls: `\clvmgf` (vertical start), `\clvmrg` (vertical continuation)
- Row alignment controls: `\trql`, `\trqc`, `\trqr`
- Row indent control: `\trleft`
- Cell vertical alignment: `\clvertalt`, `\clvertalc`, `\clvertalb`

#### DOCX Writer
- Horizontal merge output as `w:gridSpan`
- Vertical merge output as `w:vMerge` (restart/continue)
- Cell vertical alignment output as `w:vAlign`

#### Warning Categories
- `MergeConflict` warning for merge semantics conflicts
- `TableGeometryConflict` warning for geometry issues (span exceeding bounds)

#### Limits
- `max_rows_per_table` limit (default: 10000)
- `max_cells_per_row` limit (default: 1000)
- `max_merge_span` limit (default: 1000)

#### Test Coverage
- 8 new RTF fixtures for merge scenarios
- 10 new DOCX integration tests
- 11 new contract tests
- Total: 236 tests passing

### Changed

- Merge controls are now properly handled instead of degraded
- Strict mode fails on merge semantic loss (exit code 4)
- Warning cap preserves `DroppedContent` signal for strict mode
- Table defensive-limit violations (`max_rows_per_table`, `max_cells_per_row`, `max_merge_span`) now fail as parse errors (exit code 2) instead of warning-only degradation

### Fixed

- Per-cell merge state tracking for correct multi-cell merge handling

## [0.4.0] - Unreleased

### Added

- Table support (Phase 4):
  - RTF tables are now converted to DOCX tables
  - New IR types: `TableBlock`, `TableRow`, `TableCell`
  - New warning variants: `UnsupportedTableControl`, `MalformedTableStructure`, `UnclosedTableCell`, `UnclosedTableRow`
  - Table control word support: `\trowd`, `\cellxN`, `\intbl`, `\cell`, `\row`
  - Graceful degradation for malformed table structures
  - 7 new table test fixtures

### Changed

- `Block` enum now includes `TableBlock` variant (breaking change to IR JSON format)
- Paragraph finalization now routes content to table cells when `\intbl` is active
- IR schema extended with table-related types

### Fixed

- Deterministic handling of malformed table input

### Migration Notes

- IR JSON format has changed: `blocks` array can now contain `{"type": "tableblock", ...}` objects
- External consumers of IR JSON should handle the new `tableblock` type
- Golden IR snapshots need regeneration: `UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests`

## [0.3.0] - Unreleased

### Added

- List support (Phase 3):
  - Bullet and decimal list support with `ListBlock`, `ListKind`, `ListItem` IR types
  - Nested lists up to 8 levels (DOCX compatibility limit)
  - Legacy `\pn...` paragraph numbering support for older RTF files
  - `\listtable` and `\listoverridetable` destination parsing
  - Deterministic numbering ID allocation in DOCX output
  - New warning types: `UnsupportedListControl`, `UnresolvedListOverride`, `UnsupportedNestingLevel`
- New IR types: `ListBlock`, `ListKind` (Bullet/OrderedDecimal/Mixed), `ListItem`, `ListId`
- List-related fixtures for testing: bullet, decimal, nested, mixed, and malformed lists

### Changed

- IR schema extended with `Block::ListBlock` variant (breaking change to IR JSON format)
- DOCX writer now generates `numbering.xml` for list output
- Golden IR snapshots regenerated to include list fixtures

### Migration Notes

- IR JSON format has changed: `blocks` array can now contain `{"type": "listblock", ...}` objects
- External consumers of IR JSON should handle the new `listblock` type
- Golden IR snapshots need regeneration: `UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests`

## [0.2.0] - Unreleased

### Added

- DOCX output support via `-o/--output <FILE>` flag
- `--force` flag to overwrite existing output files
- `rtfkit-docx` crate with DOCX writer implementation
- Parser limits for safety:
  - Maximum input size: 10 MB
  - Maximum group depth: 256 levels
  - Maximum warnings: 1000 (then truncated)
- DOCX integration tests for end-to-end conversion validation
- IR → DOCX mapping for basic text formatting (bold, italic, underline, alignment)

### Changed

- Exit code 3 now indicates writer/IO failures (previously "conversion/IO contract error")
- `-o/--output` now produces DOCX output instead of being rejected
- Architecture documentation updated to include DOCX writer stage in pipeline
- README updated with DOCX conversion examples and exit code documentation

### Fixed

- Output file handling now properly checks for existing files unless `--force` is specified

## [0.1.0] - Unreleased

### Added

- CLI contract tests for parse errors, strict mode, and unsupported `--output`
- Additional interpreter tests for spacing, escaped symbols, destination skipping, and structural validation

### Changed

- Renamed CLI package/binary to `rtfkit` (from `rtfkit-cli` binary name)
- Improved tokenizer behavior:
  - preserves meaningful spaces
  - consumes only ignorable source formatting whitespace (`\n`, `\r`, `\t`)
  - parses `\'hh` escapes as exactly two hex digits
  - parses control symbols as single characters
- Added RTF structural validation (`{\rtf...}` header and balanced groups)
- Added destination-group skipping with report signals (`UnknownDestination`, `DroppedContent`)
- Fixed paragraph finalization to preserve run style at paragraph boundaries
- Updated golden IR snapshots to reflect corrected parsing behavior
- Replaced template README with project-specific documentation
- Rewrote architecture documentation in `docs/arch/README.md` to match implementation

### Fixed

- Release build target/package mismatch (`cargo build --release -p rtfkit` now resolves)
- `-o/--output` no longer silently ignored; now fails explicitly until DOCX writing exists
- Strict mode now has meaningful failure paths via emitted `DroppedContent` warnings
