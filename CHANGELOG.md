# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
