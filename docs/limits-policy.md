# Parser Limits Policy

This document describes the resource protection limits implemented in rtfkit, the fail-closed philosophy behind them, and guidance for when to adjust them.

## Overview

rtfkit implements hard limits to protect against pathological inputs that could cause resource exhaustion or denial of service. These limits are enforced during parsing and conversion, and when exceeded, the operation fails immediately with an explicit error.

## Fail-Closed Philosophy

rtfkit follows a **fail-closed** design philosophy:

1. **Explicit failure over silent truncation**: When a limit is exceeded, the operation fails with a clear error message rather than silently truncating or degrading output.

2. **No partial output**: After a fatal limit failure, no partial output file is created. This prevents downstream systems from processing incomplete or corrupted data.

3. **Deterministic behavior**: Limit failures are deterministic—the same input will always produce the same error.

4. **Clear error messages**: Error messages indicate which limit was exceeded and provide the actual and limit values.

## Limits Reference

### `max_input_bytes`

| Property | Value |
|----------|-------|
| Default | 10 MB (10,485,760 bytes) |
| Type | `usize` |
| Error | `ParseError::InputTooLarge` |

**Purpose**: Prevents memory exhaustion from extremely large input files.

**When to adjust**:
- Increase for environments processing large legitimate documents (e.g., technical manuals, legal documents)
- Decrease for constrained environments (e.g., embedded systems, serverless functions)

**Example error**:
```
Parse error: Input too large: 20000000 bytes exceeds limit of 10485760 bytes
```

### `max_group_depth`

| Property | Value |
|----------|-------|
| Default | 256 levels |
| Type | `usize` |
| Error | `ParseError::GroupDepthExceeded` |

**Purpose**: Prevents stack overflow from deeply nested RTF groups.

**When to adjust**:
- Increase for documents with legitimate deep nesting (rare in practice)
- The RTF specification allows deep nesting, but most documents stay under 50 levels

**Example error**:
```
Parse error: Group depth exceeded: 300 levels exceeds limit of 256
```

### `max_warning_count`

| Property | Value |
|----------|-------|
| Default | 1000 warnings |
| Type | `usize` |
| Error | `ReportError::WarningCountExceeded` |

**Purpose**: Prevents unbounded memory growth from pathological documents that generate many warnings.

**Behavior**: When this limit is reached, parsing continues but no more warnings are collected. The output will indicate that warnings were capped.

**When to adjust**:
- Increase for environments where complete warning information is critical
- Decrease for performance-sensitive environments

**Note**: In strict mode, certain warning types (e.g., `DroppedContent`) will still cause failure regardless of the warning count.

### `max_rows_per_table`

| Property | Value |
|----------|-------|
| Default | 10,000 rows |
| Type | `usize` |
| Error | `ParseError` (table structure) |

**Purpose**: Prevents resource exhaustion from extremely large tables.

**When to adjust**:
- Increase for environments processing large data tables (e.g., spreadsheets exported to RTF)
- Decrease for environments with strict memory constraints

**Example error**:
```
Parse error: Table row count exceeds maximum: 15000 rows exceeds limit of 10000
```

### `max_cells_per_row`

| Property | Value |
|----------|-------|
| Default | 1,000 cells |
| Type | `usize` |
| Error | `ParseError` (table structure) |

**Purpose**: Prevents resource exhaustion from extremely wide tables.

**When to adjust**:
- Increase for environments processing wide data tables
- Decrease for environments with strict memory constraints

**Example error**:
```
Parse error: Table cell count exceeds maximum: 1500 cells exceeds limit of 1000
```

### `max_merge_span`

| Property | Value |
|----------|-------|
| Default | 1,000 cells |
| Type | `u16` |
| Error | `ParseError` (table structure) |

**Purpose**: Limits the number of cells that can be merged horizontally in a table.

**When to adjust**:
- Increase for documents with legitimate large merged regions
- Decrease for stricter validation

**Example error**:
```
Parse error: Merge span exceeds maximum: 1500 cells exceeds limit of 1000
```

### `max_image_bytes_total`

| Property | Value |
|----------|-------|
| Default | 50 MiB (52,428,800 bytes) |
| Type | `usize` |
| Error | `ParseError::ImageBytesExceeded` |

**Purpose**: Prevents memory exhaustion from documents with large or many embedded images.

**Behavior**: When the cumulative decoded image bytes exceed this limit, parsing fails immediately with a hard parse failure (exit code 2). No partial output is created.

**When to adjust**:
- Increase for environments processing documents with many high-resolution images
- Decrease for constrained environments

**Example error**:
```
Parse error: Image bytes exceeded: 60000000 bytes exceeds limit of 52428800
```

**To disable**: Use `ParserLimits::none()` (not recommended for untrusted input).

## Exit Code Behavior

When a limit is exceeded, rtfkit exits with a specific exit code:

| Exit Code | Meaning | Limit-Related Cause |
|-----------|---------|---------------------|
| 0 | Success | — |
| 2 | Parse/validation failure | `max_input_bytes`, `max_group_depth`, `max_rows_per_table`, `max_cells_per_row`, `max_merge_span`, `max_image_bytes_total` |
| 3 | Writer/IO failure | — |
| 4 | Strict-mode violation | — |

All hard limit violations result in **exit code 2** (parse/validation failure).

## Configuring Limits

### Programmatically

```rust
use rtfkit_core::limits::ParserLimits;

let limits = ParserLimits::new()
    .with_max_input_bytes(20 * 1024 * 1024)  // 20 MB
    .with_max_group_depth(512)
    .with_max_warning_count(2000)
    .with_max_rows_per_table(20000)
    .with_max_cells_per_row(2000)
    .with_max_merge_span(2000)
    .with_max_image_bytes_total(100 * 1024 * 1024);  // 100 MiB
```

### No Limits (Not Recommended)

For trusted inputs only:

```rust
let limits = ParserLimits::none();
```

**Warning**: Using `ParserLimits::none()` may expose the parser to denial-of-service attacks from pathological inputs.

## Testing

The limits test matrix is implemented in [`crates/rtfkit-cli/tests/limits_tests.rs`](../crates/rtfkit-cli/tests/limits_tests.rs) and covers:

1. **Near-limit success tests**: Verify that inputs just under limits succeed
2. **Over-limit failure tests**: Verify that inputs exceeding limits fail with exit code 2
3. **No-partial-output tests**: Verify that no output is emitted after fatal limit failure
4. **Safety tests**: Verify no panic/unbounded loops under malformed input

Run the tests:

```bash
cargo test --test limits_tests
```

## Security Considerations

1. **Input validation**: Limits are enforced before parsing begins (for size) or during parsing (for structural limits).

2. **Memory protection**: Limits prevent unbounded memory allocation from:
   - Large input files
   - Deeply nested structures
   - Large tables
   - Excessive warning collection

3. **No silent failures**: All limit breaches result in explicit errors, never silent truncation.

4. **Deterministic behavior**: The same input always produces the same error, making debugging predictable.

## Best Practices

1. **Use defaults for untrusted input**: The default limits are conservative and appropriate for processing untrusted RTF files.

2. **Increase limits only when necessary**: If you need to process larger files, increase only the specific limit needed.

3. **Monitor warning counts**: A high warning count may indicate malformed input or a document that doesn't conform to expectations.

4. **Handle exit code 2 gracefully**: When integrating rtfkit into larger systems, handle parse failures (exit code 2) appropriately.

5. **Test with representative documents**: Before deploying with custom limits, test with documents representative of your workload.

## Related Documentation

- [Architecture Overview](arch/README.md) - System design
- [Warning Reference](warning-reference.md) - Warning documentation
- [Feature Support Matrix](feature-support.md) - Supported features

## Changelog

| Version | Change |
|---------|--------|
| 0.11.0 | Added `max_image_bytes_total` limit for embedded images |
| 0.6.0 | Added limits test matrix, updated documentation |
| 0.5.0 | Added table-specific limits (rows, cells, merge span) |
| 0.2.0 | Added initial limits (input size, depth, warnings) |
| 0.1.0 | Initial limits implementation |
