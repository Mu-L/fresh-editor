#!/bin/bash
# Fresh Editor Screenshot Tool (X11 version)
# Captures screenshots using xfce4-terminal + tmux + import (ImageMagick)
# Usage: ./screenshot-x11.sh [command]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FRESH_BIN="${FRESH_BIN:-fresh}"
SAMPLE_DIR="$SCRIPT_DIR/samples"
OUTPUT_DIR="$SCRIPT_DIR/output"
ASSETS_DIR="$REPO_ROOT/docs/assets"
SESSION_NAME="fresh-x11-screenshot"

# Default terminal size (columns x rows)
WIDTH=120
HEIGHT=35

source "$SCRIPT_DIR/shots.sh"

mkdir -p "$OUTPUT_DIR" "$SAMPLE_DIR" "$ASSETS_DIR"

# ============================================================================
# Key sequence sender (tmux)
# ============================================================================

send_keys() {
    local sequence="$1"
    [ -z "$sequence" ] && return

    IFS=':' read -ra PARTS <<< "$sequence"
    for item in "${PARTS[@]}"; do
        if [[ "$item" =~ ^[0-9]*\.?[0-9]+$ ]]; then
            sleep "$item"
        else
            local keys="$item"
            case "$keys" in
                ENTER) keys="Enter" ;;
                ESC) keys="Escape" ;;
                TAB) keys="Tab" ;;
                UP) keys="Up" ;;
                DOWN) keys="Down" ;;
                LEFT) keys="Left" ;;
                RIGHT) keys="Right" ;;
                SPACE) keys="Space" ;;
                BACKSPACE) keys="BSpace" ;;
                DELETE) keys="DC" ;;
                HOME) keys="Home" ;;
                END) keys="End" ;;
                PAGEUP) keys="PPage" ;;
                PAGEDOWN) keys="NPage" ;;
                C-*) keys="C-${keys#C-}" ;;
                M-*) keys="M-${keys#M-}" ;;
                S-*) keys="S-${keys#S-}" ;;
            esac
            tmux send-keys -t "$SESSION_NAME" "$keys"
        fi
    done
}

# ============================================================================
# Capture
# ============================================================================

capture() {
    local name="$1"
    local file="$2"
    local w="${3:-$WIDTH}"
    local h="${4:-$HEIGHT}"
    local keys="$5"

    local png_file="$OUTPUT_DIR/${name}.png"

    echo -n "Capturing $name... "

    # Create a fresh copy of the file to avoid contamination between tests
    local temp_dir=$(mktemp -d)
    local temp_file="$temp_dir/$(basename "$file")"
    cp "$file" "$temp_file"

    # Kill any existing session and clear recovery data
    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
    rm -rf ~/.local/share/fresh/recovery/

    local window_title="FRESH_SCREENSHOT_$$"

    # Launch terminal with unique title running tmux
    xfce4-terminal --geometry="${w}x${h}" --hide-menubar --hide-toolbar \
        --title="$window_title" \
        --command="tmux new-session -s $SESSION_NAME -x $w -y $h" &
    local term_pid=$!

    sleep 0.8

    # Turn off status bar and launch fresh (--no-session to start clean)
    tmux set-option -t "$SESSION_NAME" status off 2>/dev/null || true
    tmux send-keys -t "$SESSION_NAME" "$FRESH_BIN --no-session $temp_file" Enter

    sleep 1.5

    # Find the window by unique title
    local wid=$(wmctrl -l | grep "$window_title" | awk '{print $1}')

    if [ -z "$wid" ]; then
        echo "FAILED (window not found)"
        tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
        kill $term_pid 2>/dev/null || true
        return 1
    fi

    sleep 0.3

    [ -n "$keys" ] && send_keys "$keys"

    import -window "$wid" "$png_file"

    # Close tmux session (this kills fresh) and terminal
    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
    sleep 0.2
    kill $term_pid 2>/dev/null || true
    wait $term_pid 2>/dev/null || true

    # Clean up temp file
    rm -rf "$temp_dir"

    echo "-> $png_file"
}

# ============================================================================
# Main
# ============================================================================

capture_by_name() {
    local target="$1"
    for shot in "${SHOTS[@]}"; do
        IFS=':' read -r name file w h keys <<< "$shot"
        if [ "$name" = "$target" ]; then
            capture "$name" "$SAMPLE_DIR/$file" "$w" "$h" "$keys"
            return 0
        fi
    done
    echo "Unknown screenshot: $target"
    return 1
}

capture_all() {
    echo "Capturing all screenshots..."
    echo ""
    for shot in "${SHOTS[@]}"; do
        IFS=':' read -r name file w h keys <<< "$shot"
        capture "$name" "$SAMPLE_DIR/$file" "$w" "$h" "$keys"
    done
}

copy_to_assets() {
    echo ""
    echo "Copying to assets..."
    for f in "$OUTPUT_DIR"/*.png; do
        [ -f "$f" ] || continue
        cp "$f" "$ASSETS_DIR/"
        echo "  $(basename "$f")"
    done
}

show_help() {
    cat << 'EOF'
Fresh Editor Screenshot Tool (X11)

Uses xfce4-terminal + xdotool + import for pixel-perfect screenshots.

Usage: ./screenshot-x11.sh [command]

Commands:
  all              Capture all screenshots and copy to assets
  hero             Main hero screenshot (Rust code)
  typescript       TypeScript plugin example
  python           Python code example
  json             JSON config example
  explorer         File explorer (Ctrl+E)
  search           Search dialog (Ctrl+F)
  replace          Replace dialog (Ctrl+R)
  goto             Go to line (Ctrl+G)
  manual           Show manual (F1)
  shortcuts        Show keyboard shortcuts (Shift+F1)
  samples          Just create sample files
  custom NAME FILE [W] [H] [KEYS]

Requirements: xfce4-terminal, xdotool, imagemagick

Examples:
  ./screenshot-x11.sh hero
  ./screenshot-x11.sh all
EOF
}

check_deps() {
    local missing=()
    command -v xfce4-terminal &>/dev/null || missing+=("xfce4-terminal")
    command -v xdotool &>/dev/null || missing+=("xdotool")
    command -v import &>/dev/null || missing+=("imagemagick")

    if [ ${#missing[@]} -gt 0 ]; then
        echo "Missing dependencies: ${missing[*]}"
        exit 1
    fi
}

main() {
    check_deps

    case "${1:-help}" in
        help|--help|-h) show_help ;;
        all)
            create_samples "$SAMPLE_DIR"
            capture_all
            copy_to_assets
            echo ""
            echo "Done! Screenshots in: $ASSETS_DIR/"
            ;;
        samples)
            create_samples "$SAMPLE_DIR"
            echo "Sample files created in $SAMPLE_DIR"
            ;;
        custom)
            shift
            [ $# -lt 2 ] && { echo "Usage: $0 custom NAME FILE [W] [H] [KEYS]"; exit 1; }
            create_samples "$SAMPLE_DIR"
            capture "$1" "$2" "${3:-$WIDTH}" "${4:-$HEIGHT}" "${5:-}"
            ;;
        *)
            create_samples "$SAMPLE_DIR"
            capture_by_name "$1"
            ;;
    esac
}

main "$@"
