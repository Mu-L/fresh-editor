//! E2E tests for the settings modal

use crate::common::harness::EditorTestHarness;
use crossterm::event::{KeyCode, KeyModifiers};

/// Test opening settings modal with Ctrl+,
#[test]
fn test_open_settings_modal() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Render initial state
    harness.render().unwrap();

    // Settings should not be visible initially
    harness.assert_screen_not_contains("Settings");

    // Open settings with Ctrl+,
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Settings modal should now be visible
    harness.assert_screen_contains("Settings");
}

/// Test closing settings modal with Escape
#[test]
fn test_close_settings_with_escape() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();
    harness.assert_screen_contains("Settings");

    // Close with Escape
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Settings should be closed
    harness.assert_screen_not_contains("Settings");
}

/// Test settings navigation with arrow keys
#[test]
fn test_settings_navigation() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Navigate down in categories
    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Switch to settings panel with Tab
    harness.send_key(KeyCode::Tab, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Navigate down in settings
    harness.send_key(KeyCode::Down, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Close settings
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
}

/// Test settings search with /
#[test]
fn test_settings_search() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Start search with /
    harness
        .send_key(KeyCode::Char('/'), KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Type a search query
    harness
        .send_key(KeyCode::Char('t'), KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Char('h'), KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Char('e'), KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Char('m'), KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Char('e'), KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should show search results
    // The search query "theme" should match theme-related settings

    // Cancel search with Escape
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Close settings
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
}

/// Test settings help overlay with ?
#[test]
fn test_settings_help_overlay() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Open help with ?
    harness
        .send_key(KeyCode::Char('?'), KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Help overlay should be visible
    harness.assert_screen_contains("Keyboard Shortcuts");

    // Close help with Escape
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Settings should still be visible
    harness.assert_screen_contains("Settings");

    // Close settings
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
}

/// Test search text input is displayed in search box
#[test]
fn test_settings_search_text_displays() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Start search with /
    harness
        .send_key(KeyCode::Char('/'), KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should show search mode indicator
    harness.assert_screen_contains("Type to search");

    // Type search query "tab"
    harness
        .send_key(KeyCode::Char('t'), KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Char('a'), KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Char('b'), KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Search text should be visible in the search box
    harness.assert_screen_contains("tab");

    // Should show results count
    harness.assert_screen_contains("results");

    // Close with Escape
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
}

/// Test toggling a setting shows modified indicator
#[test]
fn test_settings_toggle_shows_modified() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Use search to find "Check For Updates" (a toggle setting)
    harness
        .send_key(KeyCode::Char('/'), KeyModifiers::NONE)
        .unwrap();
    for c in "check".chars() {
        harness
            .send_key(KeyCode::Char(c), KeyModifiers::NONE)
            .unwrap();
    }
    harness.render().unwrap();

    // Jump to result and toggle
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Toggle the setting
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Should show modified indicator in title
    harness.assert_screen_contains("modified");

    // Close and discard
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();
    // Select "Discard" (one right from "Save and Exit")
    harness
        .send_key(KeyCode::Right, KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
}

/// Test confirmation dialog shows pending changes
#[test]
fn test_confirmation_dialog_shows_changes() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Use search to find "Check For Updates"
    harness
        .send_key(KeyCode::Char('/'), KeyModifiers::NONE)
        .unwrap();
    for c in "check".chars() {
        harness
            .send_key(KeyCode::Char(c), KeyModifiers::NONE)
            .unwrap();
    }
    harness.render().unwrap();

    // Jump to result and toggle
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Press Escape to trigger confirmation dialog
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // Dialog should show
    harness.assert_screen_contains("Unsaved Changes");
    harness.assert_screen_contains("You have unsaved changes");

    // Should show the actual change (path contains "check_for_updates")
    harness.assert_screen_contains("check_for_updates");

    // Cancel dialog
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
}

/// Test confirmation dialog button navigation
#[test]
fn test_confirmation_dialog_button_navigation() {
    let mut harness = EditorTestHarness::new(100, 40).unwrap();

    // Open settings
    harness
        .send_key(KeyCode::Char(','), KeyModifiers::CONTROL)
        .unwrap();
    harness.render().unwrap();

    // Use search to find and toggle a setting
    harness
        .send_key(KeyCode::Char('/'), KeyModifiers::NONE)
        .unwrap();
    for c in "check".chars() {
        harness
            .send_key(KeyCode::Char(c), KeyModifiers::NONE)
            .unwrap();
    }
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Open confirmation dialog
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness.render().unwrap();

    // First button should be selected (Save and Exit has ▶ indicator)
    harness.assert_screen_contains("▶[ Save and Exit ]");

    // Navigate right to Discard
    harness
        .send_key(KeyCode::Right, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Discard should now be selected
    harness.assert_screen_contains("▶[ Discard ]");

    // Navigate right to Cancel
    harness
        .send_key(KeyCode::Right, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Cancel should now be selected
    harness.assert_screen_contains("▶[ Cancel ]");

    // Press Enter on Cancel to close dialog
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
    harness.render().unwrap();

    // Dialog should be closed but settings still open
    harness.assert_screen_not_contains("Unsaved Changes");
    harness.assert_screen_contains("Settings");

    // Discard and close
    harness.send_key(KeyCode::Esc, KeyModifiers::NONE).unwrap();
    harness
        .send_key(KeyCode::Right, KeyModifiers::NONE)
        .unwrap();
    harness
        .send_key(KeyCode::Enter, KeyModifiers::NONE)
        .unwrap();
}
