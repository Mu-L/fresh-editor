//! Minimal test to verify keyboard enhancement cleanup order bug
//!
//! Run with: cargo run --release
//!
//! Test 1 (buggy): cargo run --release -- buggy
//! Test 2 (fixed): cargo run --release -- fixed
//!
//! After each test, try pressing arrow keys or Enter.
//! - Buggy: terminal will print escape sequences like `;129u`
//! - Fixed: terminal works normally

use crossterm::{
    event::{
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("buggy");

    match mode {
        "buggy" => test_buggy_order()?,
        "fixed" => test_fixed_order()?,
        _ => {
            eprintln!("Usage: {} [buggy|fixed]", args[0]);
            eprintln!("  buggy - Test current (broken) order");
            eprintln!("  fixed - Test correct order");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Current Fresh behavior - keyboard enhancement pushed BEFORE alternate screen
fn test_buggy_order() -> std::io::Result<()> {
    println!("Testing BUGGY order (current Fresh behavior)");
    println!("After exit, try arrow keys - they should be broken");
    println!("Press Enter to start...");
    let _ = std::io::stdin().read_line(&mut String::new());

    let mut stdout = stdout();

    // Step 1: Enable raw mode
    enable_raw_mode()?;

    // Step 2: Push keyboard enhancement (to MAIN screen stack) - BUG!
    let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
    stdout.execute(PushKeyboardEnhancementFlags(flags))?;

    // Step 3: Enter alternate screen
    stdout.execute(EnterAlternateScreen)?;

    // Simulate app running
    print!("On alternate screen. Waiting 2 seconds...\r\n");
    stdout.flush()?;
    sleep(Duration::from_secs(2));

    // Step 4: Pop keyboard enhancement (from ALTERNATE screen stack - wrong!)
    stdout.execute(PopKeyboardEnhancementFlags)?;

    // Step 5: Disable raw mode
    disable_raw_mode()?;

    // Step 6: Leave alternate screen (main screen still has kb enhancement!)
    stdout.execute(LeaveAlternateScreen)?;

    stdout.flush()?;

    println!("\nExited. Try pressing arrow keys or Enter.");
    println!("If broken, run 'reset' to fix.");

    Ok(())
}

/// Fixed behavior - keyboard enhancement pushed AFTER alternate screen
fn test_fixed_order() -> std::io::Result<()> {
    println!("Testing FIXED order");
    println!("After exit, arrow keys should work normally");
    println!("Press Enter to start...");
    let _ = std::io::stdin().read_line(&mut String::new());

    let mut stdout = stdout();

    // Step 1: Enable raw mode
    enable_raw_mode()?;

    // Step 2: Enter alternate screen FIRST
    stdout.execute(EnterAlternateScreen)?;

    // Step 3: Push keyboard enhancement (to ALTERNATE screen stack) - correct!
    let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
    stdout.execute(PushKeyboardEnhancementFlags(flags))?;

    // Simulate app running
    print!("On alternate screen. Waiting 2 seconds...\r\n");
    stdout.flush()?;
    sleep(Duration::from_secs(2));

    // Step 4: Pop keyboard enhancement (from ALTERNATE screen stack - correct!)
    stdout.execute(PopKeyboardEnhancementFlags)?;

    // Step 5: Disable raw mode
    disable_raw_mode()?;

    // Step 6: Leave alternate screen (main screen was never modified!)
    stdout.execute(LeaveAlternateScreen)?;

    stdout.flush()?;

    println!("\nExited. Arrow keys should work normally.");

    Ok(())
}
