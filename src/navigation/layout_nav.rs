use crate::cursor::ViewPosition;
use crate::ui::view_pipeline::Layout;
use crate::viewport::Viewport;

/// Move vertically by view lines within a layout, preserving preferred column when provided.
pub fn move_vertical(
    layout: &Layout,
    cursor: &ViewPosition,
    preferred_col: Option<usize>,
    direction: isize,
) -> ViewPosition {
    let current_line = cursor.view_line;
    let target_line = ((current_line as isize) + direction)
        .max(0)
        .min((layout.lines.len().saturating_sub(1)) as isize) as usize;
    let target_col = preferred_col.unwrap_or(cursor.column);
    ViewPosition {
        view_line: target_line,
        column: target_col,
        source_byte: layout.view_position_to_source_byte(target_line, target_col),
    }
}

/// Move horizontally, crossing line boundaries when needed.
/// When at end of line and moving right, crosses to start of next line.
/// When at start of line and moving left, crosses to end of previous line.
pub fn move_horizontal(
    layout: &Layout,
    cursor: &ViewPosition,
    direction: isize,
) -> ViewPosition {
    let line_idx = cursor.view_line.min(layout.lines.len().saturating_sub(1));
    let line_len = layout.lines.get(line_idx).map(|l| l.char_mappings.len()).unwrap_or(0);
    let raw_col = (cursor.column as isize + direction).max(0) as usize;

    // Handle crossing to next line when moving right past end of line
    // Use >= because line_len includes the newline char, and we want to cross when moving past it
    if direction > 0 && raw_col >= line_len && line_idx + 1 < layout.lines.len() {
        let next_line_idx = line_idx + 1;
        return ViewPosition {
            view_line: next_line_idx,
            column: 0,
            source_byte: layout.view_position_to_source_byte(next_line_idx, 0),
        };
    }

    // Handle crossing to previous line when moving left past start of line
    if direction < 0 && cursor.column == 0 && line_idx > 0 {
        let prev_line_idx = line_idx - 1;
        let prev_line_len = layout
            .lines
            .get(prev_line_idx)
            .map(|l| l.char_mappings.len())
            .unwrap_or(0);
        return ViewPosition {
            view_line: prev_line_idx,
            column: prev_line_len,
            source_byte: layout.view_position_to_source_byte(prev_line_idx, prev_line_len),
        };
    }

    // Normal case: stay on current line
    let target_col = raw_col.min(line_len);
    ViewPosition {
        view_line: line_idx,
        column: target_col,
        source_byte: layout.view_position_to_source_byte(line_idx, target_col),
    }
}

/// Move to the start of the current view line.
pub fn move_line_start(layout: &Layout, cursor: &ViewPosition) -> ViewPosition {
    let line_idx = cursor.view_line.min(layout.lines.len().saturating_sub(1));
    ViewPosition {
        view_line: line_idx,
        column: 0,
        source_byte: layout.view_position_to_source_byte(line_idx, 0),
    }
}

/// Move to the end of the current view line.
/// End of line means after the last character (column = line_len), not at the last character.
pub fn move_line_end(layout: &Layout, cursor: &ViewPosition) -> ViewPosition {
    let line_idx = cursor.view_line.min(layout.lines.len().saturating_sub(1));
    let line_len = layout.lines.get(line_idx).map(|l| l.char_mappings.len()).unwrap_or(0);
    // Column should be at line_len (after last char), not line_len-1 (at last char)
    let col = line_len;
    ViewPosition {
        view_line: line_idx,
        column: col,
        source_byte: layout.view_position_to_source_byte(line_idx, col),
    }
}

/// Move by a page (viewport height) in view lines.
pub fn move_page(
    layout: &Layout,
    cursor: &ViewPosition,
    viewport: &Viewport,
    direction: isize,
) -> ViewPosition {
    let page = viewport.visible_line_count().saturating_sub(1);
    let delta = (page as isize) * direction;
    move_vertical(layout, cursor, Some(cursor.column), delta)
}

/// Scroll the viewport by view lines.
pub fn scroll_view(layout: &Layout, viewport: &mut Viewport, line_offset: isize) {
    let max_top = layout.max_top_line(viewport.visible_line_count());
    let target = (viewport.top_view_line as isize + line_offset).max(0) as usize;
    viewport.top_view_line = target.min(max_top);
    if let Some(byte) = layout.get_source_byte_for_line(viewport.top_view_line) {
        viewport.anchor_byte = byte;
    }
}

/// Move to the start of the previous word in view coordinates.
/// Note: Requires access to buffer context; will be called from action_convert with buffer access.
pub fn move_word_left(layout: &Layout, cursor: &ViewPosition, buffer: &crate::text_buffer::Buffer) -> ViewPosition {
    crate::word_navigation::find_word_start_left_view(layout, cursor, buffer)
}

/// Move to the start of the next word in view coordinates.
/// Note: Requires access to buffer context; will be called from action_convert with buffer access.
pub fn move_word_right(layout: &Layout, cursor: &ViewPosition, buffer: &crate::text_buffer::Buffer) -> ViewPosition {
    crate::word_navigation::find_word_start_right_view(layout, cursor, buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::view_pipeline::{ViewLine, LineStart};
    use proptest::prelude::*;
    use std::collections::HashSet;

    /// Create a simple test layout with the given line contents
    fn make_test_layout(lines: &[&str]) -> Layout {
        let view_lines: Vec<ViewLine> = lines
            .iter()
            .enumerate()
            .map(|(idx, content)| {
                let has_newline = idx < lines.len() - 1; // All but last line have newlines
                let mut char_mappings: Vec<Option<usize>> = Vec::new();
                let mut byte_offset = lines[..idx].iter().map(|l| l.len() + 1).sum::<usize>(); // +1 for newlines

                for ch in content.chars() {
                    char_mappings.push(Some(byte_offset));
                    byte_offset += ch.len_utf8();
                }

                // Add newline mapping if line has newline
                if has_newline {
                    char_mappings.push(Some(byte_offset));
                }

                ViewLine {
                    text: if has_newline {
                        format!("{}\n", content)
                    } else {
                        content.to_string()
                    },
                    char_mappings,
                    char_styles: vec![],
                    tab_starts: HashSet::new(),
                    line_start: LineStart::Beginning,
                    ends_with_newline: has_newline,
                }
            })
            .collect();

        let total_bytes: usize = lines.iter().map(|l| l.len()).sum::<usize>() + lines.len().saturating_sub(1);
        Layout::new(view_lines, 0..total_bytes)
    }

    // Property: move_line_start always returns column 0
    proptest! {
        #[test]
        fn prop_move_line_start_returns_column_zero(
            view_line in 0usize..10,
            column in 0usize..100,
        ) {
            let layout = make_test_layout(&["Hello", "World", "Test"]);
            let cursor = ViewPosition {
                view_line: view_line.min(2),
                column,
                source_byte: None,
            };
            let result = move_line_start(&layout, &cursor);
            prop_assert_eq!(result.column, 0);
        }
    }

    // Property: move_line_end returns column = line_len
    proptest! {
        #[test]
        fn prop_move_line_end_returns_line_len(
            view_line in 0usize..10,
            column in 0usize..100,
        ) {
            let layout = make_test_layout(&["Hello", "World", "Test"]);
            let clamped_line = view_line.min(2);
            let cursor = ViewPosition {
                view_line: clamped_line,
                column,
                source_byte: None,
            };
            let result = move_line_end(&layout, &cursor);
            let expected_len = layout.lines[result.view_line].char_mappings.len();
            prop_assert_eq!(result.column, expected_len);
        }
    }

    // Property: move_vertical clamps view_line to valid range
    proptest! {
        #[test]
        fn prop_move_vertical_clamps_to_valid_range(
            view_line in 0usize..100,
            column in 0usize..100,
            direction in -100isize..100isize,
        ) {
            let layout = make_test_layout(&["Line1", "Line2", "Line3"]);
            let cursor = ViewPosition {
                view_line,
                column,
                source_byte: None,
            };
            let result = move_vertical(&layout, &cursor, Some(column), direction);
            prop_assert!(result.view_line < layout.lines.len());
        }
    }

    // Property: move_horizontal never produces invalid view_line
    proptest! {
        #[test]
        fn prop_move_horizontal_valid_view_line(
            view_line in 0usize..10,
            column in 0usize..20,
            direction in -5isize..5isize,
        ) {
            let layout = make_test_layout(&["Hello", "World", "Test"]);
            let clamped_line = view_line.min(2);
            let cursor = ViewPosition {
                view_line: clamped_line,
                column,
                source_byte: None,
            };
            let result = move_horizontal(&layout, &cursor, direction);
            prop_assert!(result.view_line < layout.lines.len(),
                "view_line {} should be < {}", result.view_line, layout.lines.len());
        }
    }

    // Unit test: move right from end of line crosses to next line
    #[test]
    fn test_move_horizontal_crosses_to_next_line() {
        let layout = make_test_layout(&["Hello", "World"]);
        // "Hello" has 5 chars + 1 newline = 6 char_mappings
        let line_len = layout.lines[0].char_mappings.len();
        assert_eq!(line_len, 6); // H, e, l, l, o, \n

        // Cursor at end of line 0 (at the newline position)
        let cursor = ViewPosition {
            view_line: 0,
            column: 5, // At '\n'
            source_byte: Some(5),
        };

        let result = move_horizontal(&layout, &cursor, 1);
        assert_eq!(result.view_line, 1, "Should cross to line 1");
        assert_eq!(result.column, 0, "Should be at start of line 1");
    }

    // Unit test: move left from start of line crosses to previous line
    #[test]
    fn test_move_horizontal_crosses_to_prev_line() {
        let layout = make_test_layout(&["Hello", "World"]);

        // Cursor at start of line 1
        let cursor = ViewPosition {
            view_line: 1,
            column: 0,
            source_byte: Some(6),
        };

        let result = move_horizontal(&layout, &cursor, -1);
        assert_eq!(result.view_line, 0, "Should cross to line 0");
        // Should be at end of line 0 (including newline)
        let expected_col = layout.lines[0].char_mappings.len();
        assert_eq!(result.column, expected_col, "Should be at end of line 0");
    }

    // Unit test: move right stays on line when not at end
    #[test]
    fn test_move_horizontal_stays_on_line() {
        let layout = make_test_layout(&["Hello", "World"]);

        let cursor = ViewPosition {
            view_line: 0,
            column: 2, // At 'l'
            source_byte: Some(2),
        };

        let result = move_horizontal(&layout, &cursor, 1);
        assert_eq!(result.view_line, 0, "Should stay on line 0");
        assert_eq!(result.column, 3, "Should move to column 3");
    }

    // Unit test: move_line_end on multi-line layout
    #[test]
    fn test_move_line_end_multiline() {
        let layout = make_test_layout(&["Hello", "World!", "Test"]);

        // Line 1 has "World!" + newline = 7 chars
        let cursor = ViewPosition {
            view_line: 1,
            column: 0,
            source_byte: None,
        };

        let result = move_line_end(&layout, &cursor);
        assert_eq!(result.view_line, 1);
        assert_eq!(result.column, layout.lines[1].char_mappings.len());
    }
}
