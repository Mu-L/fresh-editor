# Clean Architecture for Composite Buffer Rendering

## Pipeline Overview

```
Normal Buffer:
  EditorState.buffer
    ↓ line_iterator(top_byte) - iterate from byte offset
  Lines (byte stream)
    ↓ build_base_tokens()
  Vec<ViewTokenWire>
    ↓ ViewLineIterator
  Vec<ViewLine>
    ↓ render_view_lines()
  Screen

Composite Buffer (PROPOSED):
  Source EditorStates (per pane)
    ↓ buffer.get_line(line_num) - access by line number
  Line content (per source line)
    ↓ build_view_line_from_line()
  ViewLine (per source line)
    ↓ render_styled_view_line()
  Screen (per pane, per row)
```

## Key Insight

The existing pipeline uses **byte offsets** everywhere:
- `top_byte` in Viewport
- `source_offset` in ViewTokenWire
- `char_source_bytes` in ViewLine
- Highlight spans use byte ranges

But for composite buffers, we work with **line numbers** and **alignment**:
- `scroll_row` (display row)
- `alignment.rows[display_row].get_pane_line(pane_idx)` → source line number
- Per-pane cursors by (row, column)

## Solution: Bridge Line Numbers to ViewLines

### Step 1: Build ViewLine from Source Line

```rust
/// Build a ViewLine from a buffer line (by line number)
fn build_view_line_from_line(
    buffer: &Buffer,
    line_num: usize,
    tab_size: usize,
) -> Option<ViewLine> {
    let line_content = buffer.get_line(line_num)?;
    let line_start = buffer.line_start_offset(line_num)?;

    // Build token for this line
    let text = String::from_utf8_lossy(&line_content);
    let text = text.trim_end_matches('\n').trim_end_matches('\r');

    let tokens = vec![ViewTokenWire {
        source_offset: Some(line_start),  // Absolute byte offset
        kind: ViewTokenWireKind::Text(text.to_string()),
        style: None,
    }];

    ViewLineIterator::new(&tokens, false, true, tab_size).next()
}
```

### Step 2: Get Highlight Spans for Source Line

```rust
/// Get syntax highlight spans for a source line (adjusted to line-relative offsets)
fn get_line_highlights(
    state: &EditorState,
    line_num: usize,
) -> Vec<HighlightSpan> {
    let line_start = state.buffer.line_start_offset(line_num).unwrap_or(0);
    let line_end = state.buffer.line_start_offset(line_num + 1)
        .unwrap_or_else(|| state.buffer.len());

    // Get spans from highlighter
    state.highlighter.get_highlight_spans_in_range(line_start, line_end)
        .into_iter()
        .map(|mut span| {
            // Keep absolute offsets - compute_char_style uses them
            span
        })
        .collect()
}
```

### Step 3: Render ViewLine with Styling

This is where we can **directly reuse compute_char_style()**:

```rust
/// Render a ViewLine for a composite buffer pane
fn render_composite_view_line(
    view_line: &ViewLine,
    theme: &Theme,
    highlight_spans: &[HighlightSpan],
    left_column: usize,
    cursor_column: Option<usize>,  // None if not cursor row
    background_override: Option<Color>,  // Diff coloring
    line_number: Option<usize>,
    gutter_width: usize,
    is_active: bool,
) -> Line<'static> {
    let mut spans = Vec::new();

    // Render gutter (line number)
    if let Some(num) = line_number {
        let num_str = format!("{:>width$} ", num, width = gutter_width - 1);
        spans.push(Span::styled(num_str, Style::default().fg(theme.line_number_fg)));
    } else {
        spans.push(Span::styled(" ".repeat(gutter_width), Style::default()));
    }

    // Render content with styling
    let text_chars: Vec<char> = view_line.text.chars().collect();

    for (char_idx, ch) in text_chars.iter().enumerate().skip(left_column) {
        let byte_pos = view_line.char_source_bytes.get(char_idx).copied().flatten();

        // Compute style using EXISTING compute_char_style logic
        let mut style = compute_base_style(byte_pos, highlight_spans, theme);

        // Apply background override (diff coloring)
        if let Some(bg) = background_override {
            style = style.bg(bg);
        }

        // Apply cursor styling
        if cursor_column == Some(char_idx) {
            style = Style::default().fg(theme.editor_bg).bg(theme.editor_fg);
        }

        spans.push(Span::styled(ch.to_string(), style));
    }

    Line::from(spans)
}
```

### Step 4: Integrate into render_composite_buffer()

```rust
fn render_composite_buffer(
    frame: &mut Frame,
    area: Rect,
    composite: &CompositeBuffer,
    buffers: &HashMap<BufferId, EditorState>,
    theme: &Theme,
    is_active: bool,
    view_state: &CompositeViewState,
) {
    // ... layout calculation ...

    for view_row in 0..visible_rows {
        let display_row = view_state.scroll_row + view_row;
        let aligned_row = &composite.alignment.rows[display_row];
        let is_cursor_row = display_row == view_state.cursor_row;

        // Get diff background
        let row_bg = match aligned_row.row_type {
            RowType::Addition => Some(theme.diff_add_bg),
            RowType::Deletion => Some(theme.diff_remove_bg),
            RowType::Modification => Some(theme.diff_modify_bg),
            _ => None,
        };

        for (pane_idx, (source, &width)) in composite.sources.iter().zip(&pane_widths).enumerate() {
            let pane_area = Rect::new(x_offset, content_y + view_row as u16, width, 1);
            let left_column = view_state.get_pane_viewport(pane_idx)
                .map(|v| v.left_column).unwrap_or(0);

            let is_focused_pane = pane_idx == view_state.focused_pane;
            let cursor_col = if is_cursor_row && is_focused_pane {
                Some(view_state.cursor_column)
            } else {
                None
            };

            if let Some(source_line_ref) = aligned_row.get_pane_line(pane_idx) {
                if let Some(source_state) = buffers.get(&source.buffer_id) {
                    // Build ViewLine from source line
                    let view_line = build_view_line_from_line(
                        &source_state.buffer,
                        source_line_ref.line,
                        source_state.tab_size,
                    );

                    if let Some(vl) = view_line {
                        // Get syntax highlighting
                        let highlights = get_line_highlights(source_state, source_line_ref.line);

                        // Render with full styling
                        let rendered = render_composite_view_line(
                            &vl,
                            theme,
                            &highlights,
                            left_column,
                            cursor_col,
                            row_bg,
                            Some(source_line_ref.line + 1),
                            GUTTER_WIDTH,
                            is_active,
                        );

                        let para = Paragraph::new(rendered);
                        frame.render_widget(para, pane_area);
                    }
                }
            }

            x_offset += width + separator_width;
        }
    }
}
```

## What This Reuses

| Component | Reused? | How |
|-----------|---------|-----|
| ViewLineIterator | ✅ | Build ViewLine from single line's tokens |
| ViewLine structure | ✅ | Same struct with char mappings |
| compute_char_style logic | ✅ | Same style layering for syntax highlighting |
| Highlighter | ✅ | Query spans by byte range |
| Theme | ✅ | Same colors and styling |
| Span/Line/Paragraph | ✅ | Same ratatui widgets |

## What's New/Different

1. **build_view_line_from_line()** - Builds ViewLine from line number instead of byte stream
2. **get_line_highlights()** - Queries highlighter for a single line's range
3. **render_composite_view_line()** - Simplified rendering that handles:
   - Line number display
   - Horizontal scrolling
   - Cursor (by column, not byte)
   - Diff backgrounds
   - Syntax highlighting

## Benefits

1. **Minimal duplication** - Reuses ViewLineIterator, highlighting, theming
2. **Clean abstraction** - Line-based API vs byte-based API
3. **Incremental** - Can add features (selection, semantic highlighting) later
4. **Testable** - Each function is standalone and testable

## Implementation Order

1. Add `build_view_line_from_line()` helper
2. Add `get_line_highlights()` helper
3. Replace current `render_composite_buffer()` content rendering with new approach
4. Verify syntax highlighting works
5. Add semantic highlighting support
6. Add selection rendering
