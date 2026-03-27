# Settings UI Improvement Plan — Track Two: LSP Configuration Deep Dive

## Executive Summary

This document presents findings from a comprehensive UX audit of the Settings UI dialog,
focusing on the LSP Configuration section. Testing was conducted via `tmux`-scripted
interaction with the debug build, across two rounds (pre- and post-rebase on latest master).

**Round 1** uncovered 9 bugs and 6 usability issues.
**Round 2** (post-rebase) confirmed that **5 of 9 original bugs are now fixed**, but
uncovered **1 new critical regression** and **2 remaining bugs**.

### Current Bug Status (Post-Rebase)

| ID | Status | Summary |
|----|--------|---------|
| Bug 1 | **FIXED** | Text input now works — auto-edit mode on character input |
| Bug 2 | **FIXED** | Tab now toggles Fields ↔ Buttons via `toggle_focus_region()` |
| Bug 3 | **FIXED** | Button focus indicator (`>`) renders when buttons are focused |
| Bug 4 | **FIXED** | Enter on booleans no longer toggles; Space toggles booleans |
| Bug 5 | **PARTIALLY FIXED** | Down navigation still has inconsistencies (double-visits) |
| Bug 6 | **FIXED** | Ctrl+S saves entry dialog from any mode |
| Issue 7 | **FIXED** | LSP entries now show command preview (e.g., `pylsp`) |
| Issue 8 | **PARTIALLY FIXED** | Fields reordered by importance; "Advanced" separator added |
| **NEW** | **CRITICAL** | ObjectArray `[+] Add new` unreachable via keyboard |

---

## Part 1: Test Environment & Methodology

- **Build**: `cargo build` (debug profile, post-rebase on latest master)
- **Terminal**: `tmux` session, 160×50, `TERM=xterm-256color`
- **Test file**: `/tmp/fresh-test/test.py`
- **Navigation path**: Command Palette → Open Settings → General category → scroll to
  LSP section → Edit Value → Edit Item
- **Tools**: `tmux send-keys` for input, `tmux capture-pane -p` for screen capture

---

## Part 2: Verified Fixes (Post-Rebase)

### Bug 1 — Text Input: FIXED

**What changed**: `handle_entry_dialog_navigation()` (`input.rs:454–480`) now handles
`KeyCode::Char(c)` for Text, TextList, and Number fields. It calls `dialog.start_editing()`
then forwards the character to `handle_entry_dialog_text_editing()`.

**Verified**: Typing `a` on the Command field immediately appends it: `pylsp` → `pylspa`.
Multiple characters in rapid succession all register correctly.

### Bug 2 — Tab Key: FIXED

**What changed**: `Tab` now calls `dialog.toggle_focus_region()` (`entry_dialog.rs:380–401`)
which toggles between `focus_on_buttons = true/false`. This matches the status bar hint
`Tab:Fields/Buttons`.

**Verified**: Tab cycles between the item fields and the button bar. The cycle is:
- Fields (first editable item) → Tab → Buttons (Save) → Tab → Fields

**Note**: If a text field is in edit mode, the first Tab exits edit mode; the second Tab
toggles to buttons. This is correct behavior but could use a visual hint.

### Bug 3 — Button Focus Indicator: FIXED

**What changed**: Buttons now show `> [ Save ]` when focused. The rendering code at
`render.rs:3067–3075` was already correct; the fix was in Tab navigation (Bug 2) which
now actually sets `focus_on_buttons = true`.

**Verified**: When Tab reaches buttons, `> [ Save ]` is visible with bold styling.

### Bug 4 — Enter on Booleans: FIXED

**What changed**: `Enter` and `Space` are now separate handlers (`input.rs:352–453`):
- `Enter` (line 352): Activates control (opens nested dialog, starts editing) or
  triggers button action. On boolean fields, it advances to the next field.
- `Space` (line 425): Only toggles booleans and dropdowns. Does nothing on buttons.

**Verified**: Enter on `Enabled` advances to `Name`. Space on `Enabled` toggles the
checkbox.

### Bug 6 — Ctrl+S in Entry Dialog: FIXED

**What changed**: `handle_entry_dialog_input()` (`input.rs:86–92`) now checks for
`Ctrl+S` before routing to mode-specific handlers. The status bar shows `Ctrl+S:Save`.

**Verified**: Pressing Ctrl+S in the Edit Item dialog saves and closes it, returning
to the Edit Value dialog.

### Issue 7 — Array Items Show `[1 items]`: FIXED

**What changed**: LSP entries in the main settings list now show the command name
(e.g., `pylsp`, `rust-analyzer`, `clangd`) instead of `[1 items]`.

**Verified**: All LSP language entries display their server command as the preview.

### Issue 8 — Field Ordering and Grouping: PARTIALLY FIXED

**What changed**: Fields are now reordered by importance:
```
Command (first, most important)
Enabled
Name
Args
Auto Start
Root Markers
── Advanced ──
Env
Language Id Overrides
Initialization Options
Only Features
Except Features
Process Limits
```

An `── Advanced ──` separator is now visible. However, the Advanced section is not
collapsible — all fields are always visible.

---

## Part 3: Remaining Bugs

### BUG A — ObjectArray `[+] Add new` Unreachable via Keyboard (CRITICAL / NEW)

**Reproduction**: Settings → LSP → python → Edit Value → try to navigate to `[+] Add new`.

**Observed**: The Down arrow cycle in the Edit Value dialog is:
```
Value (ObjectArray label) → [Buttons: Save, Delete, Cancel] → Value → ...
```
The `[+] Add new` row inside the ObjectArray is **never focused**. It is visually
rendered but completely unreachable via keyboard navigation.

**Impact**: Users **cannot add a second LSP server** for any language via keyboard.
This is a blocking workflow issue. For example, adding `pyright` alongside `pylsp` for
Python is impossible without mouse interaction or manual JSON editing.

**Root Cause**: The `focus_next()` method (`entry_dialog.rs:316–343`) was simplified
during the recent refactor. It now treats the entire ObjectArray as a single item:
```rust
} else if self.selected_item + 1 < self.items.len() {
    self.selected_item += 1;
    self.sub_focus = None;
} else {
    self.focus_on_buttons = true;
    self.focused_button = 0;
}
```

The old code had explicit ObjectArray sub-focus logic that navigated through entries and
the `[+] Add new` row before exiting the control. This logic was removed. Since the
Edit Value dialog has only one editable item (the ObjectArray), `focus_next()` immediately
transitions to buttons.

The same issue affects any ObjectArray in the Edit Item dialog (e.g., the inner
ObjectArray if one existed), though in practice most composite controls in the Edit Item
dialog are Maps or TextLists which have their own sub-focus issues.

**Recommended Fix**:
- Restore ObjectArray sub-focus navigation in `focus_next()`/`focus_prev()`.
- The Down arrow within a focused ObjectArray should cycle:
  `entry₁ → entry₂ → ... → [+] Add new → (exit to next item/buttons)`.
- The Up arrow should reverse this.
- Alternatively, treat `[+] Add new` as a separate virtual item in the dialog's item
  list, so `selected_item += 1` naturally reaches it.

---

### BUG B — Down Navigation Inconsistencies in Edit Item Dialog (MEDIUM)

**Reproduction**: Open Edit Item for any LSP server → press Down repeatedly.

**Observed navigation trace** (25 steps):
```
Step 1:  Command (auto-enters edit mode)
Step 2:  Command (Down exits edit mode, stays on Command)
Step 3:  Enabled
Step 4:  Name (auto-enters edit mode)
Step 5:  Args (TextList label)
Step 6:  Args (sub-focus)
Step 7:  Auto Start
Step 8:  Env (Map)
Step 9:  Language Id Overrides (Map)
Step 10: Language Id Overrides (sub-focus)
Step 11: Initialization Options (JSON)
Step 12: Except Features (JSON)
Step 13: Process Limits (JSON)
Step 14: [Save button]
Step 15: [Save button]
Step 16: [Delete button]
Step 17: Command (wrap)
...
```

**Issues identified**:
1. **Text fields consume an extra Down press** (steps 1–2, 4–5): When a text field is
   focused, the first Down auto-enters edit mode (via `start_editing()`), and the second
   Down exits edit mode and advances. This makes navigation feel sluggish — each text
   field requires 2 Down presses to pass.
2. **Root Markers appears inconsistently**: It was visited in the second cycle (step 22)
   but not in the first cycle. This suggests the TextList sub-focus state affects which
   items are visited.
3. **Only Features is sometimes skipped**: Appeared in cycle 2 but not cycle 1.
4. **Button region consumes extra presses**: Steps 14–16 show 3 presses on buttons
   (Save, Save, Delete) before wrapping, when there are only 3 buttons total.

**Root Cause**: The auto-edit mode for Text fields (`input.rs:454–480`) is triggered
by `KeyCode::Char`, but `Down` arrow while in edit mode calls
`handle_entry_dialog_text_editing()` which handles Down differently (e.g., moving cursor
in JSON editors, or navigating TextList items). This creates inconsistent behavior
depending on whether the field auto-entered edit mode.

**Recommended Fix**:
- Do NOT auto-enter edit mode from Down/Up arrow navigation — only from printable
  character input.
- Ensure `focus_next()` from a text field that is NOT in edit mode simply advances to
  the next item without entering edit mode.
- Add integration tests that assert the exact Down/Up cycle matches the expected item
  order.

---

### BUG C — Left/Right Arrow Exits Button Region (LOW)

**Reproduction**: Tab to buttons → Left or Right arrow.

**Observed**: Left/Right immediately jumps back to the items region instead of navigating
between buttons (Save ↔ Delete ↔ Cancel).

**Root Cause**: In `handle_entry_dialog_navigation()` (`input.rs:334–350`):
```rust
KeyCode::Left => {
    if !dialog.focus_on_buttons {
        dialog.decrement_number();
    } else if dialog.focused_button > 0 {
        dialog.focused_button -= 1;
    }
}
KeyCode::Right => {
    if !dialog.focus_on_buttons {
        dialog.increment_number();
    } else if dialog.focused_button + 1 < dialog.button_count() {
        dialog.focused_button += 1;
    }
}
```

The Left handler on the first button (index 0) does nothing (correctly), but the Right
handler should advance to the next button. However, testing shows that Right immediately
returns to fields. This suggests `focus_on_buttons` is being reset somewhere, or the
event is being consumed by a different handler path.

**Recommended Fix**: Debug the event flow when focus is on buttons and Left/Right is
pressed. Ensure `focus_on_buttons` remains true during button navigation.

---

## Part 4: Remaining Usability Issues

### Issue 9 — Complex Types Still Rendered as Raw JSON

`Process Limits`, `Except Features`, `Only Features`, and `Initialization Options` are
still rendered as raw JSON text editors. This was not addressed in the rebase.

### Issue 10 — No $PATH Validation for Command Field

No autocomplete or validation for the `Command` field. Users can type any string.

### Issue 11 — Advanced Section Not Collapsible

The `── Advanced ──` separator is purely visual. All advanced fields are always visible
and must be scrolled past. The separator cannot be toggled to collapse/expand the section.

---

## Part 5: Updated Improvement Plan (Phased)

### Phase 1 — Critical: ObjectArray Navigation (P0)

| # | Issue | File(s) | Effort |
|---|-------|---------|--------|
| 1 | Restore ObjectArray sub-focus in `focus_next()`/`focus_prev()` | `entry_dialog.rs` | Medium |
| 2 | Ensure `[+] Add new` is reachable in ALL ObjectArray controls | `entry_dialog.rs` | Medium |
| 3 | Add test: verify python LSP → Add new server is reachable | Integration test | Small |

**Acceptance Criteria**:
- From the Edit Value dialog for any LSP language, Down arrow visits: existing
  entries → `[+] Add new` → buttons.
- Pressing Enter on `[+] Add new` opens the Add Item dialog.
- A second LSP server can be added for Python entirely via keyboard.

### Phase 2 — Navigation Polish (P1)

| # | Issue | File(s) | Effort |
|---|-------|---------|--------|
| 4 | Fix text fields consuming extra Down press | `input.rs`, `entry_dialog.rs` | Medium |
| 5 | Fix Left/Right arrow exiting button region | `input.rs` | Small |
| 6 | Ensure consistent Down/Up cycle visits all items exactly once | `entry_dialog.rs` | Medium |
| 7 | Add integration test for complete navigation cycle | Test file | Medium |

### Phase 3 — Information Architecture (P2)

| # | Issue | File(s) | Effort |
|---|-------|---------|--------|
| 8 | Collapsible Advanced section (accordion) | New widget, `entry_dialog.rs` | Large |
| 9 | Structured editors for Process Limits | `items.rs`, schema | Medium |
| 10 | Feature checklist for Only/Except Features | New widget | Large |

### Phase 4 — Polish (P3)

| # | Issue | File(s) | Effort |
|---|-------|---------|--------|
| 11 | $PATH validation for Command field | New validation module | Medium |
| 12 | Add visual hint when text field is in edit mode vs navigation | `render.rs` | Small |

---

## Part 6: TUI UX Architecture Compliance (Updated)

| Principle | Status | Notes |
|-----------|--------|-------|
| **Dialog Modality** | ✅ Pass | Entry dialog isolates input |
| **Visual Hierarchy** | ✅ Pass | Rounded borders, padding, Advanced separator |
| **"Where Am I?" Focus Rule** | ⚠️ Partial | Focus indicator works but text auto-edit creates ambiguity |
| **Strict Tab Loop** | ✅ Pass | Tab toggles Fields ↔ Buttons |
| **Read-Only Skip** | ✅ Pass | Read-only Key field skipped |
| **Composite Bypass** | ❌ Fail | ObjectArray sub-items unreachable |
| **Esc = Abort Context** | ✅ Pass | Esc closes dialogs / exits edit mode |
| **Global Save Shortcut** | ✅ Pass | Ctrl+S works in entry dialog |
| **Collapsible Sections** | ⚠️ Partial | Separator exists but not collapsible |

---

## Appendix A: Key Source Files

| File | Role |
|------|------|
| `crates/fresh-editor/src/view/settings/entry_dialog.rs` | Entry dialog state, focus_next/prev, toggle_focus_region |
| `crates/fresh-editor/src/view/settings/input.rs` | Input routing, entry dialog navigation/text/Ctrl+S handling |
| `crates/fresh-editor/src/view/settings/render.rs` | All rendering including entry dialog, buttons, Advanced separator |
| `crates/fresh-editor/src/view/settings/state.rs` | Main settings state, panel focus management |
| `crates/fresh-editor/src/view/settings/schema.rs` | JSON schema parsing, x-display-field, x-section |
| `crates/fresh-editor/src/view/settings/items.rs` | Schema → SettingItem/SettingControl conversion |
| `crates/fresh-editor/src/view/controls/map_input/mod.rs` | MapState, get_display_value (now shows command preview) |
| `crates/fresh-editor/plugins/config-schema.json` | LSP schema (LspLanguageConfig array, LspServerConfig) |

## Appendix B: Navigation Traces

### Edit Value Dialog (python LSP) — Down Cycle

```
Value (ObjectArray) → [Save] → [Delete] → [Cancel] → Value (wrap) → ...

MISSING: pylsp entry sub-focus, [+] Add new — never visited
```

### Edit Item Dialog (pylsp) — Down Cycle (25 steps)

```
 1: Command (auto-edit)     |  14: [Save]
 2: Command (exit edit)     |  15: [Save]
 3: Enabled                 |  16: [Delete]
 4: Name (auto-edit)        |  17: Command (wrap)
 5: Args                    |  18: Enabled
 6: Args (sub-focus)        |  19: Enabled
 7: Auto Start              |  20: Name
 8: Env                     |  21: Auto Start
 9: Language Id Overrides   |  22: Root Markers
10: Language Id Overrides   |  23: Env
11: Initialization Options  |  24: Env
12: Except Features         |  25: Language Id Overrides
13: Process Limits          |

Note: Only Features and Root Markers appear inconsistently between cycles.
Text fields consume an extra Down press due to auto-edit mode.
```

## Appendix C: Specific Test Case — Add New LSP Server for Python

**Goal**: Add `pyright` as a second LSP server alongside `pylsp` for Python.

**Expected flow**:
1. Settings → General → scroll to LSP → python → Enter (Edit Value)
2. Down to `[+] Add new` → Enter (opens Add Item dialog)
3. Fill in Command: `pyright-langserver`, Args: `--stdio`, Enabled: ✓
4. Ctrl+S to save

**Actual result**: Step 2 fails — `[+] Add new` cannot be focused via keyboard.
Down goes directly from the ObjectArray label to the Save button, skipping all
internal entries and the Add new row.

**Workaround**: None via keyboard. Users must either:
- Use mouse to click `[+] Add new`
- Manually edit the JSON settings file
