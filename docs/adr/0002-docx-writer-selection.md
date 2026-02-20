# ADR-0002: DOCX Writer Library Selection

## Status

Accepted

## Context

Phase 2 of `rtfkit` requires implementing a DOCX writer to convert the Intermediate Representation (IR) to valid OOXML DOCX files. The writer must support:

- **Basic document structure**: Paragraphs (`w:p`), runs (`w:r`), text (`w:t`)
- **Paragraph alignment**: left, center, right, justify (`w:jc`)
- **Inline formatting**: bold (`w:b`), italic (`w:i`), underline (`w:u`)
- **Whitespace preservation**: Using `xml:space="preserve"` for runs with leading/trailing spaces
- **Valid OOXML output**: Files that open correctly in Microsoft Word and LibreOffice

The architecture plan in [`docs/specs/PHASE2.md`](../specs/PHASE2.md) specifies a dedicated writer crate (`crates/rtfkit-docx`) that accepts the IR [`Document`](../../crates/rtfkit-core/src/lib.rs) as input.

### Current IR Structure

The IR types defined in [`rtfkit-core/src/lib.rs`](../../crates/rtfkit-core/src/lib.rs) include:

- [`Document`](../../crates/rtfkit-core/src/lib.rs:194) - Root structure containing blocks
- [`Block`](../../crates/rtfkit-core/src/lib.rs:173) - Enum for block-level elements (currently only `Paragraph`)
- [`Paragraph`](../../crates/rtfkit-core/src/lib.rs:144) - Contains alignment and runs
- [`Run`](../../crates/rtfkit-core/src/lib.rs:89) - Text with uniform formatting (bold, italic, underline, font_size, color)
- [`Alignment`](../../crates/rtfkit-core/src/lib.rs:58) - Left, Center, Right, Justify

### Evaluation Criteria

| Criterion | Weight | Description |
|-----------|--------|-------------|
| License compatibility | High | Must be MIT or Apache-2.0 compatible |
| OOXML validity | High | Must produce valid DOCX files openable in Word/LibreOffice |
| Formatting support | High | Support for bold, italic, underline, alignment |
| API stability | Medium | Stable API, not in heavy flux |
| Maintenance status | Medium | Active maintenance, recent commits |
| Dependencies | Medium | Minimal dependency footprint |
| Learning curve | Low | Easy to integrate and use |

## Decision

**Recommendation: Use `docx-rs` crate for DOCX generation.**

The `docx-rs` crate provides the best balance of features, maintenance, and API design for our needs. It handles the complexity of OOXML structure while providing a clean Rust API.

## Alternatives Considered

### Option 1: `docx-rs` crate

**Source**: <https://crates.io/crates/docx-rs>  
**Repository**: <https://github.com/dam4rus/docx-rs>

| Aspect | Evaluation |
|--------|------------|
| License | MIT ✓ |
| Last updated | Active (2024 commits) |
| Downloads | ~30K total, growing |
| OOXML validity | Produces valid DOCX files |
| Formatting | Full support for bold, italic, underline, alignment |
| API | Builder pattern, type-safe |
| Dependencies | Moderate (quick-xml, zip, thiserror) |

**Pros**:
- MIT licensed, fully compatible with our dual-license approach
- Clean, idiomatic Rust API with builder pattern
- Type-safe element construction reduces XML errors
- Active maintenance with responsive maintainer
- Good documentation and examples
- Supports all required formatting features
- Handles DOCX package structure (content types, relationships)

**Cons**:
- Relatively young project compared to alternatives in other languages
- May lack advanced features we don't currently need
- API still evolving (pre-1.0)

**Example usage**:
```rust
use docx_rs::*;

let doc = Document::new()
    .add_paragraph(
        Paragraph::new()
            .add_run(Run::new("Hello World").bold())
            .alignment(Alignment::Center)
    );

let file = std::fs::File::create("output.docx").unwrap();
doc.pack(file).unwrap();
```

### Option 2: `docx` crate

**Source**: <https://crates.io/crates/docx>  
**Repository**: <https://github.com/stoicdevio/docx-rs>

| Aspect | Evaluation |
|--------|------------|
| License | MIT ✓ |
| Last updated | Less active (older commits) |
| Downloads | Lower than docx-rs |
| OOXML validity | Basic support |
| Formatting | Limited formatting support |
| API | Simpler but less feature-complete |

**Pros**:
- MIT licensed
- Simpler API for basic documents

**Cons**:
- Less actively maintained
- More limited formatting support
- Smaller community
- Unclear OOXML validity guarantees

### Option 3: Custom Implementation with `zip` + `quick-xml`

**Source**: Build from scratch using `zip` and `quick-xml` crates

| Aspect | Evaluation |
|--------|------------|
| License | N/A (our code) |
| Maintenance | Fully our control |
| OOXML validity | Must implement correctly |
| Formatting | Must implement all mappings |
| Dependencies | Minimal (zip, quick-xml) |

**Pros**:
- Complete control over XML structure
- No dependency on DOCX library stability
- Minimal external dependencies
- Learning opportunity for OOXML format

**Cons**:
- Highest development effort
- Must implement DOCX package structure manually
- Risk of OOXML validity issues
- Must handle all edge cases ourselves
- More boilerplate code

**Implementation sketch**:
```rust
// Would need to implement:
// 1. DOCX package structure ([Content_Types].xml, _rels/.rels, etc.)
// 2. document.xml structure
// 3. Proper XML namespaces
// 4. Whitespace preservation logic
// 5. Style mappings

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText};
use zip::write::FileOptions;
use zip::ZipWriter;

fn write_document(doc: &Document, writer: &mut ZipWriter<File>) {
    // Must implement full OOXML structure...
}
```

### Option 4: `ooxml` crate

**Source**: <https://crates.io/crates/ooxml>

| Aspect | Evaluation |
|--------|------------|
| License | MIT OR Apache-2.0 ✓ |
| Last updated | Active development |
| Scope | Broader OOXML (Excel, Word, PowerPoint) |

**Pros**:
- Dual-licensed MIT/Apache-2.0
- Broader OOXML support if needed later

**Cons**:
- Heavier dependency for just DOCX
- More complex API due to broader scope
- Less focused on DOCX specifically

## Detailed Comparison

```
+------------------+------------+------------+------------------+------------+
| Criterion        | docx-rs    | docx       | custom (zip/xml) | ooxml      |
+------------------+------------+------------+------------------+------------+
| License          | ✓ MIT      | ✓ MIT      | ✓ N/A            | ✓ MIT/Apache|
| OOXML validity   | ✓ Good     | ~ Basic    | ○ DIY            | ✓ Good     |
| Formatting       | ✓ Full     | ~ Limited  | ○ DIY            | ✓ Full     |
| Maintenance      | ✓ Active   | ~ Slower   | ✓ Our control    | ✓ Active   |
| API stability    | ~ Pre-1.0  | ~ Stable   | ✓ N/A            | ~ Evolving |
| Dev effort       | ✓ Low      | ✓ Low      | ✗ High           | ✓ Low      |
| Dependencies     | ~ Moderate | ~ Light    | ✓ Minimal        | ~ Heavy    |
| Alignment support| ✓ Yes      | ~ Limited  | ○ DIY            | ✓ Yes      |
+------------------+------------+------------+------------------+------------+
```

## Rationale

The decision to use `docx-rs` is driven by several factors:

1. **Feature Coverage**: `docx-rs` supports all required formatting features (bold, italic, underline, alignment) out of the box, matching our IR mapping contract exactly.

2. **OOXML Validity**: The crate handles the complexity of DOCX package structure, including `[Content_Types].xml`, `_rels/.rels`, and proper XML namespaces. This reduces the risk of producing invalid files.

3. **API Design**: The builder pattern provides a clean, type-safe API that maps naturally from our IR types:
   - `Document.blocks` → `docx_rs::Document::add_paragraph()`
   - `Paragraph.alignment` → `Paragraph::alignment()`
   - `Run` properties → `Run::bold()`, `Run::italic()`, `Run::underline()`

4. **Maintenance**: Active development with a responsive maintainer reduces risk of encountering unmaintained dependency issues (unlike our experience with RTF parser options).

5. **License Compatibility**: MIT license is fully compatible with our dual MIT/Apache-2.0 licensing.

6. **Right-Sized Scope**: Unlike `ooxml`, this crate focuses specifically on DOCX, keeping our dependency footprint reasonable.

### Mapping Implementation

The IR-to-DOCX mapping will follow this pattern:

```rust
// In crates/rtfkit-docx/src/lib.rs

use rtfkit_core::{Document as IrDocument, Block, Paragraph, Run, Alignment};
use docx_rs::*;

pub fn convert(ir: IrDocument) -> Result<Docx, DocxError> {
    let mut doc = Document::new();
    
    for block in ir.blocks {
        match block {
            Block::Paragraph(para) => {
                doc = doc.add_paragraph(convert_paragraph(para));
            }
        }
    }
    
    Ok(doc)
}

fn convert_paragraph(para: Paragraph) -> docx_rs::Paragraph {
    let mut p = Paragraph::new();
    
    // Map alignment
    p = p.alignment(match para.alignment {
        Alignment::Left => docx_rs::Alignment::Left,
        Alignment::Center => docx_rs::Alignment::Center,
        Alignment::Right => docx_rs::Alignment::Right,
        Alignment::Justify => docx_rs::Alignment::Both,
    });
    
    // Map runs
    for run in para.runs {
        p = p.add_run(convert_run(run));
    }
    
    p
}

fn convert_run(run: Run) -> docx_rs::Run {
    let mut r = Run::new(&run.text);
    
    if run.bold { r = r.bold(); }
    if run.italic { r = r.italic(); }
    if run.underline { r = r.underline(); }
    
    r
}
```

## Consequences

### Positive

- Rapid development of DOCX writer functionality
- Valid OOXML output without deep format expertise
- Type-safe API reduces XML construction errors
- Active maintenance reduces long-term risk
- Clean separation between IR and output format

### Negative

- Dependency on pre-1.0 library (API may change)
- Less control over exact XML structure
- Potential feature gaps for future requirements
- Adds ~5 transitive dependencies

### Mitigations

- Pin to specific version in Cargo.toml
- Create thin abstraction layer in `rtfkit-docx` to isolate from API changes
- Document any workarounds for missing features
- Monitor crate updates and contribute upstream if needed
- Keep integration tests to validate OOXML output

## References

- [ECMA-376 Office Open XML](https://www.ecma-international.org/publications-and-standards/standards/ecma-376/) (Specification)
- [docx-rs crate](https://crates.io/crates/docx-rs) (crates.io)
- [docx-rs repository](https://github.com/dam4rus/docx-rs) (GitHub)
- [DOCX file format](https://en.wikipedia.org/wiki/Office_Open_XML) (Wikipedia)
- [`docs/specs/PHASE2.md`](../specs/PHASE2.md) - Phase 2 Architecture Plan
- [`docs/adr/0001-rtf-parser-selection.md`](0001-rtf-parser-selection.md) - Related ADR on parser selection
