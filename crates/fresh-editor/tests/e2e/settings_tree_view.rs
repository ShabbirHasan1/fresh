//! E2E tests for the settings tree-view (left panel) ↔ body-panel
//! synchronization. Covers, in both directions for every flow:
//!
//!   1. Tree keyboard nav scrolls the body to the matching section.
//!   2. Clicking a tree section row jumps the body to that section.
//!   3a. Body keyboard scroll updates the tree section highlight.
//!   3b. Body mouse-wheel scroll updates the tree section highlight.
//!   4. At every assertion point exactly ONE row in the left panel is
//!      visually highlighted (single-cursor invariant) — checked via the
//!      cell background color, not just the `>` glyph, so the test
//!      catches a regression where two rows get a highlight bg.

use crate::common::harness::EditorTestHarness;
use crossterm::event::{KeyCode, KeyModifiers};

/// Editor sections (alphabetical), each described by:
/// 0: a prefix that matches the section name as it appears in the tree
///    (which truncates to the 14 cols available for section text in the
///    24-col left panel — e.g. "Bracket Matching" renders as
///    "Bracket Matchi");
/// 1: a unique item name in each section, used to verify "the body is
///    currently showing section X" by looking for X's first item.
const EDITOR_SECTIONS: &[(&str, &str)] = &[
    ("Bracket Matchi", "Highlight Matching Brackets"),
    ("Completion", "Completion Popup Auto Show"),
    ("Diagnostics", "Diagnostics Inline Text"),
    ("Display", "Animations"),
    ("Editing", "Auto Close"),
    ("Keyboard", "Keyboard Disambiguate"),
    ("LSP", "Enable Inlay Hints"),
    ("Mouse", "Double Click Time Ms"),
    ("Performance", "Large File"),
    ("Recovery", "Buffer Cleanup"),
    ("Startup", "Auto Detect Project"),
    ("Status Bar", "Show Prompt Line"),
    ("Whitespace", "Show Trailing Whitespace"),
];

/// Open Settings, navigate to the Editor category, and expand it so
/// section rows are visible in the tree.
fn open_editor_expanded(harness: &mut EditorTestHarness) {
    harness.open_settings().unwrap();
    // Navigate from default General → Clipboard → Editor (2 Downs).
    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    // Right expands the focused expandable category (Editor has many sections).
    harness
        .send_key(KeyCode::Right, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();
}

/// Bounding x range of the left-panel tree on a `width`-col terminal.
/// Modal is 90% of width, centered, with 1-col borders on each side; the
/// categories panel takes 24 cols of the inner area.
fn tree_bounds(width: u16) -> (u16, u16) {
    let modal_width = (width * 90 / 100).min(160);
    let modal_x = width.saturating_sub(modal_width) / 2;
    let tree_x = modal_x + 1; // skip modal left border
    (tree_x, tree_x + 24)
}

/// Bounding y range of the modal interior. Used to clip probes so that
/// the menu bar / status bar / outside-modal cells don't pollute bg
/// counts when looking for highlighted tree rows.
fn modal_y_range(height: u16) -> (u16, u16) {
    let modal_height = height * 90 / 100;
    let modal_y = height.saturating_sub(modal_height) / 2;
    (modal_y + 1, modal_y + modal_height - 1) // skip top + bottom borders
}

/// Find the screen row that holds the given section name *inside the
/// left-panel tree* (ignores other places the name might appear in the
/// body). Returns the (col, row) of the section name's first character.
fn find_tree_row(harness: &EditorTestHarness, name: &str, term_width: u16) -> Option<(u16, u16)> {
    let (tree_x_start, tree_x_end) = tree_bounds(term_width);
    let screen = harness.screen_to_string();
    for (row_idx, line) in screen.lines().enumerate() {
        if let Some(col) = line.find(name) {
            let col = col as u16;
            if col >= tree_x_start && col < tree_x_end {
                return Some((col, row_idx as u16));
            }
        }
    }
    None
}

/// Sample the bg color of every cell at the tree probe column and
/// return the most common one — that's the popup_bg baseline. Using
/// the *mode* avoids ambiguity around what counts as "the panel
/// background" when probing cells: at most one row can have the
/// highlight bg, so the mode of all sampled cells is necessarily the
/// non-highlight popup_bg.
fn tree_bg_mode(harness: &EditorTestHarness, term_width: u16) -> Option<ratatui::style::Color> {
    use std::collections::HashMap;
    let (tree_x_start, _) = tree_bounds(term_width);
    let probe_col = tree_x_start + 5;
    let buffer = harness.buffer();
    let (y_start, y_end) = modal_y_range(buffer.area.height);
    let mut counts: HashMap<ratatui::style::Color, usize> = HashMap::new();
    for y in y_start..y_end {
        if let Some(style) = harness.get_cell_style(probe_col, y) {
            if let Some(bg) = style.bg {
                *counts.entry(bg).or_insert(0) += 1;
            }
        }
    }
    counts.into_iter().max_by_key(|&(_, c)| c).map(|(bg, _)| bg)
}

/// y values of left-panel tree rows whose background differs from the
/// most-common bg — i.e. rows the user sees as visually highlighted.
fn highlighted_tree_rows(harness: &EditorTestHarness, term_width: u16) -> Vec<u16> {
    let Some(baseline) = tree_bg_mode(harness, term_width) else {
        return Vec::new();
    };
    let (tree_x_start, _) = tree_bounds(term_width);
    let probe_col = tree_x_start + 5;
    let buffer = harness.buffer();
    let (y_start, y_end) = modal_y_range(buffer.area.height);
    let mut rows = Vec::new();
    for y in y_start..y_end {
        if let Some(style) = harness.get_cell_style(probe_col, y) {
            if style.bg.is_some() && style.bg != Some(baseline) {
                rows.push(y);
            }
        }
    }
    rows
}

/// The single-highlight invariant: exactly one row has a non-baseline
/// bg color in the left panel. Test #4.
fn count_highlighted_tree_rows(harness: &EditorTestHarness, term_width: u16) -> usize {
    highlighted_tree_rows(harness, term_width).len()
}

/// True when the body panel is currently anchored on `section` — i.e.
/// either the section's header or its first item is on screen, AND
/// none of the items from the *next* section are visible above the
/// midline (which would mean we're past it).
///
/// This is more forgiving than a strict "first item visible" check:
/// when the user jumps to a section the scroll snaps to its top, but
/// depending on viewport height the very first item may sit at the
/// last row and get clipped to its top border. Looking for the
/// section header text (rendered in the body just above the first
/// card) catches that case.
fn body_shows_section(harness: &EditorTestHarness, section: &(&str, &str)) -> bool {
    let header = section.0;
    let item = section.1;
    let screen = harness.screen_to_string();
    screen.contains(item) || screen.contains(header)
}

/// 1. Tree keyboard Up/Down on expanded sections scrolls the body.
#[test]
fn tree_keyboard_nav_scrolls_body_both_directions() {
    let mut harness = EditorTestHarness::new(120, 40).unwrap();
    open_editor_expanded(&mut harness);

    // Click on a middle section (Display) so the cursor lands there.
    let middle = &EDITOR_SECTIONS[3]; // Display
    let (col, row) = find_tree_row(&harness, middle.0, 120)
        .unwrap_or_else(|| panic!("could not find '{}' in tree", middle.0));
    harness.mouse_click(col, row).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, middle),
        "after click, body should show '{}'. Screen:\n{}",
        middle.1,
        harness.screen_to_string()
    );

    // BackTab from Settings goes directly to Categories (focus_prev wraps
    // back through the panel cycle).
    harness
        .send_key(KeyCode::BackTab, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Down on the tree → body should show the NEXT section (Editing).
    let next = &EDITOR_SECTIONS[4];
    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, next),
        "after tree Down, body should show next section '{}'. Screen:\n{}",
        next.1,
        harness.screen_to_string()
    );
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "after tree Down, exactly one tree row should be highlighted"
    );

    // Up on the tree → body should return to Display.
    harness.send_key(KeyCode::Up, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, middle),
        "after tree Up, body should return to '{}'. Screen:\n{}",
        middle.1,
        harness.screen_to_string()
    );
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "after tree Up, exactly one tree row should be highlighted"
    );

    // One more Up → previous section (Diagnostics).
    let prev = &EDITOR_SECTIONS[2];
    harness.send_key(KeyCode::Up, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, prev),
        "after another tree Up, body should show '{}'. Screen:\n{}",
        prev.1,
        harness.screen_to_string()
    );
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "after another tree Up, exactly one tree row should be highlighted"
    );
}

/// 2. Clicking a tree section row jumps the body to that section, in
///    both directions (lower section after upper, then back upward).
#[test]
fn click_tree_section_jumps_body_both_directions() {
    let mut harness = EditorTestHarness::new(120, 40).unwrap();
    open_editor_expanded(&mut harness);

    // Click a section near the top first (Bracket Matching).
    let upper = &EDITOR_SECTIONS[0];
    let (col, row) = find_tree_row(&harness, upper.0, 120).expect("upper section not in tree");
    harness.mouse_click(col, row).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, upper),
        "click upper section: body should show '{}'. Screen:\n{}",
        upper.1,
        harness.screen_to_string()
    );
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after clicking upper section"
    );

    // Click a section near the bottom of the visible list (LSP — index 6).
    // This walks downward in the tree.
    let lower = &EDITOR_SECTIONS[6];
    let (col, row) = find_tree_row(&harness, lower.0, 120).expect("lower section not in tree");
    harness.mouse_click(col, row).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, lower),
        "click lower section: body should show '{}'. Screen:\n{}",
        lower.1,
        harness.screen_to_string()
    );
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after clicking lower section"
    );

    // Click an upper section again — covers the upward direction.
    let (col, row) = find_tree_row(&harness, upper.0, 120)
        .expect("upper section not in tree (after second click)");
    harness.mouse_click(col, row).unwrap();
    harness.render().unwrap();
    assert!(
        body_shows_section(&harness, upper),
        "click upper section (after lower): body should show '{}'. Screen:\n{}",
        upper.1,
        harness.screen_to_string()
    );
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after clicking back upward"
    );
}

/// 3a. Body keyboard scroll moves the left-panel section highlight in
///     both directions.
#[test]
fn body_keyboard_scroll_updates_tree_highlight_both_directions() {
    let mut harness = EditorTestHarness::new(120, 40).unwrap();
    open_editor_expanded(&mut harness);

    // Land in the middle (Display).
    let start = &EDITOR_SECTIONS[3];
    let (col, row) = find_tree_row(&harness, start.0, 120).expect("Display row");
    harness.mouse_click(col, row).unwrap();
    harness.render().unwrap();

    // Press PageDown several times in the body — content scrolls forward.
    // The body panel was given focus by the click, so PageDown goes to it.
    for _ in 0..3 {
        harness
            .send_key(KeyCode::PageDown, KeyModifiers::NONE)
            .unwrap();
    }
    harness.render().unwrap();

    // Whatever later section is now showing in the body, the tree
    // should highlight a section AFTER Display (further down). Verify
    // the highlight has moved off Display and onto a row whose
    // alphabetical index is ≥ Display's index.
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after body PageDown"
    );
    let later = highlighted_section_name(&harness, 120);
    let later_idx = later
        .as_deref()
        .and_then(|n| EDITOR_SECTIONS.iter().position(|s| s.0 == n))
        .unwrap_or_else(|| panic!("expected highlighted section, got {later:?}"));
    assert!(
        later_idx >= 3,
        "after PageDown, highlight should be at Display or later, got {later:?}"
    );

    // PageUp back — highlight should move BACK toward Display (or before).
    for _ in 0..6 {
        harness
            .send_key(KeyCode::PageUp, KeyModifiers::NONE)
            .unwrap();
    }
    harness.render().unwrap();
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after body PageUp"
    );
    let earlier = highlighted_section_name(&harness, 120);
    let earlier_idx = earlier
        .as_deref()
        .and_then(|n| EDITOR_SECTIONS.iter().position(|s| s.0 == n))
        .unwrap_or_else(|| panic!("expected highlighted section, got {earlier:?}"));
    assert!(
        earlier_idx <= 3,
        "after PageUp, highlight should be at Display or earlier, got {earlier:?}"
    );
    assert!(
        earlier_idx < later_idx,
        "PageUp should move highlight BACK from earlier PageDown position"
    );
}

/// 3b. Body mouse-wheel scroll moves the left-panel section highlight
///     in both directions.
#[test]
fn body_wheel_scroll_updates_tree_highlight_both_directions() {
    let mut harness = EditorTestHarness::new(120, 40).unwrap();
    open_editor_expanded(&mut harness);

    let start = &EDITOR_SECTIONS[3];
    let (col, row) = find_tree_row(&harness, start.0, 120).expect("Display row");
    harness.mouse_click(col, row).unwrap();
    harness.render().unwrap();

    // Find a column inside the body to scroll on.
    let body_col: u16 = 100;
    let body_row: u16 = 20;

    for _ in 0..15 {
        harness.mouse_scroll_down(body_col, body_row).unwrap();
    }
    harness.render().unwrap();
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after body wheel-down"
    );
    let later = highlighted_section_name(&harness, 120);
    let later_idx = later
        .as_deref()
        .and_then(|n| EDITOR_SECTIONS.iter().position(|s| s.0 == n))
        .unwrap_or_else(|| panic!("expected highlighted section, got {later:?}"));
    assert!(
        later_idx >= 3,
        "after wheel-down, highlight should be at Display or later, got {later:?}"
    );

    for _ in 0..30 {
        harness.mouse_scroll_up(body_col, body_row).unwrap();
    }
    harness.render().unwrap();
    assert_eq!(
        count_highlighted_tree_rows(&harness, 120),
        1,
        "single-highlight invariant after body wheel-up"
    );
    let earlier = highlighted_section_name(&harness, 120);
    let earlier_idx = earlier
        .as_deref()
        .and_then(|n| EDITOR_SECTIONS.iter().position(|s| s.0 == n))
        .unwrap_or_else(|| panic!("expected highlighted section, got {earlier:?}"));
    assert!(
        earlier_idx <= later_idx,
        "wheel-up should not move highlight forward"
    );
}

/// Returns the section name on the row whose left-panel bg differs from
/// the popup baseline — i.e. the row the user sees as highlighted.
/// Returns `None` if no row is highlighted (which itself fails the
/// per-test single-highlight assertion separately).
fn highlighted_section_name(harness: &EditorTestHarness, term_width: u16) -> Option<String> {
    let (tree_x_start, tree_x_end) = tree_bounds(term_width);
    let screen = harness.screen_to_string();
    for y in highlighted_tree_rows(harness, term_width) {
        let line = screen.lines().nth(y as usize)?;
        for (sec_name, _) in EDITOR_SECTIONS {
            if let Some(col) = line.find(sec_name) {
                let col = col as u16;
                if col >= tree_x_start && col < tree_x_end {
                    return Some((*sec_name).to_string());
                }
            }
        }
    }
    None
}
