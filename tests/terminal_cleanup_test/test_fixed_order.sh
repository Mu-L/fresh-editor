#!/bin/bash
# Test script demonstrating the FIXED order
# Run this in Kitty/Ghostty - terminal should work correctly after exit
#
# Fixed order:
# 1. Enter alternate screen
# 2. Push keyboard enhancement (to alternate screen stack)
# 3. [app runs]
# 4. Pop keyboard enhancement (from alternate screen stack - correct!)
# 5. Leave alternate screen (main screen was never modified)

echo "Testing FIXED order"
echo "After this script exits, arrow keys and Enter should work normally"
echo ""
echo "Press Enter to start test..."
read

# Step 1: Enter alternate screen FIRST
printf '\x1b[?1049h'

# Step 2: Push keyboard enhancement flags (to ALTERNATE screen's stack)
printf '\x1b[>1u'

# Show we're on alternate screen
echo "Now on alternate screen with keyboard enhancement"
echo "Sleeping 2 seconds..."
sleep 2

# Step 3: Pop keyboard enhancement (from ALTERNATE screen stack - correct!)
printf '\x1b[<1u'

# Step 4: Leave alternate screen (main screen was never touched)
printf '\x1b[?1049l'

echo ""
echo "Exited. Arrow keys and Enter should work normally."
