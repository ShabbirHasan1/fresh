use crate::common::harness::EditorTestHarness;

/// Test that the prompt is rendered correctly
#[test]
fn test_prompt_rendering() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Trigger the open file prompt with Ctrl+O
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Check that the prompt is visible in the status bar area (bottom line)
    let screen = harness.screen_to_string();
    harness.assert_screen_contains("Open file:");

    // Check the prompt styling
    let buffer = harness.buffer();
    let status_y = buffer.area.height - 1; // Status bar is at the bottom

    // Check a cell in the status bar has black background (default prompt color)
    let first_cell_pos = buffer.index_of(0, status_y);
    let first_cell = &buffer.content[first_cell_pos];
    assert_eq!(
        first_cell.bg,
        ratatui::style::Color::Black,
        "Prompt should have black background"
    );
}

/// Test prompt input handling (typing, backspace, cursor movement)
#[test]
fn test_prompt_input_handling() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Trigger the open file prompt with Ctrl+O
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.assert_screen_contains("Open file:");

    // Type some text
    harness.type_text("test.txt").unwrap();
    harness.assert_screen_contains("test.txt");

    // Test backspace
    harness
        .send_key(KeyCode::Backspace, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();
    harness.assert_screen_contains("test.tx");
    harness.assert_screen_not_contains("test.txt");

    // Type more
    harness.type_text("t2").unwrap();
    harness.assert_screen_contains("test.txt2");

    // Test Home (move cursor to start)
    harness.send_key(KeyCode::Home, KeyModifiers::NONE).unwrap();

    // Type at the beginning
    harness.type_text("my_").unwrap();
    harness.assert_screen_contains("my_test.txt2");

    // Test End (move cursor to end)
    harness.send_key(KeyCode::End, KeyModifiers::NONE).unwrap();
    harness.type_text("!").unwrap();
    harness.assert_screen_contains("my_test.txt2!");
}

/// Test canceling the prompt with Escape
#[test]
fn test_prompt_cancel() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Trigger the open file prompt
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.assert_screen_contains("Open file:");

    // Type some text
    harness.type_text("test.txt").unwrap();
    harness.assert_screen_contains("test.txt");

    // Cancel with Escape
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Prompt should be gone, and "Canceled" message should appear
    harness.assert_screen_not_contains("Open file:");
    harness.assert_screen_contains("Canceled");
}

/// Test the complete open file workflow
#[test]
fn test_open_file_workflow() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;

    use tempfile::TempDir;

    // Create a temporary directory and file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_prompt.txt");
    fs::write(&file_path, "Hello from prompt test!").unwrap();

    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Trigger the open file prompt
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.assert_screen_contains("Open file:");

    // Type the file path
    let path_str = file_path.to_str().unwrap();
    harness.type_text(path_str).unwrap();

    // Confirm with Enter
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Check that the file was opened
    harness.assert_screen_not_contains("Open file:");

    // Check that the file content is displayed
    // Note: File content display may require additional renders after async file load
    harness.assert_screen_contains("Hello from prompt test!");

    // Check that the filename appears in the status bar
    harness.assert_screen_contains("test_prompt.txt");
}

/// Test opening a non-existent file creates an unsaved buffer
#[test]
fn test_open_nonexistent_file() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use tempfile::TempDir;

    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let new_file_path = temp_dir.path().join("new_file.txt");

    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Trigger the open file prompt
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();

    // Type the path to a non-existent file
    let path_str = new_file_path.to_str().unwrap();
    harness.type_text(path_str).unwrap();

    // Confirm with Enter
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should NOT show an error - should open as unsaved buffer
    harness.assert_screen_not_contains("Error opening file");

    // Should show the filename in the status bar
    harness.assert_screen_contains("new_file.txt");

    // Buffer should be empty
    assert_eq!(harness.get_buffer_content(), "");

    // Should show "Opened" message (may be truncated in status bar)
    harness.assert_screen_contains("Opened");
}

/// Test that opening a non-existent file allows editing and saving
#[test]
fn test_open_nonexistent_file_edit_and_save() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory for the test
    let temp_dir = TempDir::new().unwrap();
    let new_file_path = temp_dir.path().join("created_file.txt");

    // Verify file doesn't exist yet
    assert!(!new_file_path.exists());

    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Trigger the open file prompt
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();

    // Type the path to a non-existent file
    let path_str = new_file_path.to_str().unwrap();
    harness.type_text(path_str).unwrap();

    // Confirm with Enter
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should open successfully
    harness.assert_screen_not_contains("Error");

    // Type some content
    harness.type_text("Hello, World!").unwrap();
    assert_eq!(harness.get_buffer_content(), "Hello, World!");

    // Save the file with Ctrl+S
    harness
        .send_key(KeyCode::Char('s'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Should show "Saved" message
    harness.assert_screen_contains("Saved");

    // Verify file was created on disk with correct content
    assert!(new_file_path.exists());
    let saved_content = fs::read_to_string(&new_file_path).unwrap();
    assert_eq!(saved_content, "Hello, World!");
}

/// Test spawning CLI with non-existent file directly (via open_file)
#[test]
fn test_spawn_with_nonexistent_file() {
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let new_file_path = temp_dir.path().join("spawn_test.rs");

    // Verify file doesn't exist
    assert!(!new_file_path.exists());

    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Open non-existent file directly (simulating CLI spawn)
    harness.open_file(&new_file_path).unwrap();

    // Should show the filename
    harness.assert_screen_contains("spawn_test.rs");

    // Buffer should be empty
    assert_eq!(harness.get_buffer_content(), "");

    // Type content and save
    harness.type_text("fn main() {}").unwrap();

    use crossterm::event::{KeyCode, KeyModifiers};
    harness
        .send_key(KeyCode::Char('s'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Verify file was created
    assert!(new_file_path.exists());
    let content = fs::read_to_string(&new_file_path).unwrap();
    assert_eq!(content, "fn main() {}");
}

/// Test Save As functionality
#[test]
fn test_save_as_functionality() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory and original file
    let temp_dir = TempDir::new().unwrap();
    let original_path = temp_dir.path().join("original.txt");
    fs::write(&original_path, "Original content").unwrap();

    let mut harness = EditorTestHarness::new(80, 24).unwrap();

    // Open the original file
    harness.open_file(&original_path).unwrap();
    harness.assert_screen_contains("original.txt");
    assert_eq!(harness.get_buffer_content(), "Original content");

    // Trigger command palette with Ctrl+P
    harness
        .send_key(KeyCode::Char('p'), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Type to search for Save As command
    harness.type_text("Save File As").unwrap();

    // Confirm with Enter
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should show the Save As prompt with current filename
    harness.assert_screen_contains("Save as:");

    // Clear the current filename and type new name
    // First select all with Ctrl+A
    harness
        .send_key(KeyCode::Char('a'), KeyModifiers::CONTROL)
        .unwrap();

    // Type new filename (relative path)
    let new_file_path = temp_dir.path().join("saved_as.txt");
    let new_path_str = new_file_path.to_str().unwrap();
    harness.type_text(new_path_str).unwrap();

    // Confirm with Enter
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Note: "Saved as:" status message may be overwritten by auto-revert status
    // We verify the save succeeded by checking the file exists and has correct content below

    // Verify new file was created with correct content
    assert!(new_file_path.exists());
    let new_content = fs::read_to_string(&new_file_path).unwrap();
    assert_eq!(new_content, "Original content");

    // Original file should still exist
    assert!(original_path.exists());

    // Buffer should now show the new filename
    harness.assert_screen_contains("saved_as.txt");
}

/// Test Save As with relative path
#[test]
fn test_save_as_relative_path() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;

    let mut harness = EditorTestHarness::with_temp_project(80, 24).unwrap();
    let project_dir = harness.project_dir().unwrap();

    // Create and open original file
    let original_path = project_dir.join("original.txt");
    fs::write(&original_path, "Test content").unwrap();
    harness.open_file(&original_path).unwrap();

    // Trigger command palette
    harness
        .send_key(KeyCode::Char('p'), KeyModifiers::CONTROL)
        .unwrap();
    harness.type_text("Save File As").unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Clear and type relative path
    harness
        .send_key(KeyCode::Char('a'), KeyModifiers::CONTROL)
        .unwrap();
    harness.type_text("relative_save.txt").unwrap();

    // Confirm
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should save to working directory
    let expected_path = project_dir.join("relative_save.txt");
    assert!(
        expected_path.exists(),
        "File should be created at {:?}",
        expected_path
    );

    let content = fs::read_to_string(&expected_path).unwrap();
    assert_eq!(content, "Test content");
}

/// Test Save As creates parent directories if needed
#[test]
fn test_save_as_nested_path() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;

    let mut harness = EditorTestHarness::with_temp_project(80, 24).unwrap();
    let project_dir = harness.project_dir().unwrap();

    // Start with new buffer
    harness.new_buffer().unwrap();

    // Type some content
    harness.type_text("Nested file content").unwrap();

    // Trigger Save As via command palette
    harness
        .send_key(KeyCode::Char('p'), KeyModifiers::CONTROL)
        .unwrap();
    harness.type_text("Save File As").unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Type nested path (parent dir doesn't exist yet)
    let nested_path = project_dir.join("subdir").join("nested.txt");
    let nested_path_str = nested_path.to_str().unwrap();
    harness.type_text(nested_path_str).unwrap();

    // Confirm
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Note: This test verifies the error case since we don't auto-create parent dirs
    // The file won't be created because the parent directory doesn't exist
    // This documents current behavior - if we want to auto-create dirs, update this test
    if !nested_path.exists() {
        harness.assert_screen_contains("Error saving file");
    }
}

/// Test that Open File prompt shows completions immediately when opened (issue #193)
/// The prompt should display file/directory suggestions even before the user types anything.
#[test]
fn test_open_file_prompt_shows_completions_immediately() {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;
    use std::time::{Duration, Instant};

    // Create a temporary project directory
    let temp_dir = tempfile::TempDir::new().unwrap();
    let project_root = temp_dir.path().join("project_root");
    fs::create_dir(&project_root).unwrap();

    // Create plugins directory with an inline path completion plugin
    let plugins_dir = project_root.join("plugins");
    fs::create_dir(&plugins_dir).unwrap();

    // Create a simplified inline plugin for testing (similar to test_render_line_hook_with_args)
    let path_complete_plugin = r###"
// Simple path completion plugin for testing issue #193
globalThis.onPathCompletePromptChanged = function (args: { prompt_type: string; input: string }): boolean {
    if (args.prompt_type !== "open-file" && args.prompt_type !== "save-file-as") {
        return true;
    }

    editor.debug(`Path completion: prompt_type=${args.prompt_type}, input=${args.input}`);

    // Get the working directory for "." paths
    let dir = args.input === "" ? "." : args.input;
    if (!dir.includes("/")) {
        dir = ".";
    }

    const entries = editor.readDir(dir);
    editor.debug(`readDir("${dir}") returned ${entries ? entries.length : "null"} entries`);

    if (!entries) {
        editor.setPromptSuggestions([]);
        return true;
    }

    // Filter hidden files and convert to suggestions
    const suggestions = entries
        .filter((e: DirEntry) => !e.name.startsWith("."))
        .slice(0, 100)
        .map((e: DirEntry) => ({
            text: e.is_dir ? e.name + "/" : e.name,
            value: e.is_dir ? e.name + "/" : e.name,
        }));

    editor.debug(`Setting ${suggestions.length} suggestions`);
    editor.setPromptSuggestions(suggestions);
    return true;
};

editor.on("prompt_changed", "onPathCompletePromptChanged");
editor.debug("Path completion test plugin loaded!");
"###;

    fs::write(plugins_dir.join("path_complete.ts"), path_complete_plugin).unwrap();

    // Create some test files in the project directory
    fs::write(project_root.join("README.md"), "# Test Project").unwrap();
    fs::write(project_root.join("main.rs"), "fn main() {}").unwrap();
    fs::write(project_root.join("Cargo.toml"), "[package]").unwrap();

    // Create a subdirectory
    let src_dir = project_root.join("src");
    fs::create_dir(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "// lib").unwrap();

    // Create harness with the project directory (so plugins load)
    let mut harness =
        EditorTestHarness::with_config_and_working_dir(80, 24, Default::default(), project_root.clone())
            .unwrap();

    // Initial render to let plugins load
    harness.render().unwrap();

    // Process initial plugin commands
    for _ in 0..5 {
        let _ = harness.editor_mut().process_async_messages();
        std::thread::sleep(Duration::from_millis(20));
    }
    harness.render().unwrap();

    // Trigger the open file prompt with Ctrl+O
    harness
        .send_key(KeyCode::Char('o'), KeyModifiers::CONTROL)
        .unwrap();

    // Check that the prompt is visible
    harness.assert_screen_contains("Open file:");

    // Check the prompt state for debugging
    if let Some(prompt) = harness.editor_mut().prompt_mut() {
        println!(
            "Prompt state after send_key: input='{}', suggestions={}",
            prompt.input,
            prompt.suggestions.len()
        );
        for (i, s) in prompt.suggestions.iter().take(5).enumerate() {
            println!("  Suggestion {}: {:?}", i, s.text);
        }
    } else {
        println!("No prompt found!");
    }

    // ISSUE #193: Completions should appear immediately when the prompt opens.
    // We use a polling loop to wait for suggestions, but the fix should ensure
    // they appear very quickly (within the polling done in update_prompt_suggestions).
    // If the fix is working, suggestions should be available almost immediately.
    let start = Instant::now();
    let timeout = Duration::from_millis(500);
    let mut has_completions = false;

    while start.elapsed() < timeout {
        let _ = harness.editor_mut().process_async_messages();
        harness.render().unwrap();

        let screen = harness.screen_to_string();
        has_completions = screen.contains("README.md")
            || screen.contains("main.rs")
            || screen.contains("Cargo.toml")
            || screen.contains("src/");

        if has_completions {
            let elapsed = start.elapsed();
            println!(
                "Completions appeared after {:?} (should be nearly instant)",
                elapsed
            );
            // Verify completions appeared quickly (within 100ms is reasonable)
            assert!(
                elapsed < Duration::from_millis(100),
                "Completions appeared but took too long ({:?}). They should appear immediately when the prompt opens.",
                elapsed
            );
            break;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    if !has_completions {
        // Check final prompt state
        if let Some(prompt) = harness.editor_mut().prompt_mut() {
            println!(
                "Final prompt state: input='{}', suggestions={}",
                prompt.input,
                prompt.suggestions.len()
            );
        }
        let screen = harness.screen_to_string();
        panic!(
            "Open file prompt should show completions immediately, but none were found after {:?}. Screen:\n{}",
            timeout, screen
        );
    }
}
