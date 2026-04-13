# Review Diff — NN/g Usability Evaluation

**Date:** 2026-04-13
**Editor:** `fresh` 0.2.23 (debug build, branch `claude/tui-editor-usability-eval-0LHgo`)
**Environment:** tmux 3.4, 160x45 pane, Linux 4.4.0, terminal with 256-color ANSI
**Artifacts:** Screen captures (ANSI + plain) in `/tmp/eval-workspace/screen_*.txt`

---

## 1. Executive Summary

The Review Diff feature is **functional for a first-pass review** but is held
back by a handful of silent-failure defects and discoverability gaps that
add needless cognitive load. A user who already knows the mode's vocabulary
(`Tab`, `n`, `Enter`, `q`, `s`, `u`, `d`) can complete a standard PR-style
review; a newcomer will bounce off at least two soft failures before learning
the layout.

**Overall usability score: 3.4 / 5 (Fair, not NN/g "Good")**

- Heuristic wins: highly discoverable entry (`Ctrl+P` shown in status bar),
  a permanent action toolbar, and a safe exit affordance (`q Close`).
- Heuristic losses: **no "hunk N of M" indicator**, viewport does not follow
  the cursor in side-by-side view, word-level diff highlighting is absent,
  and the fuzzy finder only does subsequence matching (no typo tolerance).

**Primary roadblocks (ranked):**

1. **H1 — Silent hunk navigation (visibility of system status).** `n`/`p`
   from the unified diff panel advance the cursor by roughly one line each
   press and give no indication of "which hunk am I on." The user cannot
   tell whether the keystroke succeeded.
2. **H1 — Viewport/cursor desync in side-by-side view.** Status bar always
   reads `Ln 1, Col 1` even after the viewport scrolls 170 lines.
3. **H5 / accessibility — Whitespace-only changes are invisible.** Trailing
   spaces and collapsed/expanded spaces render identically on both sides,
   only the `-`/`+` marker differs.
4. **H9 (error recovery) — Typo tolerance in the command palette is
   weak.** "revw difff" returns `Markdown: Toggle Compose/Preview` as the
   best match — not "Review Diff."

---

## 2. Heuristic Evaluation (NN/g, adapted for TUI diff review)

### H1. Visibility of System Status — **Partial Pass**

| Signal | Present? | Evidence |
|--------|----------|----------|
| Current file name in right-pane header | **Yes** | `DIFF FOR happy.py` (screen_04) |
| Selected row marker in files list | **Yes** | `>M  happy.py` (caret glyph) |
| Total hunk count | **Yes** | Status bar: `Review Diff: 14 hunks` |
| Current-hunk index ("hunk 3 of 10") | **No** | Never displayed |
| Cursor line / column | **Yes (unified view)** | `Ln 21, Col 1` |
| Cursor line / column in side-by-side | **BROKEN** | Stuck at `Ln 1, Col 1` even after 170-line scroll (screen_17) |
| Add/del stats per file | **Partial** | Only in side-by-side: `+10 -10 ~10` |
| Context-sensitive toolbar | **Yes** | Toolbar swaps `↵ Open / r Refresh` for `n Next / p Prev` when focus is on diff (screen_04 vs screen_06) |

**Violation — no hunk index.** With 10 hunks in `monolith.txt`, pressing `n`
four times gave no feedback except a cursor line change from 21 → 29. The
user cannot confirm they arrived at the intended hunk.

### H2. User Control and Freedom — **Pass**

- `q` cleanly closes the Review Diff tab and returns to the previous buffer
  (status: `Tab closed`, screen_27).
- `q` also exits the side-by-side view back to the unified view (screen_18).
- `Escape` closes the command palette without side effects
  (status: `Search cancelled`, screen_26).
- `Ctrl+C` at the editor root is swallowed gracefully (no crash, no prompt,
  editor stays alive — verified PID persisted).

**Gap:** there is no `Undo comment` or reopen-last-closed-review affordance
visible in the toolbar. A user who closes the review tab must re-run
`Ctrl+P` → "Review Diff" to get back in.

### H3. Consistency & Standards — **Partial Pass**

- `+`/`-` prefixes are standard (screen_04).
- Color palette matches convention: dark red `bg 256:52` for removed lines,
  dark green `bg 256:22` for added lines, no intrusive foreground recolor.
- **Deviation from `git diff --unified` standard:** the hunk header is
  rendered as `@@ line_0047 = value_47  # comment for line 47 @@` — the
  post-context line contents — instead of the standard
  `@@ -47,7 +47,7 @@ <function_sig>`. This aids casual readers but breaks
  muscle memory for anyone used to `git diff` / GitHub / `vimdiff`. The
  missing `-start,count +start,count` numbers make it impossible to count
  added/removed lines per hunk at a glance.
- Standard review-mode keys (`s`/`u`/`d`) match `git add -p` / lazygit
  conventions.
- `N` (capital) for "Note" is inconsistent with lowercase action keys
  elsewhere on the same toolbar — users will try lowercase `n` first and
  trigger hunk-nav instead.

### H4. Flexibility & Efficiency of Use — **Partial Pass**

Minimum keystrokes to enter Review Diff from editing:

| Path | Keystrokes | Notes |
|------|------------|-------|
| `Ctrl+P` → `revie` → `Enter` | **7 key events** | Default fuzzy result is "Review Diff" |
| `Ctrl+P` → `rd` → `Enter` | theoretical 4 events | **Not verified** — `rd` did not narrow to Review Diff in testing |

Minimum keystrokes to scroll to the 5th hunk of 10 once the view is open:

| Path | Keystrokes | Observed? |
|------|------------|-----------|
| `Tab` `n n n n` | 5 | Cursor advances, but **viewport does not snap to the hunk header** and there is no index feedback |

**Missing power-user shortcut:** there is no "jump to file N" or
`:<file>` addressing mode inside Review Diff. To reach the 4th file you
must `Tab` to files panel and press `j` three times.

**Good:** Tab toggles focus between files pane and diff pane — verified
twice and well-behaved. `PageDown` scrolls the diff viewport and updates
the cursor line counter correctly in the *unified* view.

### H5. Aesthetic & Minimalist Design — **Pass**

- Two-pane layout (30% files, 70% diff) is balanced at 160 cols.
- Section headers (`▸ Changes` / `▸ Untracked`) are collapsible-style
  indicators that clearly group the file list.
- The toolbar is dense (`s Stage  u Unstage  d Discard │ c Comment  N Note  x Del │ e Export  q Close  ↵ Open  Tab Switch  r Refresh`) — 28 characters of
  actionable hints, separated by `│` glyphs. At 160 cols this fits; at
  narrower widths it will wrap or truncate (see prior BUG-10).
- No superfluous chrome in the diff pane — line-number column is
  intentionally omitted in the unified view, present in side-by-side.

**Cost:** the toolbar *duplicates* information that exists in menus and
documentation, yet omits the two keys users most need (`n`/`p` are only
shown **after** Tab-switching focus to the diff pane). Unified-view users
don't know hunk navigation exists until they've already switched focus.

---

## 3. Friction Points (Flow-by-Flow)

### Flow 1 — Happy Path (modify `happy.py`: 3 adds, 2 dels, 1 hunk)

**Friction level: low.** Time-to-first-read ≈ 7 keystrokes.

- ✅ Review Diff opens in ~300ms on debug build.
- ✅ `@@ -1 +1 @@` header present; coloring correct.
- ✅ Status bar shows `Review Diff: 14 hunks`.
- ⚠ On first open, focus is on **files pane** — pressing `n` or `Enter`
  before Tabbing feels ambiguous. This matches prior BUG-3 findings.
- ⚠ Header reads "@@ -1 +1 @@" without range counts (`-1,12 +1,16`).

### Flow 2 — Monolith (1,000-line file, 10 hunks)

**Friction level: high.**

- ✅ Rendering of 10 hunks was instant; `PageDown`×50 completed in ~3s
  total (most of that was `tmux send-keys` overhead) with no visible
  input lag.
- ❌ **H1 violation:** pressing `n` four times moved cursor from line 21
  to line 29 (~+2 per press) — not a hunk-sized jump. No "hunk 5 of 10"
  indicator confirmed arrival at the target. Reproduces BUG-4.
- ❌ Top-of-viewport did not re-anchor to the jumped-to hunk header; we
  scrolled past hunks while thinking we were on hunk 5.
- ⚠ The non-standard `@@ line_0047 = value_47 ... @@` header replaces the
  `@@ -47,3 +47,4 @@` standard, so line-number context must be inferred
  from the content, not read directly.

### Flow 3 — Edge Cases

| State | Outcome |
|-------|---------|
| Whitespace-only change | Diff shows `-hello world` vs `+hello world` with **no visual cue** for the trailing space. Tab chars **are** rendered as `→` glyph (good). Two-space runs are not distinguished from one-space runs. |
| Newly created untracked file | Renders as `@@ -0 +1 @@` with all `+` lines. Layout not broken. ✅ |
| Deleted file | Unified view: `@@ -1 +0 @@` followed by `-` lines. ✅ |
| **Deleted file drill-down** (side-by-side) | **Does NOT hang** — the view opens with OLD content on the left and an empty pane on the right. BUG-5 from the prior combined report appears to be **fixed**. ✅ |

### Flow 4 — Lost User / Error Recovery

| Input | Response |
|-------|----------|
| `Ctrl+P` → "revw difff" | Top result: "Markdown: Toggle Compose/Preview" — Review Diff **not in top 10**. |
| `Ctrl+P` → "reiew" (missing v) | "Review Diff" is top match. ✅ |
| `Ctrl+P` → "rview" (missing e) | "Stop Review Diff" / "Refresh Review Diff" shown — acceptable. |
| Invalid keys in diff panel (`x`, `z`, `Q`, `%`) | Status bar: `Editing disabled in this buffer`. No panic, but a generic message that doesn't tell the user what they *can* do. |
| `Ctrl+C` at editor top level | Swallowed cleanly — editor remains alive. ✅ |

**Finding:** The fuzzy finder is a strict **subsequence** matcher. It does
not tolerate inserted or substituted characters. Standard NN/g guidance
calls for typo-tolerant search (Levenshtein within 1–2 edits) on
rarely-used commands.

---

## 4. Color & Layout Analysis (ANSI-parsed)

ANSI 256-color codes harvested from `capture-pane -e`:

| Role | Code | RGB (approx) | Notes |
|------|------|--------------|-------|
| Removed-line background | `48;5;52` | `#5F0000` | Dark red |
| Added-line background | `48;5;22` | `#005F00` | Dark green |
| Removed-line fg | `38;5;203` | `#FF5F5F` | Salmon — **contrast OK** on black bg |
| Added-line fg | `38;5;40` / `38;5;64` | `#00D700` / `#5F8700` | Bright green |
| Status bar bg | `48;5;226` | `#FFFF00` | Yellow |
| Status bar fg | `38;5;16` | `#000000` | Black |
| Tilde (empty-row) fg | `38;5;59` | dim grey | Unobtrusive |

**Accessibility notes:**

- Red/green-only encoding is the classic deuteranopia trap. The `-`/`+`
  glyph does carry the semantic, but the **background** color does the
  heavy visual lifting. A colorblind user relying on glyphs alone will
  struggle with pure-whitespace diffs (see Flow 3).
- Contrast ratios: `#FF5F5F` on `#5F0000` ≈ 4.0:1 (passes WCAG AA for
  normal text, fails AAA). `#00D700` on `#005F00` ≈ 3.9:1 (AA only).
- Status bar yellow (`#FFFF00`) bg with black text is high contrast
  (>10:1) — ✅.

**Layout / alignment:**

- In side-by-side view, line numbers are right-aligned within a 3-char
  column (`  1`, ` 42`, `171`). For files >999 lines the column expands
  correctly.
- Unified-view diff does **not** show line numbers in the gutter
  (screen_04). This is a deviation from `git diff --unified` with line
  numbers and a measurable source of friction when discussing a specific
  line with a teammate ("which line is that?").
- The `│` vertical separator between panes is rendered as single-column
  box-drawing — clean.

---

## 5. Actionable Recommendations

Ordered by effort-to-impact ratio.

### R1 — Show "Hunk N of M" in the status bar (Low effort, High impact)

Add a right-aligned segment: `Hunk 3 / 10`. Update it on every `n`, `p`,
`j`, `k`, or scroll event. This single change fixes the H1 violation in
Flow 2 and gives users navigational confidence.

*Files:* `crates/fresh-editor/plugins/audit_mode.ts` (status message
builder) + an additional `status.hunk_position` key in
`audit_mode.i18n.json`.

### R2 — Snap viewport to the hunk header on `n` / `p` (Medium effort)

Currently `n` only nudges the cursor. Change the handler so that after
moving the cursor to `hunkHeaderRows[idx]`, it forces viewport top =
`hunkHeaderRows[idx] - 2` (two lines of leading context). Also highlight
the hunk header row (reverse video) while the cursor is inside that
hunk. This addresses BUG-4 at its root.

### R3 — Word-level diff highlighting for intra-line changes (Medium effort)

Especially for whitespace-only diffs, byte-level reverse-video on the
*differing* ranges would make `-hello world` vs `+hello world ` (trailing
space) and tab-vs-space changes self-evident. The `diff_nav` plugin
already computes character ranges for `review_export_session`; expose the
same ranges to the renderer.

### R4 — Adopt standard `@@ -start,count +start,count @@` hunk header (Low effort)

Replace the custom "first context line" header with the git-standard one.
Users coming from `git diff`, GitHub, and `vimdiff` will recognize it
immediately (H3: Consistency & Standards). The first context line can be
appended after the closing `@@` as GitHub does:
`@@ -47,7 +47,7 @@ def greet(name):`.

### R5 — Upgrade the command palette to typo-tolerant fuzzy match (Medium effort)

Replace strict subsequence matching in
`crates/fresh-editor/src/input/fuzzy/mod.rs` with a hybrid: subsequence
→ if < N results, fall back to Levenshtein (edit distance ≤ 2) over the
command label. This resolves Flow 4's "revw difff" → Markdown mismatch
and brings the palette in line with VS Code / Zed expectations.

### R6 (Bonus) — Auto-focus the files panel on Review Diff launch (Trivial effort)

Documented in the prior combined report as BUG-3. A one-line fix in
`start_review_diff()` eliminates the silent "first key press does
nothing" trap that every new user hits.

---

## Appendix A — Reproduction Commands

```bash
# 1. Build (debug)
cargo build

# 2. Prepare test repo (whitespace + new + deleted + monolith)
cd /tmp && mkdir eval && cd eval && git init -q
# ... (see body for full setup; ran from /tmp/eval-workspace/testrepo)

# 3. tmux session with ANSI capture
tmux new-session -d -s tui-test -x 160 -y 45
tmux send-keys -t tui-test "cd /tmp/eval-workspace/testrepo && \
  /home/user/fresh/target/debug/fresh" C-m
sleep 1

# 4. Open Review Diff
tmux send-keys -t tui-test C-p; sleep 0.5
tmux send-keys -t tui-test -l "review diff"; sleep 0.5
tmux send-keys -t tui-test Enter; sleep 1

# 5. Capture with colors
tmux capture-pane -t tui-test -p -e > current_screen.txt
```

## Appendix B — Screen Capture Index

All artifacts under `/tmp/eval-workspace/`:

| File | Scenario |
|------|----------|
| `screen_04_review_ansi.txt` | Happy-path unified diff, ANSI |
| `screen_07_nextHunk.txt` | `n` in unified diff — no visible jump |
| `screen_13_whitespace_ansi.txt` | Whitespace-only diff |
| `screen_16_drilldown.txt` | Side-by-side for `monolith.txt` |
| `screen_17_sxs_next.txt` | `n` in side-by-side — viewport moves but `Ln` stays 1 |
| `screen_20_delete_drill.txt` | Deleted-file drill-down (no hang — BUG-5 fixed) |
| `screen_22_typo.txt` | Palette with "revw difff" — wrong top match |
| `screen_28_ctrlc.txt` | Ctrl+C at root — editor survives |
