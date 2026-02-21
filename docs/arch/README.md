# rtfkit Architecture

This document reflects the current implementation in `main` (v0.5, Phase 5).

## Overview

`rtfkit` provides a complete RTF-to-DOCX conversion pipeline with an intermediate representation (IR) and conversion reporting.

```mermaid
flowchart LR
    RTF[RTF Input] --> Tokenizer[Tokenizer]
    Tokenizer --> Events[RTF Events]
    Events --> Interpreter[Interpreter]
    Interpreter --> IR[IR Document]
    Interpreter --> Report[Report]
    IR --> DOCXWriter[DOCX Writer]
    DOCXWriter --> DOCXFile[.docx File]
    IR --> CLI[--emit-ir JSON]
    Report --> CLIOut[stdout text/json]
```

## Workspace

```text
rtfkit/
├── crates/
│   ├── rtfkit-core/   # Parser, interpreter, IR, reporting
│   ├── rtfkit-docx/   # DOCX writer implementation
│   └── rtfkit-cli/    # CLI entrypoint and IO/report rendering
├── fixtures/          # RTF inputs for tests
├── golden/            # Golden IR snapshots
└── docs/
    ├── adr/
    └── specs/
```

## `rtfkit-core`

Responsibilities:
- Tokenization and event conversion
- Stateful interpretation with group stack/style stack
- IR construction (`Document -> Block::Paragraph -> Run`)
- Warning/stats reporting
- Structural RTF validation (header + balanced groups)
- Parser limits enforcement (input size, depth, warnings)

Not in scope:
- File IO
- CLI argument handling
- DOCX writing

### IR model

- `Document { blocks: Vec<Block> }`
- `Block::Paragraph(Paragraph)`, `Block::ListBlock(ListBlock)`, or `Block::TableBlock(TableBlock)`
- `Paragraph { alignment, runs }`
- `Run { text, bold, italic, underline, font_size?, color? }`
- `ListBlock { list_id, kind, items }` — list container (Phase 3)
- `ListKind` — Bullet, OrderedDecimal, or Mixed
- `ListItem { level, blocks }` — item with nesting level (0-8)
- `TableBlock { rows }` — table container (Phase 4)
- `TableRow { cells }` — table row
- `TableCell { blocks }` — table cell with nested blocks
- `CellMerge` — merge state for cells (Phase 5)
- `CellVerticalAlign` — vertical alignment in cells (Phase 5)
- `RowAlignment` — row-level alignment (Phase 5)
- `RowProps` — row-level formatting properties (Phase 5)

See [Phase 3 IR Design](phase3-ir-design.md) for list model details.
See [Phase 4 IR Design](phase4-ir-design.md) for table model details.
See [Phase 5 IR Design](phase5-ir-design.md) for merge semantics details.

### Parser/interpreter notes

- Control words handled for MVP: `\b`, `\i`, `\ul`, `\ulnone`, `\par`, `\line`, `\ql`, `\qc`, `\qr`, `\qj`, `\uN`, `\ucN`
- List control words (Phase 3): `\lsN`, `\ilvlN`
- Table control words (Phase 4): `\trowd`, `\cellxN`, `\intbl`, `\cell`, `\row`
- Merge control words (Phase 5): `\clmgf`, `\clmrg`, `\clvmgf`, `\clvmrg`
- Cell alignment (Phase 5): `\clvertalt`, `\clvertalc`, `\clvertalb`
- Row alignment (Phase 5): `\trql`, `\trqc`, `\trqr`, `\trleft`
- Legacy paragraph-numbering controls (`\pn...`, `\pnlvl*`, `\pntext`) are currently dropped with warnings
- Destination groups are skipped at group start (e.g. `fonttbl`, `colortbl`, unknown `\*` destinations)
- `\listtable` and `\listoverridetable` are parsed for list definitions (Phase 3)
- Escaped symbols (`\\`, `\{`, `\}`) are preserved as text
- Unsupported destination content emits `DroppedContent` warnings

### Parser limits

For safety and resource management:
- Maximum input size: 10 MB
- Maximum group depth: 256 levels
- Maximum warnings: 1000 (then truncated)
- Maximum rows per table: 10000 (Phase 5)
- Maximum cells per row: 1000 (Phase 5)
- Maximum merge span: 1000 (Phase 5)

## `rtfkit-docx`

Responsibilities:
- Convert IR `Document` to DOCX format
- Map IR styles to OpenXML elements
- Write valid `.docx` ZIP archives

### IR → DOCX mapping

| IR Element | DOCX Element |
|------------|--------------|
| `Document` | `<w:document>` |
| `Block::Paragraph` | `<w:p>` |
| `Block::ListBlock` | `<w:p>` with `<w:numPr>` |
| `Block::TableBlock` | `<w:tbl>` with `<w:tr>` and `<w:tc>` |
| `Run` | `<w:r>` |
| `Run.text` | `<w:t>` |
| `Run.bold = true` | `<w:b/>` in `<w:rPr>` |
| `Run.italic = true` | `<w:i/>` in `<w:rPr>` |
| `Run.underline = true` | `<w:u w:val="single"/>` |
| `Paragraph.alignment` | `<w:jc w:val="..."/>` |
| `ListBlock` | `numbering.xml` with `<w:abstractNum>` and `<w:num>` |
| `TableBlock` | `<w:tbl>` with grid columns |
| `CellMerge::HorizontalStart` | `<w:gridSpan w:val="N"/>` (Phase 5) |
| `CellMerge::VerticalStart` | `<w:vMerge w:val="restart"/>` (Phase 5) |
| `CellMerge::VerticalContinue` | `<w:vMerge w:val="continue"/>` (Phase 5) |
| `CellVerticalAlign` | `<w:vAlign w:val="..."/>` (Phase 5) |

## `rtfkit` CLI

Binary name: `rtfkit`

Command:

```bash
rtfkit convert [OPTIONS] <INPUT>
```

Options:
- `--format <text|json>`: report output format (default `text`)
- `--emit-ir <FILE>`: write IR as pretty JSON
- `--strict`: exit non-zero if `DroppedContent` warnings exist
- `-o, --output <FILE>`: write DOCX output to file
- `--force`: overwrite existing output file
- `--verbose`: debug logging

Exit codes:
- `0`: success
- `2`: parse/validation error (invalid RTF)
- `3`: writer/IO failure (cannot write output file)
- `4`: strict-mode violation

## Reporting

Warnings:
- `UnsupportedControlWord`
- `UnknownDestination`
- `DroppedContent`
- `UnsupportedListControl` (Phase 3)
- `UnresolvedListOverride` (Phase 3)
- `UnsupportedNestingLevel` (Phase 3)
- `UnsupportedTableControl` (Phase 4)
- `MalformedTableStructure` (Phase 4)
- `UnclosedTableCell` (Phase 4)
- `UnclosedTableRow` (Phase 4)
- `MergeConflict` (Phase 5)
- `TableGeometryConflict` (Phase 5)

Stats:
- `paragraph_count`
- `run_count`
- `bytes_processed`
- `duration_ms`

Strict mode checks `DroppedContent` warnings.

## Testing

Test layers:
- Core unit tests for tokenizer/interpreter/report behavior
- DOCX writer unit tests
- Golden IR snapshot tests over all fixtures
- CLI contract tests for exit codes/strict mode/invalid input
- DOCX integration tests for end-to-end conversion

Golden update command:

```bash
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests
```

## Known gaps

- Limited RTF feature coverage (no images as IR blocks)
- DOCX output supports basic text formatting, lists, and tables
- Table support includes cell merging and vertical alignment (Phase 5)
- Row alignment and indent not supported by docx-rs (cosmetic loss only)
- No full RTF spec compliance target

## References

- [ADR-0001: RTF Parser Selection](../adr/0001-rtf-parser-selection.md)
- [ADR-0002: DOCX Writer Selection](../adr/0002-docx-writer-selection.md)
- [Phase 1 Specification](../specs/PHASE1.md)
- [Phase 2 Specification](../specs/PHASE2.md)
- [Phase 3 Specification](../specs/PHASE3.md)
- [Phase 3 IR Design](phase3-ir-design.md)
- [Phase 4 Specification](../specs/PHASE4.md)
- [Phase 4 IR Design](phase4-ir-design.md)
- [Phase 5 Specification](../specs/PHASE5.md)
- [Phase 5 IR Design](phase5-ir-design.md)
- [Initial Description](../specs/INITIAL_DESCRIPTION.md)
