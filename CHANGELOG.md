# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
