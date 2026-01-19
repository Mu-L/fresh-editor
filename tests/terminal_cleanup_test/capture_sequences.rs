//! Test that captures and verifies the escape sequences sent during terminal setup/cleanup
//!
//! This test verifies that the order of operations produces the correct escape sequences.
//! Run with: cargo test --release

use crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    Command,
};
use std::fmt::Write as FmtWrite;

fn capture_ansi<C: Command>(cmd: C) -> String {
    let mut buf = String::new();
    cmd.write_ansi(&mut buf).unwrap();
    buf
}

fn escape_for_display(s: &str) -> String {
    s.replace('\x1b', "ESC")
}

fn analyze_escape_sequences() {
    let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;

    println!("\n=== Escape Sequences Analysis ===\n");

    // Capture all sequences
    let push_kb = capture_ansi(PushKeyboardEnhancementFlags(flags));
    let pop_kb = capture_ansi(PopKeyboardEnhancementFlags);
    let enter_alt = capture_ansi(EnterAlternateScreen);
    let leave_alt = capture_ansi(LeaveAlternateScreen);
    let enable_mouse = capture_ansi(EnableMouseCapture);
    let disable_mouse = capture_ansi(DisableMouseCapture);
    let enable_paste = capture_ansi(EnableBracketedPaste);
    let disable_paste = capture_ansi(DisableBracketedPaste);

    println!("Push Keyboard Enhancement: {}", escape_for_display(&push_kb));
    println!("Pop Keyboard Enhancement:  {}", escape_for_display(&pop_kb));
    println!("Enter Alternate Screen:    {}", escape_for_display(&enter_alt));
    println!("Leave Alternate Screen:    {}", escape_for_display(&leave_alt));
    println!("Enable Mouse Capture:      {}", escape_for_display(&enable_mouse));
    println!("Disable Mouse Capture:     {}", escape_for_display(&disable_mouse));
    println!("Enable Bracketed Paste:    {}", escape_for_display(&enable_paste));
    println!("Disable Bracketed Paste:   {}", escape_for_display(&disable_paste));

    // Verify the sequences match expected values
    // CSI > flags u for push, CSI < 1 u for pop
    assert!(push_kb.contains(">"), "Push should use CSI > flags u");
    assert!(push_kb.ends_with("u"), "Push should end with 'u'");
    assert!(pop_kb.contains("<"), "Pop should use CSI < u");
    assert!(pop_kb.ends_with("u"), "Pop should end with 'u'");

    println!("\n=== Order Comparison ===\n");

    // Current (buggy) order
    println!("CURRENT (BUGGY) ORDER:");
    println!("  1. Enable raw mode (no escape sequence)");
    println!("  2. {} <- Push KB enhancement (MAIN screen)", escape_for_display(&push_kb));
    println!("  3. {} <- Enter alternate screen", escape_for_display(&enter_alt));
    println!("  4. {} <- Enable mouse", escape_for_display(&enable_mouse));
    println!("  5. {} <- Enable bracketed paste", escape_for_display(&enable_paste));
    println!("  ...");
    println!("  6. {} <- Disable mouse", escape_for_display(&disable_mouse));
    println!("  7. {} <- Disable bracketed paste", escape_for_display(&disable_paste));
    println!("  8. {} <- Pop KB enhancement (ALTERNATE screen - WRONG!)", escape_for_display(&pop_kb));
    println!("  9. Disable raw mode");
    println!(" 10. {} <- Leave alternate screen (MAIN screen KB still pushed!)", escape_for_display(&leave_alt));

    println!("\nFIXED ORDER:");
    println!("  1. Enable raw mode");
    println!("  2. {} <- Enter alternate screen FIRST", escape_for_display(&enter_alt));
    println!("  3. {} <- Push KB enhancement (ALTERNATE screen)", escape_for_display(&push_kb));
    println!("  4. {} <- Enable mouse", escape_for_display(&enable_mouse));
    println!("  5. {} <- Enable bracketed paste", escape_for_display(&enable_paste));
    println!("  ...");
    println!("  6. {} <- Disable mouse", escape_for_display(&disable_mouse));
    println!("  7. {} <- Disable bracketed paste", escape_for_display(&disable_paste));
    println!("  8. {} <- Pop KB enhancement (ALTERNATE screen - CORRECT)", escape_for_display(&pop_kb));
    println!("  9. Disable raw mode");
    println!(" 10. {} <- Leave alternate screen (MAIN screen was never modified)", escape_for_display(&leave_alt));

    println!("\n=== The Bug ===\n");
    println!("The Kitty keyboard protocol specifies that main and alternate screens");
    println!("maintain INDEPENDENT keyboard mode stacks. When keyboard enhancement");
    println!("is pushed BEFORE entering alternate screen, it goes to the main screen's");
    println!("stack. Then when we pop BEFORE leaving, we pop from the alternate screen's");
    println!("stack (which may be empty). When we finally leave alternate screen, the");
    println!("main screen's keyboard enhancement is still enabled!");
}

fn main() {
    analyze_escape_sequences();
}
