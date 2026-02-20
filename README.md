# rtfkit

RTF parsing toolkit with a CLI-first workflow.

Current status (Phase 1):
- Parses RTF into a deterministic intermediate representation (IR)
- Emits conversion reports (`text` or `json`)
- Supports `--emit-ir` for snapshot/debug workflows
- DOCX writing is planned; `-o/--output` is intentionally rejected for now

[![CI](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml/badge.svg)](https://github.com/TorstenCScholz/rtfkit/actions/workflows/ci.yml)

## Install

From source:

```sh
cargo install --path crates/rtfkit-cli
```

Or download a pre-built binary from [Releases](https://github.com/TorstenCScholz/rtfkit/releases).

## Usage

```sh
# Human-readable report
rtfkit convert fixtures/simple_paragraph.rtf

# JSON report
rtfkit convert fixtures/simple_paragraph.rtf --format json

# Emit IR JSON
rtfkit convert fixtures/simple_paragraph.rtf --emit-ir out.json

# Strict mode: fail when dropped content is reported
rtfkit convert fixtures/complex.rtf --strict --format json
```

### Reserved flags

`-o/--output` and `--to docx` are part of the long-term converter interface.
In v0.1, `--output` returns an explicit error because the DOCX writer is not implemented yet.

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

## Limitations (v0.1)

- No DOCX output writer yet
- Partial RTF coverage only (focused on common text/style cases)
- No tables/lists/images as first-class IR blocks yet

## License

Licensed under either [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT), at your option.
