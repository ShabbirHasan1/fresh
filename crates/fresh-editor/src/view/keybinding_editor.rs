//! Keybinding Editor rendering and input handling
//!
//! Renders the keybinding editor modal and handles input events.

use crate::app::keybinding_editor::{
    BindingSource, ContextFilter, EditMode, KeybindingEditor, SearchMode, SourceFilter,
};
use crate::input::keybindings::{format_keybinding, KeybindingResolver};
use crate::view::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Render the keybinding editor modal
pub fn render_keybinding_editor(
    frame: &mut Frame,
    area: Rect,
    editor: &KeybindingEditor,
    theme: &Theme,
) {
    // Modal dimensions: 90% width, 90% height
    let modal_width = (area.width as f32 * 0.90).min(120.0) as u16;
    let modal_height = (area.height as f32 * 0.90) as u16;
    let modal_width = modal_width.max(60).min(area.width.saturating_sub(2));
    let modal_height = modal_height.max(20).min(area.height.saturating_sub(2));

    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x,
        y,
        width: modal_width,
        height: modal_height,
    };

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Border
    let title = format!(" Keybinding Editor \u{2500} [{}] ", editor.active_keymap);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border_fg))
        .style(Style::default().bg(theme.popup_bg).fg(theme.editor_fg));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Layout: header (3-4 lines) | table | footer (1 line)
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header: config path + search + filters
        Constraint::Min(5),    // Table
        Constraint::Length(1), // Footer hints
    ])
    .split(inner);

    render_header(frame, chunks[0], editor, theme);
    render_table(frame, chunks[1], editor, theme);
    render_footer(frame, chunks[2], editor, theme);

    // Render dialogs on top
    if editor.showing_help {
        render_help_overlay(frame, inner, theme);
    }

    if let Some(ref dialog) = editor.edit_dialog {
        render_edit_dialog(frame, inner, dialog, editor, theme);
    }

    if editor.showing_confirm_dialog {
        render_confirm_dialog(frame, inner, editor, theme);
    }
}

/// Render the header section (config path, search, filters)
fn render_header(frame: &mut Frame, area: Rect, editor: &KeybindingEditor, theme: &Theme) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Config path + keymap info
        Constraint::Length(1), // Search bar
        Constraint::Length(1), // Filters
    ])
    .split(area);

    // Line 1: Config file path and keymap names
    let mut path_spans = vec![
        Span::styled(" Config: ", Style::default().fg(theme.line_number_fg)),
        Span::styled(
            &editor.config_file_path,
            Style::default().fg(theme.diagnostic_info_fg),
        ),
    ];
    if !editor.keymap_names.is_empty() {
        path_spans.push(Span::styled(
            "  Maps: ",
            Style::default().fg(theme.line_number_fg),
        ));
        path_spans.push(Span::styled(
            editor.keymap_names.join(", "),
            Style::default().fg(theme.editor_fg),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(path_spans)), chunks[0]);

    // Line 2: Search bar
    if editor.search_active {
        let search_spans = match editor.search_mode {
            SearchMode::Text => {
                vec![
                    Span::styled(
                        " Search: ",
                        Style::default()
                            .fg(theme.help_key_fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&editor.search_query, Style::default().fg(theme.editor_fg)),
                    Span::styled("_", Style::default().fg(theme.cursor)),
                    Span::styled(
                        "  (Esc to cancel, Tab to switch to Record Key)",
                        Style::default().fg(theme.line_number_fg),
                    ),
                ]
            }
            SearchMode::RecordKey => {
                let key_text = if editor.search_key_display.is_empty() {
                    "Press a key...".to_string()
                } else {
                    editor.search_key_display.clone()
                };
                vec![
                    Span::styled(
                        " Record Key: ",
                        Style::default()
                            .fg(theme.diagnostic_warning_fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(key_text, Style::default().fg(theme.editor_fg)),
                    Span::styled(
                        "  (Esc to cancel, Tab to switch to Text Search)",
                        Style::default().fg(theme.line_number_fg),
                    ),
                ]
            }
        };
        frame.render_widget(Paragraph::new(Line::from(search_spans)), chunks[1]);
    } else {
        let hint = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                "Press / to search, r to record key search",
                Style::default().fg(theme.line_number_fg),
            ),
        ]);
        frame.render_widget(Paragraph::new(hint), chunks[1]);
    }

    // Line 3: Filters and counts
    let total = editor.bindings.len();
    let filtered = editor.filtered_indices.len();
    let count_str = if filtered == total {
        format!("{} bindings", total)
    } else {
        format!("{}/{} shown", filtered, total)
    };

    let filter_spans = vec![
        Span::styled(" Context: ", Style::default().fg(theme.line_number_fg)),
        Span::styled(
            format!("[{}]", editor.context_filter_display()),
            Style::default().fg(if editor.context_filter == ContextFilter::All {
                theme.editor_fg
            } else {
                theme.diagnostic_info_fg
            }),
        ),
        Span::styled("  Source: ", Style::default().fg(theme.line_number_fg)),
        Span::styled(
            format!("[{}]", editor.source_filter_display()),
            Style::default().fg(if editor.source_filter == SourceFilter::All {
                theme.editor_fg
            } else {
                theme.diagnostic_info_fg
            }),
        ),
        Span::styled(
            format!("  {}", count_str),
            Style::default().fg(theme.line_number_fg),
        ),
        Span::styled(
            if editor.has_changes {
                "  [modified]"
            } else {
                ""
            },
            Style::default().fg(theme.diagnostic_warning_fg),
        ),
    ];
    frame.render_widget(Paragraph::new(Line::from(filter_spans)), chunks[2]);
}

/// Render the keybinding table
fn render_table(frame: &mut Frame, area: Rect, editor: &KeybindingEditor, theme: &Theme) {
    if area.height < 2 {
        return;
    }

    let inner_width = area.width.saturating_sub(2); // Leave room for scrollbar

    // Column widths (adaptive): Key | Action Name | Description | Context | Source
    let key_col_width = (inner_width as f32 * 0.16).min(20.0) as u16;
    let action_name_col_width = (inner_width as f32 * 0.22).min(28.0) as u16;
    let context_col_width = 14u16;
    let source_col_width = 8u16;
    let fixed_cols =
        key_col_width + action_name_col_width + context_col_width + source_col_width + 5; // +5 for spacers
    let description_col_width = inner_width.saturating_sub(fixed_cols);

    // Header line
    let header = Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(
            pad_right("Key", key_col_width as usize),
            Style::default()
                .fg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            pad_right("Action", action_name_col_width as usize),
            Style::default()
                .fg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            pad_right("Description", description_col_width as usize),
            Style::default()
                .fg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            pad_right("Context", context_col_width as usize),
            Style::default()
                .fg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(
            pad_right("Source", source_col_width as usize),
            Style::default()
                .fg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), Rect { height: 1, ..area });

    // Separator
    if area.height > 1 {
        let sep = "\u{2500}".repeat(inner_width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {}", sep),
                Style::default().fg(theme.line_number_fg),
            ))),
            Rect {
                y: area.y + 1,
                height: 1,
                ..area
            },
        );
    }

    // Table rows
    let table_area = Rect {
        y: area.y + 2,
        height: area.height.saturating_sub(2),
        ..area
    };

    let visible_rows = table_area.height as usize;

    for (display_idx, &binding_idx) in editor
        .filtered_indices
        .iter()
        .skip(editor.scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let row_y = table_area.y + display_idx as u16;
        if row_y >= table_area.y + table_area.height {
            break;
        }

        let binding = &editor.bindings[binding_idx];
        let is_selected = editor.scroll_offset + display_idx == editor.selected;

        let (row_bg, row_fg) = if is_selected {
            (theme.selection_bg, theme.editor_fg)
        } else {
            (theme.popup_bg, theme.editor_fg)
        };

        let key_style = Style::default()
            .fg(if is_selected {
                theme.editor_fg
            } else {
                theme.help_key_fg
            })
            .bg(row_bg);
        let action_name_style = Style::default()
            .fg(if is_selected {
                theme.editor_fg
            } else {
                theme.diagnostic_info_fg
            })
            .bg(row_bg);
        let action_style = Style::default().fg(row_fg).bg(row_bg);
        let context_style = Style::default()
            .fg(if is_selected {
                row_fg
            } else {
                theme.line_number_fg
            })
            .bg(row_bg);
        let source_style = Style::default()
            .fg(if binding.source == BindingSource::Custom {
                if is_selected {
                    theme.editor_fg
                } else {
                    theme.diagnostic_info_fg
                }
            } else {
                context_style.fg.unwrap_or(theme.line_number_fg)
            })
            .bg(row_bg);

        let indicator = if is_selected { ">" } else { " " };

        let row = Line::from(vec![
            Span::styled(indicator, Style::default().fg(theme.help_key_fg).bg(row_bg)),
            Span::styled(
                pad_right(&binding.key_display, key_col_width as usize),
                key_style,
            ),
            Span::styled(" ", action_name_style),
            Span::styled(
                pad_right(&binding.action, action_name_col_width as usize),
                action_name_style,
            ),
            Span::styled(" ", action_style),
            Span::styled(
                pad_right(&binding.action_display, description_col_width as usize),
                action_style,
            ),
            Span::styled(" ", context_style),
            Span::styled(
                pad_right(&binding.context, context_col_width as usize),
                context_style,
            ),
            Span::styled(" ", source_style),
            Span::styled(
                pad_right(
                    if binding.source == BindingSource::Custom {
                        "custom"
                    } else {
                        "keymap"
                    },
                    source_col_width as usize,
                ),
                source_style,
            ),
        ]);

        let row_area = Rect {
            y: row_y,
            height: 1,
            ..table_area
        };
        // Fill the row background
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(row_bg)),
            row_area,
        );
        frame.render_widget(Paragraph::new(row), row_area);
    }

    // Scrollbar
    if editor.filtered_indices.len() > visible_rows {
        let mut scrollbar_state =
            ScrollbarState::new(editor.filtered_indices.len()).position(editor.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        frame.render_stateful_widget(scrollbar, table_area, &mut scrollbar_state);
    }
}

/// Render the footer with key hints
fn render_footer(frame: &mut Frame, area: Rect, editor: &KeybindingEditor, theme: &Theme) {
    let hints = if editor.search_active {
        vec![
            Span::styled(" Esc", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Cancel  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("Tab", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Toggle Mode  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("Enter", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Confirm", Style::default().fg(theme.line_number_fg)),
        ]
    } else {
        vec![
            Span::styled(" Enter", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Edit  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("a", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Add  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("d", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Delete  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("/", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Search  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("r", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Record Key  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("c", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Context  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("s", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Source  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("?", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Help  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("Ctrl+S", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Save  ", Style::default().fg(theme.line_number_fg)),
            Span::styled("Esc", Style::default().fg(theme.help_key_fg)),
            Span::styled(":Close", Style::default().fg(theme.line_number_fg)),
        ]
    };

    frame.render_widget(Paragraph::new(Line::from(hints)), area);
}

/// Render the help overlay
fn render_help_overlay(frame: &mut Frame, area: Rect, theme: &Theme) {
    let width = 52u16.min(area.width.saturating_sub(4));
    let height = 22u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let dialog_area = Rect {
        x,
        y,
        width,
        height,
    };
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border_fg))
        .style(Style::default().bg(theme.popup_bg).fg(theme.editor_fg));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let help_lines = vec![
        help_line("Navigation", "", theme, true),
        help_line("  \u{2191} / \u{2193}", "Move up/down", theme, false),
        help_line("  PgUp / PgDn", "Page up/down", theme, false),
        help_line("  Home / End", "First/last binding", theme, false),
        help_line("", "", theme, false),
        help_line("Search", "", theme, true),
        help_line("  /", "Search by name", theme, false),
        help_line("  r", "Search by recording key", theme, false),
        help_line("  Tab", "Toggle search mode", theme, false),
        help_line("  Esc", "Cancel search", theme, false),
        help_line("", "", theme, false),
        help_line("Editing", "", theme, true),
        help_line("  Enter", "Edit selected binding", theme, false),
        help_line("  a", "Add new binding", theme, false),
        help_line("  d / Delete", "Delete custom binding", theme, false),
        help_line("", "", theme, false),
        help_line("Filters", "", theme, true),
        help_line("  c", "Cycle context filter", theme, false),
        help_line("  s", "Cycle source filter", theme, false),
        help_line("", "", theme, false),
        help_line("  Ctrl+S", "Save changes", theme, false),
        help_line("  Esc / ?", "Close help / Toggle help", theme, false),
    ];

    let para = Paragraph::new(help_lines);
    frame.render_widget(para, inner);
}

fn help_line<'a>(key: &'a str, desc: &'a str, theme: &Theme, is_header: bool) -> Line<'a> {
    if is_header {
        Line::from(vec![Span::styled(
            key,
            Style::default()
                .fg(theme.editor_fg)
                .add_modifier(Modifier::BOLD),
        )])
    } else {
        Line::from(vec![
            Span::styled(
                format!("{:16}", key),
                Style::default()
                    .fg(theme.help_key_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc, Style::default().fg(theme.editor_fg)),
        ])
    }
}

/// Maximum number of autocomplete suggestions to display
const MAX_AUTOCOMPLETE_VISIBLE: usize = 8;

/// Render the edit/add binding dialog
fn render_edit_dialog(
    frame: &mut Frame,
    area: Rect,
    dialog: &crate::app::keybinding_editor::EditBindingState,
    _editor: &KeybindingEditor,
    theme: &Theme,
) {
    let width = 56u16.min(area.width.saturating_sub(4));
    let height = 18u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let dialog_area = Rect {
        x,
        y,
        width,
        height,
    };
    frame.render_widget(Clear, dialog_area);

    let title = if dialog.editing_index.is_some() {
        " Edit Keybinding "
    } else {
        " Add Keybinding "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border_fg))
        .style(Style::default().bg(theme.popup_bg).fg(theme.editor_fg));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Instructions
        Constraint::Length(1), // Spacer
        Constraint::Length(1), // Key field
        Constraint::Length(1), // Action field
        Constraint::Length(1), // Action description (read-only)
        Constraint::Length(1), // Context field
        Constraint::Length(1), // Spacer
        Constraint::Min(3),    // Conflicts / error
        Constraint::Length(1), // Buttons
    ])
    .split(inner);

    // Instructions
    let instr = match dialog.mode {
        EditMode::RecordingKey => "Press the desired key combination...",
        EditMode::EditingAction => "Type action name (Tab/Enter to accept)",
        EditMode::EditingContext => "Select context...",
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {}", instr),
            Style::default().fg(theme.line_number_fg),
        ))),
        chunks[0],
    );

    // Key field
    let key_focused = dialog.focus_area == 0;
    let key_text = if dialog.key_display.is_empty() {
        if dialog.mode == EditMode::RecordingKey {
            "..."
        } else {
            "(none)"
        }
    } else {
        &dialog.key_display
    };
    let key_label_style = if key_focused {
        Style::default()
            .fg(theme.help_key_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.line_number_fg)
    };
    let key_value_style = if key_focused {
        Style::default()
            .fg(theme.editor_fg)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(theme.editor_fg)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("   Key:     ", key_label_style),
            Span::styled(key_text, key_value_style),
        ])),
        chunks[2],
    );

    // Action field
    let action_focused = dialog.focus_area == 1;
    let action_label_style = if action_focused {
        Style::default()
            .fg(theme.help_key_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.line_number_fg)
    };
    let has_error = dialog.action_error.is_some();
    let action_value_style = if has_error {
        Style::default().fg(theme.diagnostic_error_fg)
    } else {
        Style::default().fg(theme.editor_fg)
    };
    let action_display = if dialog.action_text.is_empty() && dialog.mode != EditMode::EditingAction
    {
        "(type action name)"
    } else {
        &dialog.action_text
    };
    let mut action_spans = vec![
        Span::styled("   Action:  ", action_label_style),
        Span::styled(action_display, action_value_style),
    ];
    if action_focused && dialog.mode == EditMode::EditingAction {
        action_spans.push(Span::styled("_", Style::default().fg(theme.cursor)));
    }
    frame.render_widget(Paragraph::new(Line::from(action_spans)), chunks[3]);

    // Action description (read-only, shown when action text is a valid action)
    if !dialog.action_text.is_empty() {
        let description = KeybindingResolver::format_action_from_str(&dialog.action_text);
        // Only show if description differs from the raw action name
        if description.to_lowercase() != dialog.action_text.replace('_', " ").to_lowercase() {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("            ", Style::default().fg(theme.line_number_fg)),
                    Span::styled(
                        format!("\u{2192} {}", description),
                        Style::default()
                            .fg(theme.line_number_fg)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])),
                chunks[4],
            );
        }
    }

    // Context field
    let ctx_focused = dialog.focus_area == 2;
    let ctx_label_style = if ctx_focused {
        Style::default()
            .fg(theme.help_key_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.line_number_fg)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("   Context: ", ctx_label_style),
            Span::styled(
                format!("[{}]", dialog.context),
                Style::default().fg(theme.editor_fg),
            ),
            if ctx_focused {
                Span::styled(
                    "  \u{2190}/\u{2192} to change",
                    Style::default().fg(theme.line_number_fg),
                )
            } else {
                Span::raw("")
            },
        ])),
        chunks[5],
    );

    // Conflicts or error in the info area
    let mut info_lines: Vec<Line> = Vec::new();
    if let Some(ref err) = dialog.action_error {
        info_lines.push(Line::from(Span::styled(
            format!("   \u{2717} {}", err),
            Style::default()
                .fg(theme.diagnostic_error_fg)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if !dialog.conflicts.is_empty() {
        info_lines.push(Line::from(Span::styled(
            "   \u{26a0} Conflicts:",
            Style::default()
                .fg(theme.diagnostic_warning_fg)
                .add_modifier(Modifier::BOLD),
        )));
        for conflict in &dialog.conflicts {
            info_lines.push(Line::from(Span::styled(
                format!("     {}", conflict),
                Style::default().fg(theme.diagnostic_warning_fg),
            )));
        }
    }
    if !info_lines.is_empty() {
        frame.render_widget(Paragraph::new(info_lines), chunks[7]);
    }

    // Buttons
    let btn_focused = dialog.focus_area == 3;
    let save_style = if btn_focused && dialog.selected_button == 0 {
        Style::default()
            .fg(theme.popup_bg)
            .bg(theme.help_key_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.editor_fg)
    };
    let cancel_style = if btn_focused && dialog.selected_button == 1 {
        Style::default()
            .fg(theme.popup_bg)
            .bg(theme.help_key_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.editor_fg)
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("   "),
            Span::styled(" Save ", save_style),
            Span::raw("  "),
            Span::styled(" Cancel ", cancel_style),
        ])),
        chunks[8],
    );

    // Render autocomplete popup on top of everything if visible
    if dialog.autocomplete_visible && !dialog.autocomplete_suggestions.is_empty() {
        render_autocomplete_popup(frame, chunks[3], dialog, theme);
    }
}

/// Render the autocomplete suggestions popup below the action field
fn render_autocomplete_popup(
    frame: &mut Frame,
    action_field_area: Rect,
    dialog: &crate::app::keybinding_editor::EditBindingState,
    theme: &Theme,
) {
    let suggestion_count = dialog
        .autocomplete_suggestions
        .len()
        .min(MAX_AUTOCOMPLETE_VISIBLE);
    if suggestion_count == 0 {
        return;
    }

    // Position: below the action field, offset to align with text
    let popup_x = action_field_area.x + 12; // offset past "   Action:  "
    let popup_y = action_field_area.y + 1;
    let popup_width = 36u16.min(action_field_area.width.saturating_sub(12));
    let popup_height = (suggestion_count as u16) + 2; // +2 for border

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border_fg))
        .style(Style::default().bg(theme.popup_bg).fg(theme.editor_fg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Determine scroll offset for autocomplete list
    let selected = dialog.autocomplete_selected.unwrap_or(0);
    let scroll_offset = if selected >= MAX_AUTOCOMPLETE_VISIBLE {
        selected - MAX_AUTOCOMPLETE_VISIBLE + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, suggestion) in dialog
        .autocomplete_suggestions
        .iter()
        .skip(scroll_offset)
        .take(MAX_AUTOCOMPLETE_VISIBLE)
        .enumerate()
    {
        let actual_idx = i + scroll_offset;
        let is_selected = Some(actual_idx) == dialog.autocomplete_selected;

        let style = if is_selected {
            Style::default()
                .fg(theme.popup_bg)
                .bg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.editor_fg).bg(theme.popup_bg)
        };

        // Pad the suggestion to fill the width
        let display = pad_right(suggestion, inner.width as usize);
        lines.push(Line::from(Span::styled(display, style)));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Render the unsaved changes confirm dialog
fn render_confirm_dialog(frame: &mut Frame, area: Rect, editor: &KeybindingEditor, theme: &Theme) {
    let width = 44u16.min(area.width.saturating_sub(4));
    let height = 7u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let dialog_area = Rect {
        x,
        y,
        width,
        height,
    };
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Unsaved Changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.diagnostic_warning_fg))
        .style(Style::default().bg(theme.popup_bg).fg(theme.editor_fg));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::vertical([
        Constraint::Length(2), // Message
        Constraint::Length(1), // Spacer
        Constraint::Length(1), // Buttons
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " You have unsaved keybinding changes.",
            Style::default().fg(theme.editor_fg),
        ))),
        chunks[0],
    );

    let options = ["Save", "Discard", "Cancel"];
    let mut spans = vec![Span::raw(" ")];
    for (i, opt) in options.iter().enumerate() {
        let style = if i == editor.confirm_selection {
            Style::default()
                .fg(theme.popup_bg)
                .bg(theme.help_key_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.editor_fg)
        };
        spans.push(Span::styled(format!(" {} ", opt), style));
        spans.push(Span::raw("  "));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), chunks[2]);
}

/// Right-pad a string to a given width, truncating if necessary
fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_string()
    } else {
        format!("{:width$}", s, width = width)
    }
}

// ==================== INPUT HANDLING ====================

/// Handle input for the keybinding editor. Returns true if the editor should close.
pub fn handle_keybinding_editor_input(
    editor: &mut KeybindingEditor,
    event: &KeyEvent,
) -> KeybindingEditorAction {
    // Help overlay
    if editor.showing_help {
        match event.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter => {
                editor.showing_help = false;
            }
            _ => {}
        }
        return KeybindingEditorAction::Consumed;
    }

    // Confirm dialog
    if editor.showing_confirm_dialog {
        return handle_confirm_input(editor, event);
    }

    // Edit dialog
    if editor.edit_dialog.is_some() {
        return handle_edit_dialog_input(editor, event);
    }

    // Search mode
    if editor.search_active {
        return handle_search_input(editor, event);
    }

    // Main table navigation
    handle_main_input(editor, event)
}

/// Actions that the keybinding editor can return to the parent
pub enum KeybindingEditorAction {
    /// Input was consumed, no further action needed
    Consumed,
    /// Close the editor (no save)
    Close,
    /// Save and close
    SaveAndClose,
    /// Status message to display
    StatusMessage(String),
}

fn handle_main_input(editor: &mut KeybindingEditor, event: &KeyEvent) -> KeybindingEditorAction {
    match (event.code, event.modifiers) {
        // Close
        (KeyCode::Esc, KeyModifiers::NONE) => {
            if editor.has_changes {
                editor.showing_confirm_dialog = true;
                editor.confirm_selection = 0;
                KeybindingEditorAction::Consumed
            } else {
                KeybindingEditorAction::Close
            }
        }

        // Save
        (KeyCode::Char('s'), m) if m.contains(KeyModifiers::CONTROL) => {
            KeybindingEditorAction::SaveAndClose
        }

        // Navigation
        (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
            editor.select_prev();
            KeybindingEditorAction::Consumed
        }
        (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
            editor.select_next();
            KeybindingEditorAction::Consumed
        }
        (KeyCode::PageUp, _) => {
            editor.page_up();
            KeybindingEditorAction::Consumed
        }
        (KeyCode::PageDown, _) => {
            editor.page_down();
            KeybindingEditorAction::Consumed
        }
        (KeyCode::Home, _) => {
            editor.selected = 0;
            editor.scroll_offset = 0;
            KeybindingEditorAction::Consumed
        }
        (KeyCode::End, _) => {
            editor.selected = editor.filtered_indices.len().saturating_sub(1);
            editor.ensure_visible_public();
            KeybindingEditorAction::Consumed
        }

        // Search
        (KeyCode::Char('/'), KeyModifiers::NONE) => {
            editor.start_search();
            KeybindingEditorAction::Consumed
        }

        // Record key search
        (KeyCode::Char('r'), KeyModifiers::NONE) => {
            editor.start_record_key_search();
            KeybindingEditorAction::Consumed
        }

        // Help
        (KeyCode::Char('?'), _) => {
            editor.showing_help = true;
            KeybindingEditorAction::Consumed
        }

        // Add binding
        (KeyCode::Char('a'), KeyModifiers::NONE) => {
            editor.open_add_dialog();
            KeybindingEditorAction::Consumed
        }

        // Edit binding
        (KeyCode::Enter, KeyModifiers::NONE) => {
            editor.open_edit_dialog();
            KeybindingEditorAction::Consumed
        }

        // Delete binding
        (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _) => {
            if editor.delete_selected() {
                KeybindingEditorAction::StatusMessage("Custom binding removed".to_string())
            } else {
                KeybindingEditorAction::StatusMessage("Can only delete custom bindings".to_string())
            }
        }

        // Context filter
        (KeyCode::Char('c'), KeyModifiers::NONE) => {
            editor.cycle_context_filter();
            KeybindingEditorAction::Consumed
        }

        // Source filter
        (KeyCode::Char('s'), KeyModifiers::NONE) => {
            editor.cycle_source_filter();
            KeybindingEditorAction::Consumed
        }

        _ => KeybindingEditorAction::Consumed,
    }
}

fn handle_search_input(editor: &mut KeybindingEditor, event: &KeyEvent) -> KeybindingEditorAction {
    match editor.search_mode {
        SearchMode::Text => match (event.code, event.modifiers) {
            (KeyCode::Esc, _) => {
                editor.cancel_search();
                KeybindingEditorAction::Consumed
            }
            (KeyCode::Enter, _) => {
                editor.search_active = false;
                // Keep the filter results
                KeybindingEditorAction::Consumed
            }
            (KeyCode::Tab, _) => {
                // Switch to record key mode
                editor.search_mode = SearchMode::RecordKey;
                editor.search_key_display.clear();
                editor.search_key_code = None;
                KeybindingEditorAction::Consumed
            }
            (KeyCode::Backspace, _) => {
                editor.search_query.pop();
                editor.apply_filters();
                KeybindingEditorAction::Consumed
            }
            (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                editor.search_query.push(c);
                editor.apply_filters();
                KeybindingEditorAction::Consumed
            }
            _ => KeybindingEditorAction::Consumed,
        },
        SearchMode::RecordKey => match (event.code, event.modifiers) {
            (KeyCode::Esc, KeyModifiers::NONE) => {
                editor.cancel_search();
                KeybindingEditorAction::Consumed
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                // Switch to text mode
                editor.search_mode = SearchMode::Text;
                editor.search_query.clear();
                editor.apply_filters();
                KeybindingEditorAction::Consumed
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                editor.search_active = false;
                KeybindingEditorAction::Consumed
            }
            _ => {
                // Record the key
                editor.record_search_key(event);
                KeybindingEditorAction::Consumed
            }
        },
    }
}

fn handle_edit_dialog_input(
    editor: &mut KeybindingEditor,
    event: &KeyEvent,
) -> KeybindingEditorAction {
    // Take the dialog out to avoid borrow conflicts
    let mut dialog = match editor.edit_dialog.take() {
        Some(d) => d,
        None => return KeybindingEditorAction::Consumed,
    };

    // Close dialog on Esc
    if event.code == KeyCode::Esc && event.modifiers == KeyModifiers::NONE {
        // Don't put it back - it's closed
        return KeybindingEditorAction::Consumed;
    }

    match dialog.focus_area {
        0 => {
            // Key recording area
            match (event.code, event.modifiers) {
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    dialog.focus_area = 1;
                    dialog.mode = EditMode::EditingAction;
                }
                (KeyCode::Enter, KeyModifiers::NONE) if dialog.key_code.is_some() => {
                    dialog.focus_area = 1;
                    dialog.mode = EditMode::EditingAction;
                }
                _ => {
                    // Record the key (but not modifier-only presses)
                    match event.code {
                        KeyCode::Modifier(_) => {}
                        _ => {
                            dialog.key_code = Some(event.code);
                            dialog.modifiers = event.modifiers;
                            dialog.key_display = format_keybinding(&event.code, &event.modifiers);
                            // Check conflicts
                            dialog.conflicts =
                                editor.find_conflicts(event.code, event.modifiers, &dialog.context);
                        }
                    }
                }
            }
        }
        1 => {
            // Action editing area with autocomplete
            match (event.code, event.modifiers) {
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    // Accept selected autocomplete suggestion, or move to next field
                    if dialog.autocomplete_visible {
                        if let Some(sel) = dialog.autocomplete_selected {
                            if sel < dialog.autocomplete_suggestions.len() {
                                let suggestion = dialog.autocomplete_suggestions[sel].clone();
                                dialog.action_text = suggestion;
                                dialog.action_cursor = dialog.action_text.len();
                                dialog.autocomplete_visible = false;
                                dialog.autocomplete_selected = None;
                                dialog.action_error = None;
                            }
                        }
                    } else {
                        dialog.focus_area = 2;
                        dialog.mode = EditMode::EditingContext;
                    }
                }
                (KeyCode::BackTab, _) => {
                    dialog.autocomplete_visible = false;
                    dialog.focus_area = 0;
                    dialog.mode = EditMode::RecordingKey;
                }
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    // Accept selected autocomplete suggestion, or move to buttons
                    if dialog.autocomplete_visible {
                        if let Some(sel) = dialog.autocomplete_selected {
                            if sel < dialog.autocomplete_suggestions.len() {
                                let suggestion = dialog.autocomplete_suggestions[sel].clone();
                                dialog.action_text = suggestion;
                                dialog.action_cursor = dialog.action_text.len();
                                dialog.autocomplete_visible = false;
                                dialog.autocomplete_selected = None;
                                dialog.action_error = None;
                            }
                        }
                    } else {
                        dialog.focus_area = 3;
                        dialog.mode = EditMode::EditingContext;
                    }
                }
                (KeyCode::Up, _) if dialog.autocomplete_visible => {
                    // Navigate autocomplete up
                    if let Some(sel) = dialog.autocomplete_selected {
                        if sel > 0 {
                            dialog.autocomplete_selected = Some(sel - 1);
                        }
                    }
                }
                (KeyCode::Down, _) if dialog.autocomplete_visible => {
                    // Navigate autocomplete down
                    if let Some(sel) = dialog.autocomplete_selected {
                        let max = dialog.autocomplete_suggestions.len().saturating_sub(1);
                        if sel < max {
                            dialog.autocomplete_selected = Some(sel + 1);
                        }
                    }
                }
                (KeyCode::Esc, _) if dialog.autocomplete_visible => {
                    // Close autocomplete without closing dialog
                    dialog.autocomplete_visible = false;
                    dialog.autocomplete_selected = None;
                    // Put dialog back and return early (don't let outer Esc handler close dialog)
                    editor.edit_dialog = Some(dialog);
                    return KeybindingEditorAction::Consumed;
                }
                (KeyCode::Backspace, _) => {
                    if dialog.action_cursor > 0 {
                        dialog.action_cursor -= 1;
                        dialog.action_text.remove(dialog.action_cursor);
                        dialog.action_error = None;
                    }
                    // Put dialog back and update autocomplete
                    editor.edit_dialog = Some(dialog);
                    editor.update_autocomplete();
                    return KeybindingEditorAction::Consumed;
                }
                (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                    dialog.action_text.insert(dialog.action_cursor, c);
                    dialog.action_cursor += 1;
                    dialog.action_error = None;
                    // Put dialog back and update autocomplete
                    editor.edit_dialog = Some(dialog);
                    editor.update_autocomplete();
                    return KeybindingEditorAction::Consumed;
                }
                _ => {}
            }
        }
        2 => {
            // Context selection area
            match (event.code, event.modifiers) {
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    dialog.focus_area = 3;
                }
                (KeyCode::BackTab, _) => {
                    dialog.focus_area = 1;
                    dialog.mode = EditMode::EditingAction;
                }
                (KeyCode::Left, _) => {
                    if dialog.context_option_index > 0 {
                        dialog.context_option_index -= 1;
                        dialog.context =
                            dialog.context_options[dialog.context_option_index].clone();
                        // Update conflicts
                        if let Some(key_code) = dialog.key_code {
                            dialog.conflicts =
                                editor.find_conflicts(key_code, dialog.modifiers, &dialog.context);
                        }
                    }
                }
                (KeyCode::Right, _) => {
                    if dialog.context_option_index + 1 < dialog.context_options.len() {
                        dialog.context_option_index += 1;
                        dialog.context =
                            dialog.context_options[dialog.context_option_index].clone();
                        if let Some(key_code) = dialog.key_code {
                            dialog.conflicts =
                                editor.find_conflicts(key_code, dialog.modifiers, &dialog.context);
                        }
                    }
                }
                (KeyCode::Enter, _) => {
                    dialog.focus_area = 3;
                }
                _ => {}
            }
        }
        3 => {
            // Buttons area
            match (event.code, event.modifiers) {
                (KeyCode::Tab, KeyModifiers::NONE) => {
                    dialog.focus_area = 0;
                    dialog.mode = EditMode::RecordingKey;
                }
                (KeyCode::BackTab, _) => {
                    dialog.focus_area = 2;
                    dialog.mode = EditMode::EditingContext;
                }
                (KeyCode::Left, _) => {
                    if dialog.selected_button > 0 {
                        dialog.selected_button -= 1;
                    }
                }
                (KeyCode::Right, _) => {
                    if dialog.selected_button < 1 {
                        dialog.selected_button += 1;
                    }
                }
                (KeyCode::Enter, _) => {
                    if dialog.selected_button == 0 {
                        // Save - put the dialog back first so apply_edit_dialog can take it
                        editor.edit_dialog = Some(dialog);
                        if let Some(err) = editor.apply_edit_dialog() {
                            // Validation failed - dialog is still open with error
                            return KeybindingEditorAction::StatusMessage(err);
                        }
                        return KeybindingEditorAction::Consumed;
                    } else {
                        // Cancel - don't put dialog back
                        return KeybindingEditorAction::Consumed;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    // Put the dialog back
    editor.edit_dialog = Some(dialog);
    KeybindingEditorAction::Consumed
}

fn handle_confirm_input(editor: &mut KeybindingEditor, event: &KeyEvent) -> KeybindingEditorAction {
    match (event.code, event.modifiers) {
        (KeyCode::Left, _) => {
            if editor.confirm_selection > 0 {
                editor.confirm_selection -= 1;
            }
            KeybindingEditorAction::Consumed
        }
        (KeyCode::Right, _) => {
            if editor.confirm_selection < 2 {
                editor.confirm_selection += 1;
            }
            KeybindingEditorAction::Consumed
        }
        (KeyCode::Enter, _) => match editor.confirm_selection {
            0 => KeybindingEditorAction::SaveAndClose,
            1 => KeybindingEditorAction::Close, // Discard
            _ => {
                editor.showing_confirm_dialog = false;
                KeybindingEditorAction::Consumed
            }
        },
        (KeyCode::Esc, _) => {
            editor.showing_confirm_dialog = false;
            KeybindingEditorAction::Consumed
        }
        _ => KeybindingEditorAction::Consumed,
    }
}
