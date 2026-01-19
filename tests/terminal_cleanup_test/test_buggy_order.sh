#!/bin/bash
# Test script demonstrating the BUGGY order (current Fresh behavior)
# Run this in Kitty/Ghostty, then try typing after it exits
#
# Current (buggy) order:
# 1. Push keyboard enhancement (to main screen stack)
# 2. Enter alternate screen
# 3. [app runs]
# 4. Pop keyboard enhancement (from alternate screen stack - wrong!)
# 5. Leave alternate screen (main screen still has keyboard enhancement!)

echo "Testing BUGGY order (current Fresh behavior)"
echo "After this script exits, try pressing arrow keys or Enter"
echo "If broken: you'll see escape sequences like ;129u or 27;5u"
echo ""
echo "Press Enter to start test..."
read

# Step 1: Push keyboard enhancement flags (DISAMBIGUATE_ESCAPE_CODES = 1)
# This goes to the MAIN screen's stack
printf '\x1b[>1u'

# Step 2: Enter alternate screen
printf '\x1b[?1049h'

# Show we're on alternate screen
echo "Now on alternate screen with keyboard enhancement"
echo "Sleeping 2 seconds..."
sleep 2

# Step 3: Pop keyboard enhancement (this pops from ALTERNATE screen stack!)
printf '\x1b[<1u'

# Step 4: Leave alternate screen (main screen still has kb enhancement pushed!)
printf '\x1b[?1049l'

echo ""
echo "Exited. Now try pressing arrow keys or Enter."
echo "If the terminal is broken, run 'reset' to fix it."
