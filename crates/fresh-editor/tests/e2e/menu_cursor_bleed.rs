//! E2E test for #1114: cursor position bleeds through dropdown menus.
//!
//! When the user clicks on text in the document (positioning the cursor) and
//! then opens a dropdown menu, the cursor (hardware or styled) should not be
//! visible through the menu overlay. The dropdown menu should fully occlude
//! the editor cursor.

use crate::common::harness::{layout, EditorTestHarness};
use crossterm::event::{KeyCode, KeyModifiers};

/// Test that the cursor cell styling does not bleed through an open dropdown menu.
///
/// Reproduction:
/// 1. Type enough text so the cursor is at a position that will be covered by a menu dropdown
/// 2. Click on the text to ensure cursor is placed there
/// 3. Open a dropdown menu (File menu) that covers the cursor position
/// 4. Verify: the cells in the dropdown area should contain menu content, not
///    cursor-styled editor content. The hardware cursor should not be positioned
///    within the dropdown's bounds.
#[test]
fn test_cursor_does_not_bleed_through_dropdown_menu() {
    // Use a tall harness so the File menu dropdown fits without clipping
    let mut harness = EditorTestHarness::new(80, 30).unwrap();

    // Type several lines of text so there's content under where the File menu will drop down.
    // The File menu dropdown starts at approximately row 1 (below menu bar) and covers rows 1..~12.
    // We need cursor to be on one of those rows.
    harness.type_text("Line one").unwrap();
    harness.send_key(KeyCode::Enter, KeyModifiers::NONE).unwrap();
    harness.type_text("Line two").unwrap();
    harness.send_key(KeyCode::Enter, KeyModifiers::NONE).unwrap();
    harness.type_text("Line three").unwrap();
    harness.send_key(KeyCode::Enter, KeyModifiers::NONE).unwrap();
    harness.type_text("Line four").unwrap();
    harness.send_key(KeyCode::Enter, KeyModifiers::NONE).unwrap();
    harness.type_text("Line five").unwrap();

    // Click on "Line two" to position the cursor there.
    // Row 0 = menu bar, row 1 = tab bar, row 2 = content line 1, row 3 = content line 2
    // The line number gutter is about 7 chars wide ("   1 │ "), so col 10 is in the text.
    let cursor_row = layout::CONTENT_START_ROW as u16 + 1; // line 2 in content area
    let cursor_col = 10;
    harness.mouse_click(cursor_col, cursor_row).unwrap();

    // Verify cursor is now on line 2
    let status = harness.get_status_bar();
    assert!(
        status.contains("Ln 2"),
        "Expected cursor on line 2, got status: {}",
        status
    );

    // Record the cursor's screen position before opening the menu
    let (cursor_x, cursor_y) = harness.screen_cursor_position();
    eprintln!(
        "Cursor screen position before menu: ({}, {})",
        cursor_x, cursor_y
    );

    // Now open the File menu (Alt+F)
    harness
        .send_key(KeyCode::Char('f'), KeyModifiers::ALT)
        .unwrap();
    harness.render().unwrap();

    // Verify the menu is open
    harness.assert_screen_contains("New File");
    harness.assert_screen_contains("Save");

    let screen = harness.screen_to_string();
    eprintln!("Screen with menu open:\n{}", screen);

    // Find the menu dropdown bounds by locating menu items on screen
    let menu_rows: Vec<u16> = (0..harness.terminal_height() as u16)
        .filter(|&row| {
            let row_text = harness.get_row_text(row);
            // Menu items have specific content
            row_text.contains("New File")
                || row_text.contains("Open")
                || row_text.contains("Save")
                || row_text.contains("Close")
                || row_text.contains("Quit")
        })
        .collect();

    assert!(
        !menu_rows.is_empty(),
        "Should find menu item rows. Screen:\n{}",
        screen
    );
    let menu_top = *menu_rows.first().unwrap();
    let menu_bottom = *menu_rows.last().unwrap();
    eprintln!("Menu dropdown spans rows {} to {}", menu_top, menu_bottom);

    // The cursor was at (cursor_x, cursor_y). Check if the cursor's row
    // is within the dropdown area.
    if cursor_y >= menu_top && cursor_y <= menu_bottom {
        // The cursor's original position IS within the menu dropdown area.
        // The hardware cursor should NOT be positioned there when the menu is open.
        let (new_cursor_x, new_cursor_y) = harness.screen_cursor_position();
        eprintln!(
            "Cursor screen position with menu open: ({}, {})",
            new_cursor_x, new_cursor_y
        );

        // The hardware cursor should NOT be within the menu dropdown area
        let cursor_in_menu = new_cursor_y >= menu_top && new_cursor_y <= menu_bottom;
        assert!(
            !cursor_in_menu,
            "Bug #1114: Hardware cursor at ({}, {}) bleeds through the dropdown menu \
             (menu spans rows {}..{}). The cursor should be hidden when a menu is open. Screen:\n{}",
            new_cursor_x, new_cursor_y, menu_top, menu_bottom, screen
        );
    }

    // Additionally, verify that cells at the original cursor position now contain
    // menu content (not cursor-styled editor text). The row at cursor_y should
    // contain menu item text if the dropdown covers it.
    if cursor_y >= menu_top && cursor_y <= menu_bottom {
        let row_text = harness.get_row_text(cursor_y);
        // The row should contain menu content, not just editor text
        let has_menu_content = row_text.contains("New")
            || row_text.contains("Open")
            || row_text.contains("Save")
            || row_text.contains("Close")
            || row_text.contains("Quit")
            || row_text.contains("Revert");
        assert!(
            has_menu_content,
            "Row {} (where cursor was) should contain menu content, but got: '{}'",
            cursor_y, row_text
        );
    }
}
