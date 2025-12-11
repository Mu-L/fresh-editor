#!/bin/bash
# Fresh Editor Screenshot Tool (tmux + agg)
# Captures screenshots using tmux + agg (asciinema gif generator)
# Usage: ./screenshot.sh [command]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FRESH_BIN="${FRESH_BIN:-fresh}"
SAMPLE_DIR="$SCRIPT_DIR/samples"
OUTPUT_DIR="$SCRIPT_DIR/output"
ASSETS_DIR="$REPO_ROOT/docs/assets"
SESSION_NAME="fresh-screenshot"

# Default terminal size
WIDTH=120
HEIGHT=35

# Rendering settings
FONT_SIZE=14
FONT_FAMILY="Noto Sans Mono"
LINE_HEIGHT=1.18
AGG_THEME="000000,ffffff,000000,cd0000,00cd00,cdcd00,0000ee,cd00cd,00cdcd,e5e5e5,7f7f7f,ff0000,00ff00,ffff00,5c5cff,ff00ff,00ffff,ffffff"

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
                CTRL-*) keys="C-${keys#CTRL-}" ;;
                ALT-*) keys="M-${keys#ALT-}" ;;
            esac
            tmux send-keys -t "$SESSION_NAME" "$keys"
        fi
    done
}

# ============================================================================
# Capture and render
# ============================================================================

render() {
    local name="$1"
    local ansi_file="$OUTPUT_DIR/${name}.ansi"
    local cast_file="$OUTPUT_DIR/${name}.cast"
    local gif_file="$OUTPUT_DIR/${name}.gif"
    local png_file="$OUTPUT_DIR/${name}.png"

    tmux capture-pane -t "$SESSION_NAME" -e -p > "$ansi_file"

    local cols=$(tmux display -t "$SESSION_NAME" -p '#{pane_width}')
    local rows=$(tmux display -t "$SESSION_NAME" -p '#{pane_height}')

    {
        echo "{\"version\": 2, \"width\": $cols, \"height\": $rows}"
        python3 -c "
import json, sys
lines = sys.stdin.readlines()
content = ''.join(f'\x1b[{i+1};1H' + line.rstrip('\n') for i, line in enumerate(lines))
print(json.dumps([0.0, 'o', content]))
" < "$ansi_file"
    } > "$cast_file"

    agg --font-family "$FONT_FAMILY" --font-size "$FONT_SIZE" --line-height "$LINE_HEIGHT" \
        --theme "$AGG_THEME" --last-frame-duration 0 --no-loop \
        "$cast_file" "$gif_file" 2>/dev/null

    convert "${gif_file}[0]" "$png_file" 2>/dev/null

    echo "  -> $png_file"
}

capture() {
    local name="$1"
    local file="$2"
    local w="${3:-$WIDTH}"
    local h="${4:-$HEIGHT}"
    local keys="$5"

    echo -n "Capturing $name... "

    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
    sleep 0.2
    tmux new-session -d -s "$SESSION_NAME" -x "$w" -y "$h"
    sleep 0.3

    tmux send-keys -t "$SESSION_NAME" "$FRESH_BIN --no-session $file" Enter
    sleep 1.5

    [ -n "$keys" ] && send_keys "$keys"

    render "$name"

    tmux send-keys -t "$SESSION_NAME" C-q
    sleep 0.3
    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
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
Fresh Editor Screenshot Tool (tmux + agg)

Usage: ./screenshot.sh [command]

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

Key sequence format: keys:delay:keys:delay:...

Examples:
  ./screenshot.sh hero
  ./screenshot.sh all
EOF
}

main() {
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
