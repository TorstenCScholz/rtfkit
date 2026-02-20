# rtfkit

RTF parsing toolkit with a CLI-first workflow.

Current status (Phase 2):
- Parses RTF into a deterministic intermediate representation (IR)
- Converts RTF to DOCX via `-o/--output` flag
- Emits conversion reports (`text` or `json`)
- Supports `--emit-ir` for snapshot/debug workflows
- Parser limits for safety (input size, depth, warnings)

[![CI](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml)

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
rtfkit convert fixtures/simple_paragraph.rtf

# JSON report (stdout)
rtfkit convert fixtures/simple_paragraph.rtf --format json

# Emit IR JSON to file
rtfkit convert fixtures/simple_paragraph.rtf --emit-ir out.json

# Strict mode: fail when dropped content is reported
rtfkit convert fixtures/complex.rtf --strict --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Parse/validation error (invalid RTF) |
| 3 | Writer/IO failure (e.g., cannot write output file) |
| 4 | Strict-mode violation (dropped content detected) |

### Parser Limits

For safety, the parser enforces these limits:

- Maximum input size: 10 MB
- Maximum group depth: 256 levels
- Maximum warnings: 1000

## Output contract

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

`warnings` can contain:
- `unsupported_control_word`
- `unknown_destination`
- `dropped_content`

### IR JSON (`--emit-ir`)

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

## Limitations (v0.2)

- Partial RTF coverage only (focused on common text/style cases)
- No tables/lists/images as first-class IR blocks yet
- DOCX output supports basic text formatting (bold, italic, underline, alignment)

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
