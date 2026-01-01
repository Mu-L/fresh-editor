# Composite Buffer Architecture

## Overview

CompositeBuffer is a **thin coordination layer** that wraps multiple normal buffers into a unified view. Each pane is a full `EditorState` with its own viewport, cursors, and rendering. The composite layer only handles:

1. **Visual arrangement** - laying out panes side-by-side
2. **Scroll synchronization** - keeping panes aligned via chunk-based alignment
3. **Input routing** - directing cursor/edit actions to the focused pane
4. **Diff highlighting** - applying background overlays based on alignment

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CompositeBuffer                         │
│                                                              │
│  ┌──────────────────┐        ┌──────────────────┐           │
│  │     Pane 0       │        │     Pane 1       │           │
│  │  ┌────────────┐  │        │  ┌────────────┐  │           │
│  │  │EditorState │  │        │  │EditorState │  │           │
│  │  │  - buffer  │  │        │  │  - buffer  │  │           │
│  │  │  - cursors │  │        │  │  - cursors │  │           │
│  │  │  - highlight│ │        │  │  - highlight│ │           │
│  │  │  - overlays │  │        │  │  - overlays │  │           │
│  │  ├────────────┤  │        │  ├────────────┤  │           │
│  │  │  Viewport  │  │        │  │  Viewport  │  │           │
│  │  │ (derived)  │  │        │  │ (derived)  │  │           │
│  │  └────────────┘  │        │  └────────────┘  │           │
│  └──────────────────┘        └──────────────────┘           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                   ChunkAlignment                        │ │
│  │  chunks: [Context, Hunk, Context, Hunk, Context, ...]  │ │
│  │  (markers at chunk boundaries for edit-robustness)     │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  scroll_display_row: usize   (unified scroll position)      │
│  focused_pane: usize         (which pane receives input)    │
└─────────────────────────────────────────────────────────────┘
```

## Key Principle: Full Reuse

Each pane renders using the **existing normal buffer rendering pipeline**:

```
Pane's EditorState
    ↓ build_view_data()
ViewLines
    ↓ render_view_lines()
Screen
```

All features work automatically:
- Syntax highlighting
- Selection & multiple cursors
- Line wrapping
- ANSI escape handling
- Virtual text (code lens, diagnostics)
- Semantic highlighting
- Scrollbar
- Mouse support

The composite layer just:
1. Computes each pane's `Viewport.top_byte` from the unified scroll position
2. Calls the existing rendering for each pane's area
3. Adds diff background overlays

## Chunk-Based Alignment with Markers

### Why Markers at Chunk Boundaries?

Traditional alignment stores line numbers, which break on edit:
```rust
// FRAGILE: line numbers become wrong after insert/delete
struct AlignedRow {
    old_line: usize,  // Breaks if lines inserted above
    new_line: usize,
}
```

Instead, we use **markers at chunk boundaries**:
```rust
// ROBUST: markers auto-adjust on edit
struct AlignmentChunk {
    start_markers: Vec<Option<MarkerId>>,  // One per pane
    kind: ChunkKind,
}
```

### Data Structures

```rust
struct ChunkAlignment {
    chunks: Vec<AlignmentChunk>,
}

struct AlignmentChunk {
    /// Marker at the START of this chunk in each pane
    /// None if this pane has no content at this chunk (e.g., pure insertion)
    start_markers: Vec<Option<MarkerId>>,

    /// What kind of chunk this is
    kind: ChunkKind,

    /// Whether this chunk needs recomputation after an edit
    dirty: bool,
}

enum ChunkKind {
    /// Unchanged lines - same content in all panes
    Context {
        line_count: usize,
    },

    /// Changed region - diff operations within
    Hunk {
        /// Sequence of (old_lines, new_lines) pairs:
        /// (1, 0) = deletion (1 old line, 0 new lines)
        /// (0, 1) = insertion (0 old lines, 1 new line)
        /// (1, 1) = modification (1 old line maps to 1 new line)
        /// (2, 3) = 2 old lines replaced by 3 new lines
        ops: Vec<(usize, usize)>,
    },
}
```

### Example

Given this diff:
```
  Line 1    |   Line 1      (context)
  Line 2    |   Line 2      (context)
- Line 3    |               (deletion)
- Line 4    |               (deletion)
            |+  New 3       (insertion)
  Line 5    |   Line 5      (context)
```

The alignment becomes:
```rust
chunks: [
    AlignmentChunk {
        start_markers: [M0_old, M0_new],  // Markers at "Line 1"
        kind: Context { line_count: 2 },
    },
    AlignmentChunk {
        start_markers: [M1_old, M1_new],  // Markers at "Line 3" / "New 3"
        kind: Hunk {
            ops: [(1, 0), (1, 0), (0, 1)]  // del, del, ins
        },
    },
    AlignmentChunk {
        start_markers: [M2_old, M2_new],  // Markers at "Line 5"
        kind: Context { line_count: 1 },
    },
]
```

**Total: 6 markers** (2 per chunk) instead of one per line.

### Converting to Display Rows

```rust
impl ChunkAlignment {
    fn to_display_rows(&self, pane_buffers: &[&Buffer]) -> Vec<AlignedRow> {
        let mut rows = Vec::new();

        for chunk in &self.chunks {
            // Resolve markers to current line numbers
            let start_lines: Vec<Option<usize>> = chunk.start_markers
                .iter()
                .enumerate()
                .map(|(pane_idx, marker_opt)| {
                    marker_opt.and_then(|m| pane_buffers[pane_idx].marker_to_line(m))
                })
                .collect();

            match &chunk.kind {
                ChunkKind::Context { line_count } => {
                    for offset in 0..*line_count {
                        rows.push(AlignedRow {
                            pane_lines: start_lines.iter()
                                .map(|opt| opt.map(|l| l + offset))
                                .collect(),
                            row_type: RowType::Context,
                        });
                    }
                }

                ChunkKind::Hunk { ops } => {
                    let mut offsets = vec![0usize; start_lines.len()];

                    for &(old_n, new_n) in ops {
                        let max_n = old_n.max(new_n);
                        for i in 0..max_n {
                            let old_line = if i < old_n {
                                start_lines[0].map(|l| l + offsets[0] + i)
                            } else { None };
                            let new_line = if i < new_n {
                                start_lines[1].map(|l| l + offsets[1] + i)
                            } else { None };

                            rows.push(AlignedRow {
                                pane_lines: vec![old_line, new_line],
                                row_type: match (old_line, new_line) {
                                    (Some(_), None) => RowType::Deletion,
                                    (None, Some(_)) => RowType::Addition,
                                    (Some(_), Some(_)) => RowType::Modification,
                                    (None, None) => continue,
                                },
                            });
                        }
                        offsets[0] += old_n;
                        offsets[1] += new_n;
                    }
                }
            }
        }

        rows
    }
}
```

### Edit Handling

```rust
impl ChunkAlignment {
    fn on_buffer_edit(
        &mut self,
        pane_idx: usize,
        edit_start_line: usize,
        lines_inserted: isize,  // positive = insert, negative = delete
    ) {
        // Markers auto-adjust their byte positions - no action needed for them

        // Find which chunk contains the edit
        for chunk in &mut self.chunks {
            let chunk_start = chunk.current_start_line(pane_idx);
            let chunk_end = chunk.current_end_line(pane_idx);

            if let (Some(start), Some(end)) = (chunk_start, chunk_end) {
                if edit_start_line >= start && edit_start_line < end {
                    match &mut chunk.kind {
                        ChunkKind::Context { line_count } => {
                            // Edit within context: just adjust the count
                            *line_count = (*line_count as isize + lines_inserted) as usize;
                        }
                        ChunkKind::Hunk { .. } => {
                            // Edit within hunk: mark dirty for recomputation
                            chunk.dirty = true;
                        }
                    }
                    return;
                }
            }
        }
    }

    fn recompute_dirty_chunks(&mut self, pane_buffers: &[&Buffer]) {
        for chunk in &mut self.chunks {
            if chunk.dirty {
                // Extract the text range for this chunk in each pane
                // Run diff algorithm on just this region
                // Update chunk.kind with new ops
                chunk.dirty = false;
            }
        }
    }
}
```

## Scroll Synchronization

The composite buffer has a unified `scroll_display_row`. Each pane's `Viewport.top_byte` is derived:

```rust
impl CompositeBuffer {
    fn derive_pane_viewport(
        &self,
        pane_idx: usize,
        pane_buffer: &Buffer,
        display_row: usize,
    ) -> Viewport {
        // Convert display_row to source line for this pane
        let display_rows = self.alignment.to_display_rows(&self.pane_buffers);

        let source_line = display_rows
            .get(display_row)
            .and_then(|row| row.pane_lines.get(pane_idx))
            .flatten();

        let top_byte = source_line
            .and_then(|line| pane_buffer.line_start_offset(line))
            .unwrap_or(0);

        Viewport {
            top_byte,
            left_column: self.pane_viewports[pane_idx].left_column,
            ..Default::default()
        }
    }
}
```

## Input Routing

Cursor and edit actions go to the focused pane's EditorState:

```rust
impl CompositeBuffer {
    fn handle_action(&mut self, action: Action) -> Option<Event> {
        match action {
            // Navigation between panes
            Action::FocusNextPane => {
                self.focused_pane = (self.focused_pane + 1) % self.panes.len();
                None
            }

            // Vertical movement updates unified scroll
            Action::CursorDown => {
                self.scroll_display_row += 1;
                // Also move cursor in focused pane
                self.focused_pane_state_mut().handle_cursor_down()
            }

            // Horizontal movement / editing goes to focused pane
            Action::CursorRight | Action::Insert(_) | Action::Delete => {
                self.focused_pane_state_mut().handle_action(action)
            }

            // Mouse clicks determine focus
            Action::MouseClick { x, y } => {
                let pane_idx = self.pane_at_position(x, y);
                self.focused_pane = pane_idx;
                self.panes[pane_idx].handle_mouse_click(x, y)
            }

            _ => None
        }
    }
}
```

## Diff Highlighting via Overlays

Instead of custom rendering, add overlays to each pane's EditorState:

```rust
impl CompositeBuffer {
    fn apply_diff_overlays(&mut self, theme: &Theme) {
        let display_rows = self.alignment.to_display_rows(&self.pane_buffers);

        for (pane_idx, pane_state) in self.pane_states.iter_mut().enumerate() {
            // Clear previous diff overlays
            pane_state.overlays.clear_category("diff");

            for row in &display_rows {
                if let Some(source_line) = row.pane_lines.get(pane_idx).flatten() {
                    let bg_color = match row.row_type {
                        RowType::Addition => Some(theme.diff_add_bg),
                        RowType::Deletion => Some(theme.diff_remove_bg),
                        RowType::Modification => Some(theme.diff_modify_bg),
                        _ => None,
                    };

                    if let Some(color) = bg_color {
                        let line_range = pane_state.buffer.line_byte_range(source_line);
                        pane_state.overlays.add(Overlay {
                            range: line_range,
                            face: OverlayFace::Background { color },
                            category: "diff".to_string(),
                        });
                    }
                }
            }
        }
    }
}
```

## Rendering

```rust
impl CompositeBuffer {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let pane_rects = self.compute_pane_rects(area);

        for (pane_idx, pane_rect) in pane_rects.iter().enumerate() {
            // Derive viewport from unified scroll position
            let viewport = self.derive_pane_viewport(
                pane_idx,
                &self.pane_states[pane_idx].buffer,
                self.scroll_display_row,
            );

            // Use EXISTING normal buffer rendering!
            render_buffer_in_split(
                frame,
                *pane_rect,
                &self.pane_states[pane_idx],
                &viewport,
                theme,
                pane_idx == self.focused_pane,  // is_active
            );
        }

        // Render separators between panes
        self.render_separators(frame, &pane_rects, theme);
    }
}
```

## Summary

| Aspect | Design |
|--------|--------|
| **Pane rendering** | Full reuse of normal buffer rendering |
| **Features** | All automatic (syntax, selection, wrapping, etc.) |
| **Alignment storage** | Chunks with markers at boundaries |
| **Edit robustness** | Markers auto-adjust; context chunks update count; hunks marked dirty |
| **Scroll sync** | Unified display_row → per-pane top_byte via alignment |
| **Diff highlighting** | Overlays on pane EditorStates |
| **Input handling** | Route to focused pane's EditorState |
| **Complexity** | Coordination only, no custom rendering |

## Benefits

1. **Zero rendering code** - reuses entire normal buffer pipeline
2. **All features free** - syntax, selection, wrapping, ANSI, virtual text, etc.
3. **Edit-robust alignment** - markers + chunks handle edits gracefully
4. **Minimal markers** - O(chunks) not O(lines)
5. **Localized recomputation** - only dirty chunks re-diffed
6. **Clean separation** - CompositeBuffer is pure coordination
