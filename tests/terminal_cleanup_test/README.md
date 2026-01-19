# Terminal Cleanup Test

This test verifies the root cause of issue #773: terminal input state not restored on exit.

## The Bug

The keyboard enhancement flags are pushed **before** entering alternate screen, but popped
**before** leaving alternate screen. According to the Kitty keyboard protocol, each screen
maintains its own independent keyboard mode stack. This means:

1. Push keyboard enhancement → goes to **main screen's** stack
2. Enter alternate screen → switches to alternate screen (separate stack)
3. Pop keyboard enhancement → pops from **alternate screen's** stack (empty/wrong!)
4. Leave alternate screen → back to main screen, **keyboard enhancement still pushed!**

## Running the Tests

### Shell Scripts (no compilation needed)

```bash
# Make executable
chmod +x test_buggy_order.sh test_fixed_order.sh

# Test buggy behavior (current Fresh) - terminal will break after exit
./test_buggy_order.sh

# Test fixed behavior - terminal works normally after exit
./test_fixed_order.sh
```

### Rust Binary (uses crossterm like Fresh)

```bash
# Build
cargo build --release

# Test buggy behavior
./target/release/terminal_cleanup_test buggy

# Test fixed behavior
./target/release/terminal_cleanup_test fixed
```

## Expected Results

**Run in Kitty, Ghostty, or WezTerm** (terminals supporting Kitty keyboard protocol)

After `buggy` test:
- Arrow keys print escape sequences like `;129u` or `27;5u`
- Enter key may not work (produces `13;1u` or similar)
- Run `reset` to fix the terminal

After `fixed` test:
- Arrow keys work normally
- Enter key works normally
- No escape sequences printed

## The Fix

In `terminal_modes.rs`, change the order so keyboard enhancement is:
1. Pushed **after** entering alternate screen
2. Popped **before** leaving alternate screen

This ensures we operate on the alternate screen's stack, leaving the main screen untouched.
