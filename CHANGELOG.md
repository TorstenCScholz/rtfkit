# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
