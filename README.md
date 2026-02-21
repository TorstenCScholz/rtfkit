# rtfkit

RTF parsing toolkit with a CLI-first workflow.

[![CI](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml)

## Current Status (Phase 6)

rtfkit provides a complete RTF-to-DOCX conversion pipeline with:

- **Text extraction** with formatting preservation (bold, italic, underline, alignment)
- **List support** - bullet and decimal lists with nested levels (up to 8)
- **Table support** - rows, cells, horizontal/vertical merges, cell alignment
- **Conversion reports** - JSON or text format with warnings and statistics
- **IR emission** - `--emit-ir` for snapshot/debug workflows
- **Parser limits** - safety limits for input size, depth, and warnings
- **Strict mode** - fail on dropped content for quality assurance

See [RTF Feature Overview](docs/rtf-feature-overview.md) for supported vs. not-yet-supported features.

## Install

From source:

```sh
cargo install --path crates/rtfkit-cli
```

Or download a pre-built binary from [Releases](https://github.com/TorstenCScholz/rtfkit/releases).

## Usage

```sh
# Convert RTF to DOCX
rtfkit convert input.rtf -o output.docx

# Convert and overwrite existing output file
rtfkit convert input.rtf -o output.docx --force

# Human-readable report (stdout)
rtfkit convert fixtures/text_simple_paragraph.rtf

# JSON report (stdout)
rtfkit convert fixtures/text_simple_paragraph.rtf --format json

# Emit IR JSON to file
rtfkit convert fixtures/text_simple_paragraph.rtf --emit-ir out.json

# Strict mode: fail when dropped content is reported
rtfkit convert fixtures/mixed_complex.rtf --strict --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Parse/validation error (invalid RTF or limit violation) |
| 3 | Writer/IO failure (e.g., cannot write output file) |
| 4 | Strict-mode violation (dropped content detected) |

### Parser Limits

For safety, the parser enforces these limits (see [Limits Policy](docs/limits-policy.md)):

| Limit | Default | Purpose |
|-------|---------|---------|
| Maximum input size | 10 MB | Prevents memory exhaustion |
| Maximum group depth | 256 levels | Prevents stack overflow |
| Maximum warnings | 1000 | Prevents unbounded memory growth |
| Maximum rows per table | 10,000 | Prevents resource exhaustion |
| Maximum cells per row | 1,000 | Prevents resource exhaustion |
| Maximum merge span | 1,000 cells | Limits merged regions |

## Output Contract

### Report JSON

```json
{
  "warnings": [],
  "stats": {
    "paragraph_count": 1,
    "run_count": 1,
    "bytes_processed": 29,
    "duration_ms": 0
  }
}
```

### Warning Types

| Type | Meaning | Strict Mode |
|------|---------|-------------|
| `unsupported_control_word` | RTF control not yet implemented | No failure |
| `unknown_destination` | RTF destination skipped | No failure |
| `dropped_content` | Content could not be represented | **Fails** |
| `unsupported_list_control` | List control not fully supported | No failure |
| `unresolved_list_override` | List reference not found | **Fails** |
| `unsupported_nesting_level` | List level > 8 clamped | No failure |
| `unsupported_table_control` | Table control not mapped | No failure |
| `malformed_table_structure` | Table structure issue | May fail |
| `unclosed_table_cell` | Missing `\cell` terminator | May fail |
| `unclosed_table_row` | Missing `\row` terminator | May fail |
| `merge_conflict` | Merge semantics conflict | **Fails** |
| `table_geometry_conflict` | Invalid table geometry | **Fails** |

See [Warning Reference](docs/warning-reference.md) for detailed documentation.

### IR JSON (`--emit-ir`)

Paragraph example:
```json
{
  "blocks": [
    {
      "type": "paragraph",
      "alignment": "left",
      "runs": [
        {
          "text": "Hello World",
          "bold": false,
          "italic": false,
          "underline": false
        }
      ]
    }
  ]
}
```

List example:
```json
{
  "blocks": [
    {
      "type": "listblock",
      "list_id": 1,
      "kind": "bullet",
      "items": [
        {
          "level": 0,
          "blocks": [
            {
              "type": "paragraph",
              "alignment": "left",
              "runs": [{"text": "First item", "bold": false, "italic": false, "underline": false}]
            }
          ]
        }
      ]
    }
  ]
}
```

Table example:
```json
{
  "blocks": [
    {
      "type": "tableblock",
      "rows": [
        {
          "cells": [
            {
              "blocks": [{"type": "paragraph", "alignment": "left", "runs": [{"text": "Cell 1"}]}],
              "width_twips": 1440
            }
          ]
        }
      ]
    }
  ]
}
```

## Development

```sh
# Test workspace
cargo test --all

# Update golden snapshots
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt --all
```

## Testing

rtfkit has comprehensive test coverage:

- **Contract tests** - Exit codes, strict mode, warning semantics
- **Determinism tests** - IR/report/DOCX stability verification
- **Limits tests** - Safety and resource protection
- **Golden tests** - IR snapshot validation
- **DOCX integration tests** - End-to-end conversion verification

See [CONTRIBUTING.md](CONTRIBUTING.md) for the fixture-first contribution workflow.

## Limitations (v0.6)

- Partial RTF coverage (focused on common text/style cases)
- No images as first-class IR blocks
- No hyperlinks/fields as first-class output
- DOCX output supports basic text formatting, lists, and tables
- List nesting limited to 8 levels (DOCX compatibility)
- Row alignment and indent not fully supported by docx-rs (cosmetic loss only)

For up-to-date support details, see [RTF Feature Overview](docs/rtf-feature-overview.md) and [Feature Support Matrix](docs/feature-support.md).

## Documentation

- [Architecture Overview](docs/arch/README.md)
- [RTF Feature Overview](docs/rtf-feature-overview.md)
- [Feature Support Matrix](docs/feature-support.md)
- [Warning Reference](docs/warning-reference.md)
- [Limits Policy](docs/limits-policy.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
