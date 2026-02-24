# Refactor Plan: Replace Monolithic Interpreter with a Cohesive RTF Parsing Stack

## Summary

`interpreter.rs` is currently very large and multiplexes too many concerns in one file: tokenizer, event conversion, state machine, destination handling, lists, tables, fields, style/color/font resolution, and unit tests.

Current facts:

- File size: 4,865 LOC in `crates/rtfkit-core/src/interpreter.rs`
- Tests inside same file: 2,037 LOC (starts at line 2829)
- Public API currently re-exported from `crates/rtfkit-core/src/lib.rs`

Chosen direction:

- API freedom: allow redesign
- Depth: full redesign
- Migration: immediate break (no deprecation window)

## Public API Changes (Intentional Breaking Changes)

Replace the current `interpreter`-centric public surface with a smaller `rtf` parsing API.

1. Remove from public API:

- `Interpreter`
- `Token`
- `RtfEvent`
- `StyleState`
- `ParsedListDefinition`
- `ParsedListLevel`
- `ParsedListOverride`
- `ParagraphListRef`
- `tokenize(...)` public export

1. Add new public API:

- `rtfkit_core::rtf::parse(input: &str) -> Result<(Document, Report), ConversionError>`
- `rtfkit_core::rtf::parse_with_limits(input: &str, limits: ParserLimits) -> Result<(Document, Report), ConversionError>`
- `rtfkit_core::rtf::RtfParser` (stateful parser object with configurable limits)

1. Update all in-repo callers immediately:

- `crates/rtfkit-cli/src/main.rs`
- `crates/rtfkit-cli/tests/golden_tests.rs`
- README/docs snippets that currently use `Interpreter::parse`

## Target Internal Architecture

Create a new module tree under:

- `crates/rtfkit-core/src/rtf/mod.rs`

Planned files:

- `crates/rtfkit-core/src/rtf/api.rs`: public entrypoints and `RtfParser`
- `crates/rtfkit-core/src/rtf/pipeline.rs`: parse flow (`tokenize -> validate -> interpret`)
- `crates/rtfkit-core/src/rtf/tokenizer.rs`: token model and nom tokenizer
- `crates/rtfkit-core/src/rtf/events.rs`: event model and token-to-event conversion
- `crates/rtfkit-core/src/rtf/state.rs`: top-level runtime state
- `crates/rtfkit-core/src/rtf/state_style.rs`: style/run state
- `crates/rtfkit-core/src/rtf/state_destinations.rs`: destination skip state
- `crates/rtfkit-core/src/rtf/state_lists.rs`: list parse/resolve state
- `crates/rtfkit-core/src/rtf/state_tables.rs`: table/merge/shading state
- `crates/rtfkit-core/src/rtf/state_fields.rs`: field/hyperlink state
- `crates/rtfkit-core/src/rtf/handlers_destinations.rs`
- `crates/rtfkit-core/src/rtf/handlers_control_words.rs`
- `crates/rtfkit-core/src/rtf/handlers_text.rs`
- `crates/rtfkit-core/src/rtf/handlers_lists.rs`
- `crates/rtfkit-core/src/rtf/handlers_tables.rs`
- `crates/rtfkit-core/src/rtf/handlers_fields.rs`
- `crates/rtfkit-core/src/rtf/finalize.rs`: paragraph/cell/row/table finalization logic

Design rule: no traits/framework abstractions; plain structs + free functions + `impl` blocks only.

## Implementation Sequence (Decision-Complete)

1. Introduce new `rtf` module scaffold and wire it in `crates/rtfkit-core/src/lib.rs`.
2. Move tokenizer/event/validation code first (pure parsing layer, minimal coupling).
3. Introduce `RuntimeState` with nested sub-states (`style`, `destinations`, `lists`, `tables`, `fields`, `resources`).
4. Move destination logic (`maybe_start_destination`, skip-depth flow) into destination handler module.
5. Move control-word dispatch into domain-specific handler functions:

- character/paragraph controls
- list controls
- table controls
- field controls
- font/color resource controls

6. Move text/run resolution and style comparison into text handler module.
7. Move paragraph/list/table/field finalization functions into finalize + domain handler files.
8. Remove old `interpreter.rs` and replace with new module-based API.
9. Apply immediate API break in `rtfkit-core` exports; update all in-repo consumers and docs in same change.
10. Split tests into module-scoped test files under:

- `crates/rtfkit-core/src/rtf/tests/mod.rs`
- tokenizer, limits, destinations, lists, tables, fields, font_color, shading, regression suites

## Test Cases and Scenarios

Preserve all existing behavior with focused suites:

1. Parse/tokenization:

- balanced/unbalanced groups
- missing `\\rtf` header
- unicode + `\\ucN` fallback skip
- escaped symbols and spacing behavior

1. Destination behavior:

- metadata skip (no warning)
- unknown destination warning + dropped content
- list/font/color destination parsing

1. List semantics:

- `\\listtable`/`\\listoverridetable` resolution
- `\\ls`/`\\ilvl` mapping and clamping
- unresolved override warnings + strict-mode dropped reasons

1. Table semantics:

- row/cell lifecycle (`\\trowd`, `\\cell`, `\\row`)
- width from `\\cellx` deltas
- merge normalization and orphan continuation degradation
- hard limits (`max_cells_per_row`, `max_rows_per_table`, `max_merge_span`)

1. Field/hyperlink semantics:

- supported URL schemes
- malformed/non-hyperlink field degradation
- nested field behavior preservation

1. Font/color/shading:

- `\\fonttbl`, `\\deff`, `\\f`, `\\fs`
- `\\colortbl`, `\\cf`, `\\highlight`, `\\cb`
- paragraph/cell shading controls and reset semantics (`\\plain` vs `\\pard`)

1. Workspace-level confidence:

- `cargo test -p rtfkit-core`
- `cargo test -p rtfkit-cli --test golden_tests`
- `cargo test --workspace`

## Acceptance Criteria

1. No single file in new `rtf` module exceeds 700 LOC; target average <350 LOC.
2. Public API is reduced to `rtf` parse entrypoints and parser object.
3. All existing interpreter behaviors remain unchanged (including warning reason strings and strict-mode triggers).
4. All existing tests pass after relocation and API updates.
5. CLI and golden tests compile and pass with new API.
6. README/docs examples no longer reference old `Interpreter::parse`.

## Assumptions and Defaults

1. Immediate API break is intentional and acceptable in this refactor.
2. No functional feature additions are included; this is structural + API cleanup.
3. No new dependencies will be introduced.
4. Warning reason strings and failure semantics are treated as stable contracts and must not change.
