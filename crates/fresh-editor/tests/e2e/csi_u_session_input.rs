//! Regression tests for issue #1113: CSI u escape sequences are written as
//! literal text in session attach mode.
//!
//! In session mode, raw terminal bytes flow through `InputParser` on the server
//! side. The parser must recognise CSI u sequences (the fixterms / kitty
//! keyboard protocol) and convert them to the appropriate crossterm events.
//! Before the fix, `parse_csi_final` had no handler for the `u` final byte,
//! causing the sequence to be treated as `Invalid` and its bytes dumped as
//! literal text into the editor buffer.

use crate::common::harness::EditorTestHarness;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use fresh::server::input_parser::InputParser;

/// Helper: parse raw bytes through InputParser, feed resulting key events into
/// the editor (same code path as EditorServer), then return the buffer content.
fn parse_and_apply(harness: &mut EditorTestHarness, parser: &mut InputParser, bytes: &[u8]) {
    let events = parser.parse(bytes);
    for event in events {
        if let Event::Key(ke) = event {
            harness
                .send_key(ke.code, ke.modifiers)
                .expect("send_key failed");
        }
    }
}

// ---------------------------------------------------------------------------
// Reproduction: CSI u sequences must NOT appear as literal text
// ---------------------------------------------------------------------------

/// \x1b[13;5u = Ctrl+Enter in CSI u format.
/// Before the fix this inserted "[13;5u" as literal text.
#[test]
fn test_csi_u_ctrl_enter_not_literal_text() {
    let mut harness = EditorTestHarness::new(80, 24).unwrap();
    let mut parser = InputParser::new();

    // Type "hello" then send Ctrl+Enter
    parse_and_apply(&mut harness, &mut parser, b"hello");
    parse_and_apply(&mut harness, &mut parser, b"\x1b[13;5u");

    let content = harness.get_buffer_content().unwrap_or_default();
    assert!(
        !content.contains("[13;5u"),
        "CSI u sequence leaked as literal text: {:?}",
        content
    );
}

/// \x1b[9;5u = Ctrl+Tab in CSI u format.
#[test]
fn test_csi_u_ctrl_tab_not_literal_text() {
    let mut harness = EditorTestHarness::new(80, 24).unwrap();
    let mut parser = InputParser::new();

    parse_and_apply(&mut harness, &mut parser, b"hello");
    parse_and_apply(&mut harness, &mut parser, b"\x1b[9;5u");

    let content = harness.get_buffer_content().unwrap_or_default();
    assert!(
        !content.contains("[9;5u"),
        "CSI u sequence leaked as literal text: {:?}",
        content
    );
}

/// \x1b[13;2u = Shift+Enter in CSI u format.
#[test]
fn test_csi_u_shift_enter_not_literal_text() {
    let mut harness = EditorTestHarness::new(80, 24).unwrap();
    let mut parser = InputParser::new();

    parse_and_apply(&mut harness, &mut parser, b"hello");
    parse_and_apply(&mut harness, &mut parser, b"\x1b[13;2u");

    let content = harness.get_buffer_content().unwrap_or_default();
    assert!(
        !content.contains("[13;2u"),
        "CSI u sequence leaked as literal text: {:?}",
        content
    );
}

/// \x1b[97u = 'a' key with no modifiers in CSI u format (keycode 97 = 'a').
#[test]
fn test_csi_u_plain_key_not_literal_text() {
    let mut harness = EditorTestHarness::new(80, 24).unwrap();
    let mut parser = InputParser::new();

    parse_and_apply(&mut harness, &mut parser, b"\x1b[97u");

    let content = harness.get_buffer_content().unwrap_or_default();
    assert!(
        !content.contains("[97u"),
        "CSI u sequence leaked as literal text: {:?}",
        content
    );
}

// ---------------------------------------------------------------------------
// InputParser unit-level: CSI u sequences produce correct events
// ---------------------------------------------------------------------------

/// Verify that InputParser correctly parses \x1b[13;5u as Ctrl+Enter.
#[test]
fn test_input_parser_csi_u_ctrl_enter() {
    let mut parser = InputParser::new();
    let events = parser.parse(b"\x1b[13;5u");

    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Enter, "Expected Enter keycode");
            assert!(
                ke.modifiers.contains(KeyModifiers::CONTROL),
                "Expected Ctrl modifier, got {:?}",
                ke.modifiers
            );
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}

/// Verify \x1b[9;5u → Ctrl+Tab.
#[test]
fn test_input_parser_csi_u_ctrl_tab() {
    let mut parser = InputParser::new();
    let events = parser.parse(b"\x1b[9;5u");

    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Tab, "Expected Tab keycode");
            assert!(
                ke.modifiers.contains(KeyModifiers::CONTROL),
                "Expected Ctrl modifier, got {:?}",
                ke.modifiers
            );
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}

/// Verify \x1b[97u → 'a' with no modifiers.
#[test]
fn test_input_parser_csi_u_plain_a() {
    let mut parser = InputParser::new();
    let events = parser.parse(b"\x1b[97u");

    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Char('a'), "Expected 'a' keycode");
            assert!(
                ke.modifiers.is_empty(),
                "Expected no modifiers, got {:?}",
                ke.modifiers
            );
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}

/// Verify \x1b[127;5u → Ctrl+Backspace.
#[test]
fn test_input_parser_csi_u_ctrl_backspace() {
    let mut parser = InputParser::new();
    let events = parser.parse(b"\x1b[127;5u");

    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Backspace, "Expected Backspace keycode");
            assert!(
                ke.modifiers.contains(KeyModifiers::CONTROL),
                "Expected Ctrl modifier, got {:?}",
                ke.modifiers
            );
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}

/// Verify \x1b[27u → Escape with no modifiers.
#[test]
fn test_input_parser_csi_u_escape() {
    let mut parser = InputParser::new();
    let events = parser.parse(b"\x1b[27u");

    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Esc, "Expected Escape keycode");
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}

/// Verify CSI u sequence split across parse chunks is handled correctly.
#[test]
fn test_input_parser_csi_u_split_across_chunks() {
    let mut parser = InputParser::new();

    // First chunk: ESC [
    let events = parser.parse(b"\x1b[13");
    assert!(events.is_empty(), "Incomplete CSI u should buffer");

    // Second chunk: ;5u
    let events = parser.parse(b";5u");
    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Enter);
            assert!(ke.modifiers.contains(KeyModifiers::CONTROL));
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}

/// Verify \x1b[13;4u → Alt+Ctrl+Enter (modifier 4 = 1+Alt+Ctrl = Shift+Ctrl,
/// actually modifier 4 means 1 + 2(alt) + 0 = shift+alt... let me check).
/// modifier encoding: value = 1 + (shift) + 2*(alt) + 4*(ctrl)
/// So 4 = 1 + 0 + 0 + 4*(0) ... no, 4 = 1 + shift(1) + alt(2) = shift+alt
/// Actually: modifier_param - 1 = bitmask, so 4 - 1 = 3 = shift(1) | alt(2)
#[test]
fn test_input_parser_csi_u_shift_alt_enter() {
    let mut parser = InputParser::new();
    // modifier 4 → param-1 = 3 → shift(1) | alt(2)
    let events = parser.parse(b"\x1b[13;4u");

    assert_eq!(events.len(), 1, "Expected 1 event, got: {:?}", events);
    match &events[0] {
        Event::Key(ke) => {
            assert_eq!(ke.code, KeyCode::Enter);
            assert!(
                ke.modifiers.contains(KeyModifiers::SHIFT),
                "Expected Shift modifier"
            );
            assert!(
                ke.modifiers.contains(KeyModifiers::ALT),
                "Expected Alt modifier"
            );
        }
        other => panic!("Expected Key event, got {:?}", other),
    }
}
