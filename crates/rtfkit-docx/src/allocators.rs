//! Deterministic ID allocators for DOCX output.
//!
//! Tracks numbering definitions and image relationship IDs to ensure
//! reproducible DOCX output regardless of iteration order.

use docx_rs::{
    AbstractNumbering, Level, LevelJc, LevelText, NumberFormat, Numbering, Numberings, Start,
};
use indexmap::IndexMap;
use rtfkit_core::{ListBlock, ListId, ListKind, Note};
use rtfkit_style_tokens::StyleProfile;
use std::collections::BTreeMap;

// =============================================================================
// Note lookup alias
// =============================================================================

/// Maps a note ID to its full Note IR node, used when rendering footnote bodies.
pub type NoteLookup = BTreeMap<u32, Note>;

// =============================================================================
// Numbering Allocator
// =============================================================================

/// Allocates numbering IDs deterministically for DOCX output.
///
/// The `NumberingAllocator` tracks abstract numbering definitions and concrete
/// numbering instances, ensuring deterministic ID assignment for reproducible
/// DOCX output.
#[derive(Debug, Clone)]
pub struct NumberingAllocator {
    /// Maps (ListKind, level pattern) -> abstractNumId
    /// Uses IndexMap for deterministic iteration order
    abstract_num_ids: IndexMap<(ListKind, Vec<u8>), u32>,
    /// Maps ListId -> (numId, abstractNumId)
    /// Tracks which numId and abstractNumId each list uses
    list_to_num: IndexMap<ListId, (u32, u32)>,
    /// Next abstractNumId to assign
    next_abstract_num_id: u32,
    /// Next numId to assign (starts at 2 since 1 is reserved by docx-rs)
    next_num_id: u32,
    /// Left indent step per level in twips (default: 420 = 21pt).
    indent_step_twips: i32,
    /// Hanging indent (marker gap) in twips (default: 420 = 21pt).
    marker_gap_twips: i32,
}

impl NumberingAllocator {
    /// Creates a new empty NumberingAllocator with default indentation (21pt / 420 twips).
    pub fn new() -> Self {
        Self {
            abstract_num_ids: IndexMap::new(),
            list_to_num: IndexMap::new(),
            next_abstract_num_id: 0,
            next_num_id: 2, // Start at 2, since docx-rs reserves 1 for default
            indent_step_twips: 420,
            marker_gap_twips: 420,
        }
    }

    /// Creates a `NumberingAllocator` with indentation values from a style profile.
    pub fn with_profile(profile: &StyleProfile) -> Self {
        let mut s = Self::new();
        s.indent_step_twips = (profile.components.list.indentation_step * 20.0).round() as i32;
        s.marker_gap_twips = (profile.components.list.marker_gap * 20.0).round() as i32;
        s
    }

    /// Registers a list and returns its numId.
    ///
    /// This method ensures that:
    /// - Each unique (ListKind, levels) combination gets a unique abstractNumId
    /// - Each unique ListId gets a unique numId
    /// - The same ListId always maps to the same numId (determinism)
    pub fn register_list(&mut self, list: &ListBlock) -> u32 {
        // Check if we've already registered this list
        if let Some((num_id, _)) = self.list_to_num.get(&list.list_id) {
            return *num_id;
        }

        // Determine the levels used in this list
        let levels = self.extract_levels(list);

        // Get or create abstractNumId for this (kind, levels) combination
        let key = (list.kind, levels.clone());
        let abstract_num_id = if let Some(&id) = self.abstract_num_ids.get(&key) {
            id
        } else {
            let id = self.next_abstract_num_id;
            self.next_abstract_num_id += 1;
            self.abstract_num_ids.insert(key, id);
            id
        };

        // Assign a new numId for this list
        let num_id = self.next_num_id;
        self.next_num_id += 1;
        self.list_to_num
            .insert(list.list_id, (num_id, abstract_num_id));

        num_id
    }

    /// Returns the assigned numId for a given ListId, if registered.
    pub fn num_id_for(&self, list_id: ListId) -> Option<u32> {
        self.list_to_num.get(&list_id).map(|(num_id, _)| *num_id)
    }

    /// Extracts the unique levels used in a list.
    fn extract_levels(&self, list: &ListBlock) -> Vec<u8> {
        let mut levels: Vec<u8> = list.items.iter().map(|item| item.level).collect();
        levels.sort();
        levels.dedup();
        levels
    }

    /// Returns true if any lists have been registered.
    pub fn has_numbering(&self) -> bool {
        !self.list_to_num.is_empty()
    }

    /// Builds the Numberings structure for DOCX output.
    pub fn build_numberings(&self) -> Numberings {
        let mut numberings = Numberings::new();

        // Add abstract numbering definitions
        for ((kind, levels), abstract_num_id) in &self.abstract_num_ids {
            let abstract_num = self.build_abstract_num(*kind, levels, *abstract_num_id);
            numberings = numberings.add_abstract_numbering(abstract_num);
        }

        // Add numbering instances
        for (_list_id, (num_id, abstract_num_id)) in &self.list_to_num {
            numberings = numberings
                .add_numbering(Numbering::new(*num_id as usize, *abstract_num_id as usize));
        }

        numberings
    }

    /// Builds an AbstractNumbering definition for a list kind.
    fn build_abstract_num(
        &self,
        kind: ListKind,
        levels: &[u8],
        abstract_num_id: u32,
    ) -> AbstractNumbering {
        let mut abstract_num = AbstractNumbering::new(abstract_num_id as usize);

        // Determine max level needed
        let max_level = levels.iter().max().copied().unwrap_or(0);

        // Add level definitions for all levels up to max_level
        for level_idx in 0..=max_level {
            let level = self.build_level(kind, level_idx);
            abstract_num = abstract_num.add_level(level);
        }

        abstract_num
    }

    /// Builds a Level definition for a specific level index.
    fn build_level(&self, kind: ListKind, level_idx: u8) -> Level {
        let (format, text) = match kind {
            ListKind::Bullet => ("bullet", "•".to_string()),
            ListKind::OrderedDecimal => ("decimal", format!("%{}.", level_idx + 1)),
            ListKind::Mixed => ("decimal", format!("%{}.", level_idx + 1)),
        };

        // Calculate indentation based on level
        let left_indent = self.indent_step_twips * (level_idx as i32 + 1);
        let hanging_indent = self.marker_gap_twips;

        Level::new(
            level_idx as usize,
            Start::new(1),
            NumberFormat::new(format),
            LevelText::new(&text),
            LevelJc::new("left"),
        )
        .indent(
            Some(left_indent),
            Some(docx_rs::SpecialIndentType::Hanging(hanging_indent)),
            None,
            None,
        )
    }
}

impl Default for NumberingAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Image Allocator
// =============================================================================

/// Allocates image IDs and tracks images for DOCX output.
///
/// This allocator only tracks deterministic relationship IDs. Media packaging
/// is handled natively by `docx-rs` drawing support.
#[derive(Debug, Clone)]
pub struct ImageAllocator {
    /// Next image number for deterministic relationship IDs.
    next_image_num: u32,
}

impl ImageAllocator {
    /// Creates a new empty ImageAllocator.
    pub fn new() -> Self {
        Self { next_image_num: 1 }
    }

    /// Allocate the next deterministic image ID (1-based).
    pub fn allocate_image_id(&mut self) -> u32 {
        let id = self.next_image_num;
        self.next_image_num += 1;
        id
    }
}

impl Default for ImageAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rtfkit_core::{ListItem, ListKind};

    #[test]
    fn test_numbering_allocator_new() {
        let allocator = NumberingAllocator::new();
        assert!(!allocator.has_numbering());
        assert!(allocator.abstract_num_ids.is_empty());
        assert!(allocator.list_to_num.is_empty());
    }

    #[test]
    fn test_numbering_allocator_single_list() {
        let mut allocator = NumberingAllocator::new();

        let list = ListBlock::new(1, ListKind::Bullet);
        let num_id = allocator.register_list(&list);

        assert!(allocator.has_numbering());
        assert_eq!(num_id, 2); // First numId (starts at 2)
        assert_eq!(allocator.abstract_num_ids.len(), 1);
    }

    #[test]
    fn test_numbering_allocator_same_list_twice() {
        let mut allocator = NumberingAllocator::new();

        let list = ListBlock::new(1, ListKind::Bullet);
        let num_id1 = allocator.register_list(&list);
        let num_id2 = allocator.register_list(&list);

        // Same list should return same numId
        assert_eq!(num_id1, num_id2);
        assert_eq!(allocator.list_to_num.len(), 1);
    }

    #[test]
    fn test_numbering_allocator_different_lists_same_kind() {
        let mut allocator = NumberingAllocator::new();

        let list1 = ListBlock::new(1, ListKind::Bullet);
        let list2 = ListBlock::new(2, ListKind::Bullet);

        let num_id1 = allocator.register_list(&list1);
        let num_id2 = allocator.register_list(&list2);

        // Different lists should get different numIds
        assert_ne!(num_id1, num_id2);
        // But same abstractNumId (same kind)
        let (_, abs1) = allocator.list_to_num.get(&1).unwrap();
        let (_, abs2) = allocator.list_to_num.get(&2).unwrap();
        assert_eq!(abs1, abs2);
    }

    #[test]
    fn test_numbering_allocator_different_kinds() {
        let mut allocator = NumberingAllocator::new();

        let bullet_list = ListBlock::new(1, ListKind::Bullet);
        let decimal_list = ListBlock::new(2, ListKind::OrderedDecimal);

        allocator.register_list(&bullet_list);
        allocator.register_list(&decimal_list);

        // Different kinds should get different abstractNumIds
        assert_eq!(allocator.abstract_num_ids.len(), 2);
    }

    #[test]
    fn test_numbering_allocator_determinism() {
        // Test that the same input always produces the same output
        let mut allocator1 = NumberingAllocator::new();
        let mut allocator2 = NumberingAllocator::new();

        let list1 = ListBlock::new(1, ListKind::Bullet);
        let list2 = ListBlock::new(2, ListKind::OrderedDecimal);
        let list3 = ListBlock::new(3, ListKind::Bullet);

        let num_id1_1 = allocator1.register_list(&list1);
        let num_id1_2 = allocator1.register_list(&list2);
        let num_id1_3 = allocator1.register_list(&list3);

        let num_id2_1 = allocator2.register_list(&list1);
        let num_id2_2 = allocator2.register_list(&list2);
        let num_id2_3 = allocator2.register_list(&list3);

        assert_eq!(num_id1_1, num_id2_1);
        assert_eq!(num_id1_2, num_id2_2);
        assert_eq!(num_id1_3, num_id2_3);
    }

    #[test]
    fn test_numbering_allocator_build_numberings() {
        let mut allocator = NumberingAllocator::new();

        let list = ListBlock::new(1, ListKind::Bullet);
        allocator.register_list(&list);

        let numberings = allocator.build_numberings();

        // Should have one abstract numbering and one numbering instance
        assert_eq!(numberings.abstract_nums.len(), 1);
        assert_eq!(numberings.numberings.len(), 1);
    }

    #[test]
    fn test_numbering_allocator_levels_extraction() {
        let mut allocator = NumberingAllocator::new();

        let mut list = ListBlock::new(1, ListKind::OrderedDecimal);
        list.add_item(ListItem::new(0));
        list.add_item(ListItem::new(1));
        list.add_item(ListItem::new(0)); // Duplicate level

        allocator.register_list(&list);

        // Should have one abstract num with levels 0 and 1
        let key = (ListKind::OrderedDecimal, vec![0, 1]);
        assert!(allocator.abstract_num_ids.contains_key(&key));
    }

    #[test]
    fn test_numbering_xml_content() {
        let mut allocator = NumberingAllocator::new();

        let mut list = ListBlock::new(1, ListKind::Bullet);
        list.add_item(ListItem::new(0));

        allocator.register_list(&list);

        let numberings = allocator.build_numberings();

        // Check that we have the expected structure
        assert_eq!(numberings.abstract_nums.len(), 1);
        assert_eq!(numberings.numberings.len(), 1);

        // Check abstract numbering ID
        let abstract_num = &numberings.abstract_nums[0];
        assert_eq!(abstract_num.id, 0);

        // Check numbering instance
        let numbering = &numberings.numberings[0];
        assert_eq!(numbering.id, 2); // numId starts at 2
        assert_eq!(numbering.abstract_num_id, 0);
    }
}
