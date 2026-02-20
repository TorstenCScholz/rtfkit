# ADR-0001: RTF Parser Selection

## Status

Accepted

## Context

The `rtfkit` project requires an RTF parser to convert RTF documents into an Intermediate Representation (IR) as part of Phase 1. The parser must support:

- **Basic RTF features**: Plain text, paragraphs (`\par`, `\line`), inline styles (bold, italic, underline)
- **Paragraph alignment**: left, center, right, justify
- **Unicode handling**: Proper decoding of RTF escape sequences and Unicode characters
- **Group/destination handling**: RTF's nested `{}` group structure

The project's goal is "90% real documents correctly, robustly, and reproducibly converted" - not full RTF spec coverage.

### Current Codebase State

The project currently has:
- A workspace structure with `rtfkit-core` and `rtfkit-cli` crates
- A custom `nom`-based tokenizer/interpreter in [`rtfkit-core/src/interpreter.rs`](../../crates/rtfkit-core/src/interpreter.rs)
- Report/warning model in [`rtfkit-core/src/report.rs`](../../crates/rtfkit-core/src/report.rs)
- CLI entrypoint in [`rtfkit-cli/src/main.rs`](../../crates/rtfkit-cli/src/main.rs)

### Evaluation Criteria

| Criterion | Weight | Description |
|-----------|--------|-------------|
| License compatibility | High | Must be MIT or Apache-2.0 compatible |
| Unicode handling | High | Must properly decode `\uN` escapes and special characters |
| Maintenance status | High | Active maintenance, recent commits, issue responsiveness |
| API design | Medium | Easy integration, clear semantics, event-based preferred |
| Performance | Medium | Reasonable parsing speed for typical documents |
| Feature coverage | Medium | Support for basic RTF features needed for v0.1 |

## Decision

**Recommendation: Build a custom parser using `nom` parser combinators.**

We will implement a custom RTF parser rather than using an existing crate. This decision is based on the evaluation below and aligns with the project's "stateful interpreter" architecture described in [`docs/specs/PHASE1.md`](../specs/PHASE1.md).

## Alternatives Considered

### Option 1: `rtf-parser` crate

**Source**: <https://crates.io/crates/rtf-parser>  
**Repository**: <https://github.com/davidhao3300/rtf-parser>

| Aspect | Evaluation |
|--------|------------|
| License | MIT ✓ |
| Last updated | ~2 years ago (concerning) |
| Downloads | Low download count |
| Unicode | Basic support, limited testing |
| API | Returns AST-like structure |
| Maintenance | Appears unmaintained |

**Pros**:
- MIT licensed
- Provides basic RTF tokenization
- Some Unicode support

**Cons**:
- **Unmaintained**: No commits for 2+ years
- Limited feature coverage
- API returns full AST, not events (less flexible for our interpreter pattern)
- Unknown production readiness
- No recent issue activity

### Option 2: `rtf-grimoire` crate

**Source**: <https://crates.io/crates/rtf-grimoire>  
**Repository**: <https://github.com/spebern/rtf-grimoire>

| Aspect | Evaluation |
|--------|------------|
| License | MIT OR Apache-2.0 ✓ |
| Last updated | Active (2024 commits) |
| Downloads | Moderate |
| Unicode | Good support |
| API | Token/Event based |
| Maintenance | Actively maintained |

**Pros**:
- Dual-licensed MIT/Apache-2.0
- Actively maintained with recent commits
- Better Unicode handling
- Event-based API suitable for interpreter pattern
- More complete RTF feature coverage

**Cons**:
- Heavier dependency with more features than needed
- API complexity for our simple use case
- May require adaptation for our specific IR model

### Option 3: Custom Parser with `nom`

**Source**: <https://crates.io/crates/nom>

| Aspect | Evaluation |
|--------|------------|
| License | MIT ✓ |
| Last updated | Actively maintained |
| Downloads | Extremely popular |
| Unicode | Handled by our implementation |
| API | Parser combinators |
| Maintenance | Very active |

**Pros**:
- Full control over parsing logic
- Perfect fit for "stateful interpreter" architecture
- No unnecessary features or dependencies
- Can optimize for our specific 90% use case
- Excellent error handling and reporting capabilities
- Well-documented and battle-tested library
- Learning opportunity for RTF format

**Cons**:
- More initial development effort
- Need to implement RTF knowledge from scratch
- Risk of missing edge cases initially

### Option 4: Hand-written Parser (No Dependencies)

| Aspect | Evaluation |
|--------|------------|
| License | N/A (our code) |
| Maintenance | Fully our control |
| Unicode | Must implement manually |
| API | Custom design |
| Performance | Potentially optimal |

**Pros**:
- Zero dependencies for parsing
- Complete control
- No license concerns

**Cons**:
- Highest development effort
- Must handle all edge cases manually
- More boilerplate code
- Harder to test parser combinators in isolation

## Detailed Comparison

```
+------------------+------------+--------------+-------+----------------+
| Criterion        | rtf-parser | rtf-grimoire | nom   | hand-written   |
+------------------+------------+--------------+-------+----------------+
| License          | ✓ MIT      | ✓ MIT/Apache | ✓ MIT | ✓ N/A          |
| Maintenance      | ✗ Poor     | ✓ Good       | ✓ Best| ✓ Our control  |
| Unicode          | ~ Basic    | ✓ Good       | ○ DIY | ○ DIY          |
| API fit          | ~ AST      | ✓ Events     | ✓ DIY | ✓ DIY          |
| Dev effort       | ✓ Low      | ✓ Low        | ~ Med | ✗ High         |
| Flexibility      | ✗ Low      | ~ Medium     | ✓ High| ✓ High         |
| Dependencies     | ✓ 1        | ~ Few        | ~ 1   | ✓ 0            |
+------------------+------------+--------------+-------+----------------+
```

## Rationale

The decision to build a custom parser using `nom` is driven by several factors:

1. **Architecture Alignment**: The PHASE1 specification calls for a "stateful interpreter" approach where the parser emits events and an interpreter builds the IR. A custom parser gives us full control over this event stream.

2. **Maintenance Risk**: `rtf-parser` appears unmaintained, creating risk for bug fixes and feature additions. While `rtf-grimoire` is actively maintained, it may not align perfectly with our needs.

3. **Scope Control**: Our goal is 90% coverage, not full spec compliance. A custom parser lets us focus on what matters and gracefully handle the rest with warnings.

4. **Learning & Control**: Understanding the RTF format deeply will help with debugging, edge cases, and future feature development.

5. **Reporting Integration**: We need detailed warnings and stats. A custom parser can emit these naturally during parsing.

### Implementation Strategy

If this recommendation is accepted, the implementation would follow this pattern:

```rust
// Tokenizer using nom
fn parse_control_word(input: &str) -> IResult<&str, Token> { ... }
fn parse_text(input: &str) -> IResult<&str, Token> { ... }
fn parse_group(input: &str) -> IResult<&str, Vec<Token>> { ... }

// Event emission for interpreter
enum RtfEvent {
    GroupStart,
    GroupEnd,
    ControlWord { word: &'str, parameter: Option<i32> },
    Text(&'str str),
    BinaryData(Vec<u8>),
}

// Interpreter builds IR
struct Interpreter {
    group_stack: Vec<StyleState>,
    current_style: StyleState,
    document: Document,
    warnings: Vec<Warning>,
}
```

## Consequences

### Positive

- Full control over parsing behavior and error handling
- Perfect alignment with interpreter architecture
- No dependency on potentially unmaintained crates
- Can optimize for our specific use cases
- Better integration with warning/reporting system

### Negative

- Initial development time for parser implementation
- Need to learn RTF specification details
- May miss edge cases that existing parsers handle
- Ongoing maintenance burden for parser code

### Mitigations

- Start with minimal feature set (v0.1 requirements)
- Add comprehensive test fixtures early
- Document RTF spec decisions in code comments
- Consider contributing to or forking existing crates if needs grow

## References

- [RTF Specification v1.9.1](https://www.microsoft.com/en-us/download/details.aspx?id=10725) (Microsoft)
- [RTF Format Overview](https://en.wikipedia.org/wiki/Rich_Text_Format) (Wikipedia)
- [`nom` parser combinators](https://github.com/rust-bakery/nom) (GitHub)
- [`rtf-grimoire`](https://github.com/spebern/rtf-grimoire) (GitHub)
- [`rtf-parser`](https://github.com/davidhao3300/rtf-parser) (GitHub)
