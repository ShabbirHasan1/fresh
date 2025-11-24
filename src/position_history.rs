/// View-centric position history for back/forward navigation (VS Code style).
///
/// Tracks cursor movements as view coordinates and coalesces small moves into a
/// single history entry. This lets users jump back/forward through meaningful
/// locations instead of every keystroke.
use crate::event::{BufferId, ViewEventPosition};

/// A single entry in the position history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionEntry {
    /// The buffer ID.
    pub buffer_id: BufferId,
    /// Cursor position in view coordinates (with optional source mapping).
    pub position: ViewEventPosition,
    /// Optional selection anchor in view coordinates.
    pub anchor: Option<ViewEventPosition>,
}

impl PositionEntry {
    /// Create a new position entry.
    pub fn new(
        buffer_id: BufferId,
        position: ViewEventPosition,
        anchor: Option<ViewEventPosition>,
    ) -> Self {
        Self {
            buffer_id,
            position,
            anchor,
        }
    }
}

/// Pending movement that may be coalesced with subsequent movements.
#[derive(Clone, Debug)]
struct PendingMovement {
    start_entry: PositionEntry,
}

/// View-line threshold for a "large" jump that should break coalescing.
const LARGE_LINE_THRESHOLD: usize = 5;
/// Column threshold (on the same line) for a "large" jump.
const LARGE_COLUMN_THRESHOLD: usize = 80;

/// Position history manager.
///
/// Stores a stack of positions with a current index for back/forward navigation.
/// Movements are coalesced: consecutive small moves are treated as a single jump.
pub struct PositionHistory {
    entries: Vec<PositionEntry>,
    current_index: Option<usize>,
    max_entries: usize,
    pending_movement: Option<PendingMovement>,
}

impl PositionHistory {
    /// Create a new position history with default max entries (100).
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Create a new position history with specified max entries.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            current_index: None,
            max_entries,
            pending_movement: None,
        }
    }

    /// Record a cursor movement event (view-centric).
    ///
    /// Called for every MoveCursor. Coalesces small moves; commits when:
    /// - Buffer changes
    /// - Jump exceeds thresholds
    /// - Navigation back/forward is requested
    pub fn record_movement(
        &mut self,
        buffer_id: BufferId,
        position: ViewEventPosition,
        anchor: Option<ViewEventPosition>,
    ) {
        let entry = PositionEntry::new(buffer_id, position, anchor);

        if let Some(pending) = &mut self.pending_movement {
            if pending.start_entry.buffer_id == buffer_id {
                if !is_large_jump(&pending.start_entry.position, &position) {
                    // Small move: keep coalescing.
                    return;
                }
            }
            // Different buffer or large jump: commit pending.
            self.commit_pending_movement();
        }

        // Start a new pending movement.
        self.pending_movement = Some(PendingMovement { start_entry: entry });
    }

    /// Commit any pending movement to history.
    pub fn commit_pending_movement(&mut self) {
        if let Some(pending) = self.pending_movement.take() {
            self.push(pending.start_entry);
        }
    }

    /// Push a new position to the history (truncates forward history if needed).
    pub fn push(&mut self, entry: PositionEntry) {
        if let Some(current_idx) = self.current_index {
            self.entries.truncate(current_idx + 1);
        }

        if let Some(current_idx) = self.current_index {
            if current_idx < self.entries.len() && self.entries[current_idx] == entry {
                return;
            }
        }

        self.entries.push(entry);

        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        self.current_index = Some(self.entries.len() - 1);
    }

    /// Navigate back in history (commits pending movement first).
    pub fn back(&mut self) -> Option<&PositionEntry> {
        self.commit_pending_movement();

        match self.current_index {
            Some(0) | None => None,
            Some(idx) => {
                self.current_index = Some(idx - 1);
                self.entries.get(idx - 1)
            }
        }
    }

    /// Navigate forward in history.
    pub fn forward(&mut self) -> Option<&PositionEntry> {
        match self.current_index {
            None => None,
            Some(idx) if idx + 1 >= self.entries.len() => None,
            Some(idx) => {
                self.current_index = Some(idx + 1);
                self.entries.get(idx + 1)
            }
        }
    }

    /// Check if we can go back.
    pub fn can_go_back(&self) -> bool {
        matches!(self.current_index, Some(idx) if idx > 0)
    }

    /// Check if we can go forward.
    pub fn can_go_forward(&self) -> bool {
        matches!(self.current_index, Some(idx) if idx + 1 < self.entries.len())
    }

    /// Get the current position entry.
    pub fn current(&self) -> Option<&PositionEntry> {
        self.current_index.and_then(|idx| self.entries.get(idx))
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_index = None;
        self.pending_movement = None;
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is history empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get current index (for debugging/inspection).
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }
}

impl Default for PositionHistory {
    fn default() -> Self {
        Self::new()
    }
}

fn is_large_jump(from: &ViewEventPosition, to: &ViewEventPosition) -> bool {
    let line_delta = from
        .view_line
        .max(to.view_line)
        .saturating_sub(from.view_line.min(to.view_line));
    let col_delta = from
        .column
        .max(to.column)
        .saturating_sub(from.column.min(to.column));

    line_delta > LARGE_LINE_THRESHOLD || (line_delta == 0 && col_delta > LARGE_COLUMN_THRESHOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp(line: usize, col: usize) -> ViewEventPosition {
        ViewEventPosition {
            view_line: line,
            column: col,
            source_byte: None,
        }
    }

    fn make_entry(buffer_id: usize, position: ViewEventPosition) -> PositionEntry {
        PositionEntry::new(BufferId(buffer_id), position, None)
    }

    #[test]
    fn new_history_is_empty() {
        let history = PositionHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn push_and_current() {
        let mut history = PositionHistory::new();
        let entry = make_entry(1, vp(10, 5));
        history.push(entry.clone());

        assert_eq!(history.len(), 1);
        assert_eq!(history.current(), Some(&entry));
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn back_and_forward_navigation() {
        let mut history = PositionHistory::new();
        let entry1 = make_entry(1, vp(0, 0));
        let entry2 = make_entry(1, vp(1, 0));
        let entry3 = make_entry(2, vp(2, 10));

        history.push(entry1.clone());
        history.push(entry2.clone());
        history.push(entry3.clone());

        assert_eq!(history.back(), Some(&entry2));
        assert_eq!(history.current(), Some(&entry2));
        assert!(history.can_go_back());
        assert!(history.can_go_forward());

        assert_eq!(history.back(), Some(&entry1));
        assert_eq!(history.current(), Some(&entry1));
        assert!(!history.can_go_back());

        assert_eq!(history.forward(), Some(&entry2));
        assert_eq!(history.forward(), Some(&entry3));
        assert!(history.forward().is_none());
    }

    #[test]
    fn truncate_forward_history_on_push() {
        let mut history = PositionHistory::new();
        let entry1 = make_entry(1, vp(0, 0));
        let entry2 = make_entry(1, vp(1, 0));
        let entry3 = make_entry(1, vp(2, 0));

        history.push(entry1.clone());
        history.push(entry2.clone());
        history.push(entry3.clone());

        history.back();
        history.back();
        assert_eq!(history.current(), Some(&entry1));

        let new_entry = make_entry(1, vp(10, 0));
        history.push(new_entry.clone());

        assert_eq!(history.len(), 2);
        assert_eq!(history.current(), Some(&new_entry));
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn duplicate_consecutive_entries_not_added() {
        let mut history = PositionHistory::new();
        let entry = make_entry(1, vp(0, 0));

        history.push(entry.clone());
        history.push(entry.clone());
        history.push(entry.clone());

        assert_eq!(history.len(), 1);
    }

    #[test]
    fn respects_max_entries() {
        let mut history = PositionHistory::with_capacity(3);
        history.push(make_entry(1, vp(0, 0)));
        history.push(make_entry(1, vp(1, 0)));
        history.push(make_entry(1, vp(2, 0)));
        history.push(make_entry(1, vp(3, 0)));
        history.push(make_entry(1, vp(4, 0)));

        assert_eq!(history.len(), 3);
        assert_eq!(history.current(), Some(&make_entry(1, vp(4, 0))));
    }

    #[test]
    fn clears_all_state() {
        let mut history = PositionHistory::new();
        history.push(make_entry(1, vp(0, 0)));
        history.push(make_entry(1, vp(1, 0)));

        history.clear();
        assert!(history.is_empty());
        assert_eq!(history.current(), None);
        assert_eq!(history.current_index(), None);
    }

    #[test]
    fn coalesces_small_moves_and_commits_large() {
        let mut history = PositionHistory::new();
        // Small vertical moves within threshold: should coalesce.
        history.record_movement(BufferId(1), vp(0, 0), None);
        history.record_movement(BufferId(1), vp(1, 0), None);
        history.record_movement(BufferId(1), vp(2, 0), None);
        assert!(history.is_empty()); // pending only

        // Large jump triggers commit.
        history.record_movement(BufferId(1), vp(20, 0), None);
        assert_eq!(history.len(), 1);
        assert_eq!(history.current().unwrap().position.view_line, 2);

        // Buffer change also commits.
        history.record_movement(BufferId(2), vp(0, 0), None);
        assert_eq!(history.len(), 2);
    }
}
