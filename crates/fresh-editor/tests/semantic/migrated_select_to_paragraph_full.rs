//! Faithful migration of `tests/e2e/select_to_paragraph.rs`.
//!
//! Originals send `Ctrl+Shift+Up/Down` which the editor binds to
//! `Action::SelectToParagraphUp`/`SelectToParagraphDown` —
//! "extend selection to nearest empty line in that direction."
//!
//! The originals only assert `selection.is_some()` and
//! `range.start < range.end`. The scenarios below additionally
//! pin the exact byte ranges, adding coverage the e2e tests
//! lacked.

use crate::common::scenario::buffer_scenario::{
    assert_buffer_scenario, BufferScenario, CursorExpect,
};
use fresh::test_api::Action;

#[test]
fn migrated_select_to_paragraph_down_from_first_line() {
    // Original: `test_select_to_paragraph_down`. Cursor at line 1
    // start; press Ctrl+Shift+Down; expect a non-empty forward
    // selection that ends at the empty line.
    //
    // Buffer:
    //   "paragraph 1 line 1\n"   bytes 0..19
    //   "paragraph 1 line 2\n"   bytes 19..38
    //   "\n"                     bytes 38..39  (empty line)
    //   "paragraph 2 line 1\n"   bytes 39..58
    //   "paragraph 2 line 2\n"   bytes 58..77
    assert_buffer_scenario(BufferScenario {
        description: "SelectToParagraphDown from line 1 selects to first empty line".into(),
        initial_text:
            "paragraph 1 line 1\nparagraph 1 line 2\n\nparagraph 2 line 1\nparagraph 2 line 2\n"
                .into(),
        actions: vec![Action::MoveLineStart, Action::SelectToParagraphDown],
        expected_text:
            "paragraph 1 line 1\nparagraph 1 line 2\n\nparagraph 2 line 1\nparagraph 2 line 2\n"
                .into(),
        // Anchor at 0; cursor at the empty-line position (38).
        expected_primary: CursorExpect::range(0, 38),
        ..Default::default()
    });
}

#[test]
fn migrated_select_to_paragraph_up_from_paragraph_2() {
    // Original: `test_select_to_paragraph_up`. Move down 3 times
    // (to line 4 = paragraph 2), then Ctrl+Shift+Up.
    assert_buffer_scenario(BufferScenario {
        description: "SelectToParagraphUp from paragraph 2 line 1 selects up to empty line".into(),
        initial_text:
            "paragraph 1 line 1\nparagraph 1 line 2\n\nparagraph 2 line 1\nparagraph 2 line 2\n"
                .into(),
        actions: vec![
            Action::MoveDown,
            Action::MoveDown,
            Action::MoveDown,
            Action::SelectToParagraphUp,
        ],
        expected_text:
            "paragraph 1 line 1\nparagraph 1 line 2\n\nparagraph 2 line 1\nparagraph 2 line 2\n"
                .into(),
        // Anchor at start of paragraph-2-line-1 (byte 39); cursor
        // moved up to the empty line (byte 38).
        expected_primary: CursorExpect::range(39, 38),
        ..Default::default()
    });
}

#[test]
fn migrated_multiple_select_to_paragraph_up_extends_selection() {
    // Original: `test_multiple_select_to_paragraph_up`.
    //
    // Buffer:
    //   "para 1\n"            bytes 0..7
    //   "\n"                  bytes 7..8   (empty)
    //   "para 2\n"            bytes 8..15
    //   "\n"                  bytes 15..16 (empty)
    //   "para 3\n"            bytes 16..23
    //   "para 3 continued\n"  bytes 23..40
    //
    // From end of buffer (byte 40), one SelectToParagraphUp lands
    // on the nearer empty line, a second extends further.
    assert_buffer_scenario(BufferScenario {
        description: "Multiple SelectToParagraphUp extends selection further upward".into(),
        initial_text: "para 1\n\npara 2\n\npara 3\npara 3 continued\n".into(),
        actions: vec![
            Action::MoveDocumentEnd,
            Action::SelectToParagraphUp,
            Action::SelectToParagraphUp,
        ],
        expected_text: "para 1\n\npara 2\n\npara 3\npara 3 continued\n".into(),
        // Anchor at end (40); two paragraph-up jumps land at the
        // first empty line (byte 7).
        expected_primary: CursorExpect::range(40, 7),
        ..Default::default()
    });
}
