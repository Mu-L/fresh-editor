#!/bin/bash
# Diagnostic script to check if keyboard enhancement is still enabled
# Run this AFTER exiting Fresh to diagnose the issue
#
# If keyboard enhancement is left enabled, pressing keys will produce
# CSI-u sequences instead of normal key codes.

echo "Keyboard State Diagnostic"
echo "========================="
echo ""
echo "This will read a few keystrokes and show what sequences are received."
echo "If keyboard enhancement is LEFT ENABLED (bug), you'll see:"
echo "  - Arrow keys: sequences like '[1;1A' or '[A' with extra data"
echo "  - Enter: '13;1u' or similar"
echo "  - Letters with Ctrl: '3;5u' (Ctrl+C) instead of raw ^C"
echo ""
echo "If keyboard enhancement is properly DISABLED, you'll see:"
echo "  - Arrow keys: normal '[A', '[B', '[C', '[D'"
echo "  - Enter: empty or just newline"
echo "  - Ctrl+C: exits the script"
echo ""
echo "Press some keys (Ctrl+C to exit):"
echo ""

# Read raw input and display what we get
stty raw -echo
trap 'stty sane; echo; echo "Done."; exit' INT

while true; do
    # Read one character at a time with timeout
    char=$(dd bs=1 count=1 2>/dev/null | xxd -p)
    if [ -n "$char" ]; then
        # Convert hex to readable format
        case "$char" in
            1b) printf "[ESC]" ;;
            0d) printf "[CR]" ;;
            0a) printf "[LF]" ;;
            7f) printf "[DEL]" ;;
            *)
                # Try to print as ASCII if printable
                dec=$((16#$char))
                if [ $dec -ge 32 ] && [ $dec -le 126 ]; then
                    printf "\\x$char"
                else
                    printf "[0x$char]"
                fi
                ;;
        esac
    fi
done
