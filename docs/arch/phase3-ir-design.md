# Phase 3 IR Model Extension Design

## Overview

This document describes the IR model extension for Phase 3 list support, following Path A from the specification (adding an explicit `ListBlock` to the IR).

## Current State Analysis

### Existing Block Enum

The current [`Block`](../crates/rtfkit-core/src/lib.rs:190) enum only supports paragraphs:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    Paragraph(Paragraph),
}
```

### Existing Paragraph Finalization

The [`finalize_paragraph()`](../crates/rtfkit-core/src/interpreter.rs:601) method in the interpreter:
1. Flushes remaining text as a run
2. Adds paragraph to document if it has content
3. Resets current paragraph state

### Existing DOCX Writer

The [`convert_document()`](../crates/rtfkit-docx/src/writer.rs:82) function iterates over blocks and only handles `Block::Paragraph`.

---

## IR Model Extension

### New Types

Add the following types to [`crates/rtfkit-core/src/lib.rs`](../crates/rtfkit-core/src/lib.rs):

#### ListId Type

```rust
/// Unique identifier for a list within a document.
///
/// List IDs are assigned during interpretation and used to group
/// list items that belong to the same logical list.
pub type ListId = u32;
```

#### ListKind Enum

```rust
/// The kind of list numbering.
///
/// Represents the numbering style for a list or list level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    /// Bullet list with glyph markers
    #[default]
    Bullet,
    /// Decimal numbered list (1, 2, 3, ...)
    OrderedDecimal,
    /// Mixed or inconsistent list kind (fallback for ambiguous sources)
    Mixed,
}
```

#### ListItem Struct

```rust
/// A single item within a list.
///
/// Each item has a nesting level and contains blocks (typically paragraphs).
/// The level is 0-indexed, where 0 is the top level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// Nesting level (0-8, clamped for DOCX compatibility)
    pub level: u8,
    /// Content blocks for this item (typically one Paragraph in Phase 3)
    pub blocks: Vec<Block>,
}

impl ListItem {
    /// Creates a new list item at the given level.
    pub fn new(level: u8) -> Self {
        Self {
            level: level.min(8), // Clamp to DOCX max
            blocks: Vec::new(),
        }
    }

    /// Creates a list item from a paragraph at the given level.
    pub fn from_paragraph(level: u8, paragraph: Paragraph) -> Self {
        Self {
            level: level.min(8),
            blocks: vec![Block::Paragraph(paragraph)],
        }
    }
}
```

#### ListBlock Struct

```rust
/// A list containing one or more items.
///
/// Lists are block-level elements that contain numbered or bulleted items.
/// All items in a ListBlock share the same list_id and kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListBlock {
    /// Unique identifier for this list
    pub list_id: ListId,
    /// The kind of list (bullet, ordered, or mixed)
    pub kind: ListKind,
    /// The items in this list
    pub items: Vec<ListItem>,
}

impl ListBlock {
    /// Creates a new empty list with the given ID and kind.
    pub fn new(list_id: ListId, kind: ListKind) -> Self {
        Self {
            list_id,
            kind,
            items: Vec::new(),
        }
    }

    /// Adds an item to the list.
    pub fn add_item(&mut self, item: ListItem) {
        self.items.push(item);
    }

    /// Returns true if the list has no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
```

### Modified Block Enum

Update the `Block` enum to include `ListBlock`:

```rust
/// A block-level element in the document.
///
/// `Block` represents the top-level structural elements of a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    /// A paragraph block containing text
    Paragraph(Paragraph),
    /// A list block containing items
    ListBlock(ListBlock),
}
```

---

## Interpreter Changes

### New Interpreter State

Add the following fields to the [`Interpreter`](../crates/rtfkit-core/src/interpreter.rs:138) struct:

```rust
pub struct Interpreter {
    // ... existing fields ...
    
    /// Parsed list definitions from \listtable
    list_table: HashMap<i32, ParsedListDefinition>,
    
    /// Parsed list overrides from \listoverridetable
    list_overrides: HashMap<i32, ParsedListOverride>,
    
    /// Current paragraph's list reference (from \lsN and \ilvlN)
    current_list_ref: Option<ParagraphListRef>,
    
    /// Current open list blocks being built (stack for nesting)
    open_lists: Vec<ListBlock>,
}
```

### New Internal Types

```rust
/// Parsed list definition from \listtable destination.
#[derive(Debug, Clone)]
struct ParsedListDefinition {
    /// RTF list template ID
    template_id: i32,
    /// Kind per level (index = level)
    level_kinds: Vec<ListKind>,
}

/// Parsed list override from \listoverridetable.
#[derive(Debug, Clone)]
struct ParsedListOverride {
    /// Override index (referenced by \lsN)
    override_index: i32,
    /// Reference to the list template
    list_id: i32,
    /// Optional start number overrides per level
    start_overrides: HashMap<u8, i32>,
}

/// Resolved list reference for the current paragraph.
#[derive(Debug, Clone)]
struct ParagraphListRef {
    /// Canonical list ID (resolved from override)
    canonical_list_id: ListId,
    /// List kind (resolved from definition)
    kind: ListKind,
    /// Nesting level (from \ilvlN or default 0)
    level: u8,
}

```

### Modified Paragraph Finalization

The [`finalize_paragraph()`](../crates/rtfkit-core/src/interpreter.rs:601) method needs to be updated:

```rust
fn finalize_paragraph(&mut self) {
    // 1. Flush remaining text as a run (existing logic)
    if !self.current_text.is_empty() {
        let run = self.create_run();
        self.current_paragraph.runs.push(run);
        self.current_text.clear();
    }

    // 2. Skip empty paragraphs
    if self.current_paragraph.runs.is_empty() {
        self.current_paragraph = Paragraph::new();
        self.current_run_style = self.current_style.snapshot();
        self.paragraph_alignment = self.current_style.alignment;
        return;
    }

    // 3. Set alignment
    self.current_paragraph.alignment = self.paragraph_alignment;

    // 4. Resolve list metadata
    let list_ref = self.resolve_list_metadata();

    // 5. Handle based on list membership
    if let Some(ref) = list_ref {
        self.add_to_list(ref);
    } else {
        // Close any open lists before adding a non-list paragraph
        self.close_open_lists();
        self.document
            .blocks
            .push(Block::Paragraph(self.current_paragraph.clone()));
    }

    // 6. Update stats
    self.report_builder.increment_paragraph_count();
    self.report_builder
        .add_runs(self.current_paragraph.runs.len());

    // 7. Reset state
    self.current_paragraph = Paragraph::new();
    self.current_run_style = self.current_style.snapshot();
    self.paragraph_alignment = self.current_style.alignment;
    self.current_list_ref = None;
}

/// Resolve list metadata from explicit \ls.
fn resolve_list_metadata(&self) -> Option<ParagraphListRef> {
    // Explicit \lsN + \ilvlN
    if let Some(ref) = &self.current_list_ref {
        return Some(ref.clone());
    }

    None
}

/// Add current paragraph to appropriate list block.
fn add_to_list(&mut self, list_ref: ParagraphListRef) {
    let item = ListItem::from_paragraph(list_ref.level, self.current_paragraph.clone());

    // Check if we can append to the trailing list
    if let Some(last_list) = self.open_lists.last_mut() {
        if last_list.list_id == list_ref.canonical_list_id {
            last_list.add_item(item);
            return;
        }
    }

    // Need to start a new list
    self.close_open_lists();
    let mut new_list = ListBlock::new(list_ref.canonical_list_id, list_ref.kind);
    new_list.add_item(item);
    self.open_lists.push(new_list);
}

/// Close all open lists and add them to the document.
fn close_open_lists(&mut self) {
    for list in self.open_lists.drain(..) {
        if !list.is_empty() {
            self.document.blocks.push(Block::ListBlock(list));
        }
    }
}
```

### Control Word Handling Updates

Update [`handle_control_word()`](../crates/rtfkit-core/src/interpreter.rs:450) to handle list-related controls:

```rust
fn handle_control_word(&mut self, word: &str, parameter: Option<i32>) {
    // ... existing handling ...

    match word {
        // ... existing cases ...
        
        // List reference: \lsN
        "ls" => {
            if let Some(idx) = parameter {
                self.current_list_ref = self.resolve_ls_index(idx);
            }
        }
        
        // List level: \ilvlN
        "ilvl" => {
            if let Some(ref mut list_ref) = self.current_list_ref {
                list_ref.level = parameter.unwrap_or(0) as u8;
            }
        }
        
        // Legacy paragraph numbering controls are unsupported in Phase 3
        "pnlvl" | "pnlvlblt" | "pnlvlbody" | "pnlvlcont" => {
            self.report_builder.unsupported_list_control(word);
            self.report_builder
                .dropped_content("Dropped legacy paragraph numbering content", None);
        }
        
        // ... rest of existing handling ...
    }
}

/// Resolve \lsN index to a ParagraphListRef.
fn resolve_ls_index(&self, index: i32) -> Option<ParagraphListRef> {
    // Look up override
    let override_data = self.list_overrides.get(&index)?;
    
    // Look up list definition
    let definition = self.list_table.get(&override_data.list_id)?;
    
    // Determine kind from level 0
    let kind = definition.level_kinds.first().copied().unwrap_or_default();
    
    Some(ParagraphListRef {
        canonical_list_id: override_data.list_id as ListId,
        kind,
        level: 0,
    })
}
```

### Destination Handling Updates

Update [`destination_behavior()`](../crates/rtfkit-core/src/interpreter.rs:402) to parse list tables instead of dropping them:

```rust
fn destination_behavior(word: &str) -> Option<DestinationBehavior> {
    match word {
        // List tables - now parsed instead of treated as metadata
        "listtable" | "listoverridetable" => {
            // These need special handling - not dropped, not regular content
            // Will be handled by dedicated parsing state
            Some(DestinationBehavior::ListTable)
        }
        
        // Legacy paragraph numbering destinations are dropped in Phase 3
        "pntext" | "pntxtb" | "pntxta" => {
            Some(DestinationBehavior::Dropped(
                "Dropped legacy paragraph numbering text destination",
            ))
        }
        
        // ... rest of existing handling ...
    }
}
```

---

## DOCX Writer Changes

### Numbering Allocator

Add a numbering allocator to track IDs deterministically:

```rust
/// Allocates numbering IDs deterministically.
pub struct NumberingAllocator {
    /// Maps (kind, level_pattern) to abstractNumId
    abstract_num_defs: IndexMap<(ListKind, Vec<u8>), u32>,
    /// Maps list_id to numId
    num_ids: IndexMap<ListId, u32>,
    /// Next abstractNumId
    next_abstract_num_id: u32,
    /// Next numId
    next_num_id: u32,
}

impl NumberingAllocator {
    pub fn new() -> Self {
        Self {
            abstract_num_defs: IndexMap::new(),
            num_ids: IndexMap::new(),
            next_abstract_num_id: 0,
            next_num_id: 1, // numId starts at 1 in DOCX
        }
    }

    /// Get or create an abstractNumId for the given list definition.
    pub fn get_or_create_abstract_num(&mut self, kind: ListKind, levels: &[u8]) -> u32 {
        let key = (kind, levels.to_vec());
        if let Some(&id) = self.abstract_num_defs.get(&key) {
            return id;
        }
        let id = self.next_abstract_num_id;
        self.next_abstract_num_id += 1;
        self.abstract_num_defs.insert(key, id);
        id
    }

    /// Get or create a numId for the given list_id.
    pub fn get_or_create_num(&mut self, list_id: ListId) -> u32 {
        if let Some(&id) = self.num_ids.get(&list_id) {
            return id;
        }
        let id = self.next_num_id;
        self.next_num_id += 1;
        self.num_ids.insert(list_id, id);
        id
    }
}
```

### Modified Document Conversion

Update [`convert_document()`](../crates/rtfkit-docx/src/writer.rs:82):

```rust
fn convert_document(document: &Document) -> Result<docx_rs::XMLDocx, DocxError> {
    let mut doc = Docx::new();
    let mut numbering = NumberingAllocator::new();

    for block in &document.blocks {
        match block {
            Block::Paragraph(para) => {
                doc = doc.add_paragraph(convert_paragraph(para, None));
            }
            Block::ListBlock(list) => {
                for item in &list.items {
                    for item_block in &item.blocks {
                        if let Block::Paragraph(para) = item_block {
                            let num_pr = create_num_pr(&mut numbering, list, item.level);
                            doc = doc.add_paragraph(convert_paragraph(para, Some(num_pr)));
                        }
                    }
                }
            }
        }
    }

    // Add numbering part if needed
    if !numbering.is_empty() {
        doc = doc.add_numbering(build_numbering_part(&numbering));
    }

    Ok(doc.build())
}

/// Create w:numPr for a list paragraph.
fn create_num_pr(
    numbering: &mut NumberingAllocator,
    list: &ListBlock,
    level: u8,
) -> docx_rs::NumPr {
    let num_id = numbering.get_or_create_num(list.list_id);
    
    docx_rs::NumPr {
        ilvl: docx_rs::Level { val: level as i32 },
        num_id: docx_rs::NumId { val: num_id as i32 },
    }
}

/// Build the numbering.xml content.
fn build_numbering_part(numbering: &NumberingAllocator) -> docx_rs::Numbering {
    let mut num = docx_rs::Numbering::new();

    // Add abstractNum definitions
    for ((kind, levels), abstract_num_id) in &numbering.abstract_num_defs {
        let abstract_num = build_abstract_num(*kind, levels, *abstract_num_id);
        num = num.add_abstract_num(abstract_num);
    }

    // Add num instances
    for (list_id, num_id) in &numbering.num_ids {
        let abstract_num_id = numbering.get_abstract_num_for_list(*list_id);
        num = num.add_num(docx_rs::Num {
            num_id: *num_id as i32,
            abstract_num_id: abstract_num_id as i32,
        });
    }

    num
}

/// Build an abstractNum definition for a list kind.
fn build_abstract_num(kind: ListKind, levels: &[u8], abstract_num_id: u32) -> docx_rs::AbstractNum {
    let mut abstract_num = docx_rs::AbstractNum::new(abstract_num_id as i32);

    for (level_idx, &_level) in levels.iter().enumerate() {
        let level_format = match kind {
            ListKind::Bullet => docx_rs::LevelFormat::Bullet,
            ListKind::OrderedDecimal => docx_rs::LevelFormat::Decimal,
            ListKind::Mixed => docx_rs::LevelFormat::Decimal, // Fallback
        };

        let level = docx_rs::Level {
            level: level_idx as i32,
            start: docx_rs::Start { val: 1 },
            num_fmt: docx_rs::NumFmt { val: level_format },
            lvl_text: docx_rs::LvlText {
                val: match kind {
                    ListKind::Bullet => "•".to_string(),
                    ListKind::OrderedDecimal => format!("%{}.", level_idx + 1),
                    ListKind::Mixed => format!("%{}.", level_idx + 1),
                },
            },
            // ... additional level properties (indent, etc.)
        };

        abstract_num = abstract_num.add_level(level);
    }

    abstract_num
}
```

### Modified Paragraph Conversion

Update [`convert_paragraph()`](../crates/rtfkit-docx/src/writer.rs:97) to accept optional numbering:

```rust
fn convert_paragraph(para: &Paragraph, num_pr: Option<docx_rs::NumPr>) -> DocxParagraph {
    let mut p = DocxParagraph::new();

    // Add numbering properties if present
    if let Some(np) = num_pr {
        p = p.numbering(np);
    }

    // Map alignment
    p = p.align(convert_alignment(para.alignment));

    // Map runs
    for run in &para.runs {
        p = p.add_run(convert_run(run));
    }

    p
}
```

---

## Warning Types

### New Warning Variants

Add to [`Warning`](../crates/rtfkit-core/src/report.rs:50) enum:

```rust
/// A list-related control word was encountered but not fully supported.
///
/// This indicates list functionality that is recognized but partially implemented.
UnsupportedListControl {
    /// The control word that was encountered
    word: String,
    /// Optional parameter
    parameter: Option<i32>,
    /// Description of the limitation
    limitation: String,
    /// Severity of this warning
    severity: WarningSeverity,
},

/// A list override could not be resolved.
///
/// This indicates a reference to a list definition that doesn't exist or is malformed.
UnresolvedListOverride {
    /// The \ls index that couldn't be resolved
    ls_index: i32,
    /// Description of the issue
    reason: String,
    /// Severity of this warning
    severity: WarningSeverity,
},

/// List nesting level exceeds supported range.
///
/// DOCX supports levels 0-8; levels beyond this are clamped.
UnsupportedNestingLevel {
    /// The level that was encountered
    level: u8,
    /// The maximum supported level
    max_level: u8,
    /// Severity of this warning
    severity: WarningSeverity,
},
```

### Warning Helper Methods

Add to `Warning` impl:

```rust
/// Creates a new `UnsupportedListControl` warning.
pub fn unsupported_list_control(
    word: impl Into<String>,
    parameter: Option<i32>,
    limitation: impl Into<String>,
) -> Self {
    Warning::UnsupportedListControl {
        word: word.into(),
        parameter,
        limitation: limitation.into(),
        severity: WarningSeverity::Warning,
    }
}

/// Creates a new `UnresolvedListOverride` warning.
pub fn unresolved_list_override(ls_index: i32, reason: impl Into<String>) -> Self {
    Warning::UnresolvedListOverride {
        ls_index,
        reason: reason.into(),
        severity: WarningSeverity::Warning,
    }
}

/// Creates a new `UnsupportedNestingLevel` warning.
pub fn unsupported_nesting_level(level: u8, max_level: u8) -> Self {
    Warning::UnsupportedNestingLevel {
        level,
        max_level,
        severity: WarningSeverity::Info,
    }
}
```

### ReportBuilder Updates

Add to [`ReportBuilder`](../crates/rtfkit-core/src/report.rs:234):

```rust
/// Records an unsupported list control word.
pub fn unsupported_list_control(
    &mut self,
    word: &str,
    parameter: Option<i32>,
    limitation: &str,
) {
    if self.can_add_warning() {
        self.warnings
            .push(Warning::unsupported_list_control(word, parameter, limitation));
    }
}

/// Records an unresolved list override.
pub fn unresolved_list_override(&mut self, ls_index: i32, reason: &str) {
    if self.can_add_warning() {
        self.warnings
            .push(Warning::unresolved_list_override(ls_index, reason));
    }
}

/// Records an unsupported nesting level.
pub fn unsupported_nesting_level(&mut self, level: u8, max_level: u8) {
    if self.can_add_warning() {
        self.warnings
            .push(Warning::unsupported_nesting_level(level, max_level));
    }
}
```

---

## Summary of Changes

### Files Modified

| File | Changes |
|------|---------|
| [`crates/rtfkit-core/src/lib.rs`](../crates/rtfkit-core/src/lib.rs) | Add `ListId`, `ListKind`, `ListItem`, `ListBlock` types; update `Block` enum |
| [`crates/rtfkit-core/src/interpreter.rs`](../crates/rtfkit-core/src/interpreter.rs) | Add list state tracking; update `finalize_paragraph()`; add list control word handling |
| [`crates/rtfkit-core/src/report.rs`](../crates/rtfkit-core/src/report.rs) | Add `UnsupportedListControl`, `UnresolvedListOverride`, `UnsupportedNestingLevel` warnings |
| [`crates/rtfkit-docx/src/writer.rs`](../crates/rtfkit-docx-docx/src/writer.rs) | Add `NumberingAllocator`; update `convert_document()`; add numbering XML generation |

### IR Serialization Impact

The IR JSON format will change. Example before:

```json
{
  "blocks": [
    {"type": "paragraph", "alignment": "left", "runs": [{"text": "Item 1", "bold": false, "italic": false, "underline": false}]}
  ]
}
```

Example after (with list):

```json
{
  "blocks": [
    {
      "type": "list_block",
      "list_id": 1,
      "kind": "bullet",
      "items": [
        {
          "level": 0,
          "blocks": [
            {"type": "paragraph", "alignment": "left", "runs": [{"text": "Item 1", "bold": false, "italic": false, "underline": false}]}
          ]
        }
      ]
    }
  ]
}
```

### Golden Test Updates Required

All golden test files in [`golden/`](../golden/) will need to be regenerated after the IR changes.

---

## Implementation Order

1. **IR Model Extension** - Add types to `lib.rs`
2. **Warning Types** - Add new warning variants to `report.rs`
3. **Interpreter State** - Add list tracking fields to `Interpreter`
4. **Paragraph Finalization** - Update `finalize_paragraph()` logic
5. **Control Word Handling** - Add `\ls`, `\ilvl`, and legacy `\pn` degradation warnings
6. **DOCX Writer** - Add numbering allocator and XML generation
7. **Golden Tests** - Regenerate golden files

---

## Open Questions

1. **List table parsing complexity**: Should we fully parse `\listtable` and `\listoverridetable` destinations in Phase 3, or use a simpler heuristic approach initially?

2. **Level kind tracking**: Should `ListBlock.kind` be per-list or per-level? The spec suggests per-list, but RTF allows different kinds per level.

3. **docx-rs API**: The exact API for adding numbering to paragraphs needs verification against the current docx-rs version.
