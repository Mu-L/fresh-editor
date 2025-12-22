// Review Diff Plugin
// Provides a unified workflow for reviewing code changes (diffs, conflicts, AI outputs).

/// <reference path="./lib/fresh.d.ts" />
/// <reference path="./lib/types.ts" />
/// <reference path="./lib/virtual-buffer-factory.ts" />

import { VirtualBufferFactory } from "./lib/virtual-buffer-factory.ts";

/**
 * Hunk status for staging
 */
type HunkStatus = 'pending' | 'staged' | 'discarded';

/**
 * A diff hunk (block of changes)
 */
interface Hunk {
  id: string;
  file: string;
  range: { start: number; end: number }; // Line numbers in original file
  type: 'add' | 'remove' | 'modify';
  lines: string[];
  status: HunkStatus;
  contextHeader: string; // e.g., "fn process_data()"
  byteOffset: number; // Position in the virtual buffer
}

/**
 * Review Session State
 */
interface ReviewState {
  hunks: Hunk[];
  // Mapping from hunk ID to status (persisted across re-renders)
  hunkStatus: Record<string, HunkStatus>;
  // The buffer ID of the main "Review Stream"
  reviewBufferId: number | null;
  // Currently focused hunk index
  focusedHunkIndex: number;
}

const state: ReviewState = {
  hunks: [],
  hunkStatus: {},
  reviewBufferId: null,
  focusedHunkIndex: -1,
};

// --- Colors & Styles ---
const STYLE_BORDER: [number, number, number] = [80, 80, 80]; // Dim grey
const STYLE_HEADER: [number, number, number] = [100, 100, 255]; // Blueish
const STYLE_FILE_NAME: [number, number, number] = [200, 200, 100]; // Yellowish
const STYLE_ADD_BG: [number, number, number] = [20, 60, 20]; // Dark Green BG
const STYLE_REMOVE_BG: [number, number, number] = [60, 20, 20]; // Dark Red BG
const STYLE_ADD_TEXT: [number, number, number] = [100, 255, 100]; // Bright Green
const STYLE_REMOVE_TEXT: [number, number, number] = [255, 100, 100]; // Bright Red
const STYLE_STAGED: [number, number, number] = [100, 100, 100]; // Dimmed/Grey
const STYLE_DISCARDED: [number, number, number] = [150, 50, 50];

/**
 * Calculate UTF-8 byte length of a string manually since TextEncoder is not available
 */
function getByteLength(str: string): number {
    let s = 0;
    for (let i = 0; i < str.length; i++) {
        const code = str.charCodeAt(i);
        if (code <= 0x7f) s += 1;
        else if (code <= 0x7ff) s += 2;
        else if (code >= 0xd800 && code <= 0xdfff) {
            // Surrogate pair
            s += 4;
            i++;
        } else s += 3;
    }
    return s;
}

// --- Diff Logic ---

interface DiffPart {
    text: string;
    type: 'added' | 'removed' | 'unchanged';
}

/**
 * Simple character-level LCS for word diffing
 */
function diffStrings(oldStr: string, newStr: string): DiffPart[] {
    const n = oldStr.length;
    const m = newStr.length;
    const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));

    for (let i = 1; i <= n; i++) {
        for (let j = 1; j <= m; j++) {
            if (oldStr[i - 1] === newStr[j - 1]) {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
            }
        }
    }

    const result: DiffPart[] = [];
    let i = n, j = m;
    while (i > 0 || j > 0) {
        if (i > 0 && j > 0 && oldStr[i - 1] === newStr[j - 1]) {
            result.unshift({ text: oldStr[i - 1], type: 'unchanged' });
            i--; j--;
        } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
            result.unshift({ text: newStr[j - 1], type: 'added' });
            j--;
        } else {
            result.unshift({ text: oldStr[i - 1], type: 'removed' });
            i--;
        }
    }

    const coalesced: DiffPart[] = [];
    for (const part of result) {
        const last = coalesced[coalesced.length - 1];
        if (last && last.type === part.type) {
            last.text += part.text;
        } else {
            coalesced.push(part);
        }
    }
    return coalesced;
}

async function getGitDiff(): Promise<Hunk[]> {
    editor.debug("ReviewDiff: Running git diff HEAD");
    const result = await editor.spawnProcess("git", ["diff", "HEAD", "--unified=3"]);
    if (result.exit_code !== 0) {
        editor.debug(`ReviewDiff: Git diff failed: ${result.stderr}`);
        return [];
    }

    const lines = result.stdout.split('\n');
    const hunks: Hunk[] = [];
    let currentFile = "";
    let currentHunk: Hunk | null = null;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];

        if (line.startsWith('diff --git')) {
            const match = line.match(/diff --git a\/(.+) b\/(.+)/);
            if (match) {
                currentFile = match[2];
                currentHunk = null;
            }
        } else if (line.startsWith('@@')) {
            const match = line.match(/@@ -(\d+),?\d* \+(\d+),?\d* @@(.*)/);
            if (match && currentFile) {
                const start = parseInt(match[2]);
                currentHunk = {
                    id: `${currentFile}:${start}`,
                    file: currentFile,
                    range: { start, end: start },
                    type: 'modify',
                    lines: [],
                    status: 'pending',
                    contextHeader: match[3]?.trim() || "",
                    byteOffset: 0
                };
                hunks.push(currentHunk);
            }
        } else if (currentHunk && (line.startsWith('+') || line.startsWith('-') || line.startsWith(' '))) {
            if (!line.startsWith('---') && !line.startsWith('+++')) {
                 currentHunk.lines.push(line);
            }
        }
    }
    return hunks;
}

interface HighlightTask {
    range: [number, number];
    fg: [number, number, number];
    bg?: [number, number, number];
    bold?: boolean;
}

/**
 * Render the Review Stream buffer content and return highlight tasks
 */
function renderReviewStream(): { entries: TextPropertyEntry[], highlights: HighlightTask[] } {
  const entries: TextPropertyEntry[] = [];
  const highlights: HighlightTask[] = [];
  let currentFile = "";
  let currentByte = 0;

  state.hunks.forEach((hunk, hunkIndex) => {
    if (hunk.file !== currentFile) {
      // Top border with filename
      const titlePrefix = "┌─ ";
      const titleLine = `${titlePrefix}${hunk.file} ${"─".repeat(Math.max(0, 60 - hunk.file.length))}\n`;
      const titleLen = getByteLength(titleLine);
      entries.push({
        text: titleLine,
        properties: { type: "banner", file: hunk.file }
      });
      highlights.push({ range: [currentByte, currentByte + titleLen], fg: STYLE_BORDER });
      // filename highlight
      const prefixLen = getByteLength(titlePrefix);
      highlights.push({ range: [currentByte + prefixLen, currentByte + prefixLen + getByteLength(hunk.file)], fg: STYLE_FILE_NAME, bold: true });
      currentByte += titleLen;
      currentFile = hunk.file;
    }

    hunk.byteOffset = currentByte;

    const statusIcon = hunk.status === 'staged' ? '✓' : (hunk.status === 'discarded' ? '✗' : ' ');
    const headerPrefix = "│ ";
    const headerText = `${headerPrefix}${statusIcon} [ ${hunk.contextHeader} ]\n`;
    const headerLen = getByteLength(headerText);
    let hunkColor = STYLE_HEADER;
    if (hunk.status === 'staged') hunkColor = STYLE_STAGED;
    else if (hunk.status === 'discarded') hunkColor = STYLE_DISCARDED;

    entries.push({
      text: headerText,
      properties: { type: "header", hunkId: hunk.id, index: hunkIndex }
    });
    highlights.push({ range: [currentByte, currentByte + headerLen], fg: STYLE_BORDER });
    // Status icon highlight
    const headerPrefixLen = getByteLength(headerPrefix);
    highlights.push({ range: [currentByte + headerPrefixLen, currentByte + headerPrefixLen + getByteLength(statusIcon)], fg: hunkColor, bold: true });
    // Header content highlight
    highlights.push({ range: [currentByte + headerPrefixLen + getByteLength(statusIcon) + 3, currentByte + headerLen - 3], fg: hunkColor });
    
    currentByte += headerLen;

    for (let i = 0; i < hunk.lines.length; i++) {
        const line = hunk.lines[i];
        const nextLine = hunk.lines[i + 1];
        const marker = line[0];
        const content = line.substring(1);
        const linePrefix = "│   ";
        const lineText = `${linePrefix}${marker} ${content}\n`;
        const lineLen = getByteLength(lineText);
        const linePrefixLen = getByteLength(linePrefix);

        if (line.startsWith('-') && nextLine && nextLine.startsWith('+') && hunk.status === 'pending') {
            const oldContent = line.substring(1);
            const newContent = nextLine.substring(1);
            const diffParts = diffStrings(oldContent, newContent);

            // Removed Line
            entries.push({ text: lineText, properties: { type: "content", hunkId: hunk.id } });
            highlights.push({ range: [currentByte, currentByte + lineLen], fg: STYLE_BORDER });
            highlights.push({ range: [currentByte + linePrefixLen, currentByte + linePrefixLen + 1], fg: STYLE_REMOVE_TEXT, bold: true });
            
            let charByteOffset = currentByte + linePrefixLen + 2; // skip "│   - "
            diffParts.forEach(p => {
                const partLen = getByteLength(p.text);
                if (p.type === 'removed') {
                    highlights.push({ range: [charByteOffset, charByteOffset + partLen], fg: STYLE_REMOVE_TEXT, bg: STYLE_REMOVE_BG, bold: true });
                    charByteOffset += partLen;
                } else if (p.type === 'unchanged') {
                    highlights.push({ range: [charByteOffset, charByteOffset + partLen], fg: STYLE_REMOVE_TEXT });
                    charByteOffset += partLen;
                }
            });
            currentByte += lineLen;

            // Added Line
            const nextMarker = nextLine[0];
            const nextContent = nextLine.substring(1);
            const nextLineText = `${linePrefix}${nextMarker} ${nextContent}\n`;
            const nextLineLen = getByteLength(nextLineText);
            entries.push({ text: nextLineText, properties: { type: "content", hunkId: hunk.id } });
            highlights.push({ range: [currentByte, currentByte + nextLineLen], fg: STYLE_BORDER });
            highlights.push({ range: [currentByte + linePrefixLen, currentByte + linePrefixLen + 1], fg: STYLE_ADD_TEXT, bold: true });

            charByteOffset = currentByte + linePrefixLen + 2; // skip "│   + "
            diffParts.forEach(p => {
                const partLen = getByteLength(p.text);
                if (p.type === 'added') {
                    highlights.push({ range: [charByteOffset, charByteOffset + partLen], fg: STYLE_ADD_TEXT, bg: STYLE_ADD_BG, bold: true });
                    charByteOffset += partLen;
                } else if (p.type === 'unchanged') {
                    highlights.push({ range: [charByteOffset, charByteOffset + partLen], fg: STYLE_ADD_TEXT });
                    charByteOffset += partLen;
                }
            });
            currentByte += nextLineLen;
            i++; 
        } else {
            entries.push({ text: lineText, properties: { type: "content", hunkId: hunk.id } });
            highlights.push({ range: [currentByte, currentByte + lineLen], fg: STYLE_BORDER });
            if (hunk.status === 'pending') {
                if (line.startsWith('+')) {
                    highlights.push({ range: [currentByte + linePrefixLen, currentByte + linePrefixLen + 1], fg: STYLE_ADD_TEXT, bold: true });
                    highlights.push({ range: [currentByte + linePrefixLen + 2, currentByte + lineLen], fg: STYLE_ADD_TEXT });
                } else if (line.startsWith('-')) {
                    highlights.push({ range: [currentByte + linePrefixLen, currentByte + linePrefixLen + 1], fg: STYLE_REMOVE_TEXT, bold: true });
                    highlights.push({ range: [currentByte + linePrefixLen + 2, currentByte + lineLen], fg: STYLE_REMOVE_TEXT });
                }
            } else {
                highlights.push({ range: [currentByte + linePrefixLen, currentByte + lineLen], fg: hunkColor });
            }
            currentByte += lineLen;
        }
    }

    // Bottom border for file (only if last hunk of file)
    const isLastHunkOfFile = hunkIndex === state.hunks.length - 1 || state.hunks[hunkIndex + 1].file !== hunk.file;
    if (isLastHunkOfFile) {
        const bottomLine = `└${"─".repeat(64)}\n`;
        const bottomLen = getByteLength(bottomLine);
        entries.push({
            text: bottomLine,
            properties: { type: "border" }
        });
        highlights.push({ range: [currentByte, currentByte + bottomLen], fg: STYLE_BORDER });
        currentByte += bottomLen;
    }
  });

  if (entries.length === 0) {
      entries.push({ text: "No changes to review.\n", properties: {} });
  }

  return { entries, highlights };
}

function refreshReviewStream() {
  if (state.reviewBufferId !== null) {
    const { entries, highlights } = renderReviewStream();
    editor.setVirtualBufferContent(state.reviewBufferId, entries);
    
    editor.clearNamespace(state.reviewBufferId, "review-diff");
    highlights.forEach((h) => {
        const bg = h.bg || [-1, -1, -1];
        editor.addOverlay(
            state.reviewBufferId!,
            "review-diff", 
            h.range[0],
            h.range[1],
            h.fg[0], h.fg[1], h.fg[2],
            bg[0], bg[1], bg[2],
            false, h.bold || false, false
        );
    });
  }
}

// --- Refresh Logic ---

let isUpdating = false;

async function updateHunks(): Promise<boolean> {
    const newHunks = await getGitDiff();
    newHunks.forEach(hunk => {
        hunk.status = state.hunkStatus[hunk.id] || 'pending';
    });
    state.hunks = newHunks;
    return true;
}

async function refreshReviewStreamBuffer() {
    if (isUpdating) return;
    isUpdating = true;
    editor.setStatus("Refreshing review diff...");

    try {
        await updateHunks();
        refreshReviewStream();
        editor.setStatus(`Review diff updated. Found ${state.hunks.length} hunks.`);
    } catch (e) {
        editor.debug(`ReviewDiff: Error updating: ${e}`);
    } finally {
        isUpdating = false;
    }
}

// --- Actions ---

globalThis.review_stage_hunk = () => {
    const bufferId = editor.getActiveBufferId();
    const props = editor.getTextPropertiesAtCursor(bufferId);
    if (props.length > 0 && props[0].hunkId) {
        const hunkId = props[0].hunkId as string;
        state.hunkStatus[hunkId] = 'staged';
        const hunk = state.hunks.find(h => h.id === hunkId);
        if (hunk) hunk.status = 'staged';
        refreshReviewStream();
    }
};

globalThis.review_discard_hunk = () => {
    const bufferId = editor.getActiveBufferId();
    const props = editor.getTextPropertiesAtCursor(bufferId);
    if (props.length > 0 && props[0].hunkId) {
        const hunkId = props[0].hunkId as string;
        state.hunkStatus[hunkId] = 'discarded';
        const hunk = state.hunks.find(h => h.id === hunkId);
        if (hunk) hunk.status = 'discarded';
        refreshReviewStream();
    }
};

globalThis.review_undo_action = () => {
    const bufferId = editor.getActiveBufferId();
    const props = editor.getTextPropertiesAtCursor(bufferId);
    if (props.length > 0 && props[0].hunkId) {
        const hunkId = props[0].hunkId as string;
        state.hunkStatus[hunkId] = 'pending';
        const hunk = state.hunks.find(h => h.id === hunkId);
        if (hunk) hunk.status = 'pending';
        refreshReviewStream();
    }
};

globalThis.review_next_hunk = () => {
    const bufferId = editor.getActiveBufferId();
    const props = editor.getTextPropertiesAtCursor(bufferId);
    let currentIndex = -1;
    if (props.length > 0 && props[0].index !== undefined) {
        currentIndex = props[0].index as number;
    }
    const nextIndex = currentIndex + 1;
    if (nextIndex < state.hunks.length) {
        const hunk = state.hunks[nextIndex];
        editor.setBufferCursor(bufferId, hunk.byteOffset);
    }
};

globalThis.review_prev_hunk = () => {
    const bufferId = editor.getActiveBufferId();
    const props = editor.getTextPropertiesAtCursor(bufferId);
    let currentIndex = state.hunks.length;
    if (props.length > 0 && props[0].index !== undefined) {
        currentIndex = props[0].index as number;
    }
    const prevIndex = currentIndex - 1;
    if (prevIndex >= 0) {
        const hunk = state.hunks[prevIndex];
        editor.setBufferCursor(bufferId, hunk.byteOffset);
    }
};

globalThis.review_refresh = () => {
    refreshReviewStreamBuffer();
};

/**
 * Side-by-Side Diff State
 */
interface DiffViewState {
    leftBufferId: number;
    rightBufferId: number;
    leftSplitId: number;
    rightSplitId: number;
}

let activeDiffView: DiffViewState | null = null;

globalThis.on_viewport_changed = (data: any) => {
    if (!activeDiffView) return;
    if (data.split_id === activeDiffView.leftSplitId) {
        (editor as any).setSplitScroll(activeDiffView.rightSplitId, data.top_byte);
    } else if (data.split_id === activeDiffView.rightSplitId) {
        (editor as any).setSplitScroll(activeDiffView.leftSplitId, data.top_byte);
    }
};

globalThis.review_drill_down = async () => {
    const bufferId = editor.getActiveBufferId();
    const props = editor.getTextPropertiesAtCursor(bufferId);
    if (props.length > 0 && props[0].hunkId) {
        const hunkId = props[0].hunkId as string;
        const hunk = state.hunks.find(h => h.id === hunkId);
        if (!hunk) return;

        const gitShow = await editor.spawnProcess("git", ["show", `HEAD:${hunk.file}`]);
        if (gitShow.exit_code !== 0) return;

        const leftBufferId = await editor.createVirtualBuffer({
            name: `HEAD:${hunk.file}`,
            mode: "special",
            read_only: true,
            entries: [{ text: gitShow.stdout, properties: {} }],
            show_line_numbers: true
        });

        editor.openFile(hunk.file, hunk.range.start, 0);
        const rightBufferId = editor.getActiveBufferId();
        const rightSplitId = (editor as any).getActiveSplitId();

        const leftResult = await editor.createVirtualBufferInSplit({
            name: `HEAD:${hunk.file}`,
            mode: "special",
            read_only: true,
            entries: [{ text: gitShow.stdout, properties: {} }],
            ratio: 0.5,
            direction: "vertical",
            show_line_numbers: true
        });

        activeDiffView = {
            leftBufferId: leftResult.buffer_id,
            rightBufferId: rightBufferId,
            leftSplitId: leftResult.split_id!,
            rightSplitId: rightSplitId
        };

        editor.on("viewport_changed", "on_viewport_changed");
    }
};

// --- Initialization ---

globalThis.start_review_diff = async () => {
    editor.setStatus("Generating Review Diff Stream...");
    editor.setContext("review-mode", true);

    await updateHunks();

    const bufferId = await VirtualBufferFactory.create({
        name: "*Review Diff*",
        mode: "review-mode",
        read_only: true,
        entries: renderReviewStream().entries,
        showLineNumbers: false
    });

    state.reviewBufferId = bufferId;
    refreshReviewStream(); 
    editor.setStatus(`Review Diff Mode Active. Found ${state.hunks.length} hunks. Press 'r' to refresh.`);

    editor.on("buffer_activated", "on_review_buffer_activated");
    editor.on("buffer_closed", "on_review_buffer_closed");
};

globalThis.stop_review_diff = () => {
    state.reviewBufferId = null;
    editor.setContext("review-mode", false);
    editor.off("buffer_activated", "on_review_buffer_activated");
    editor.off("buffer_closed", "on_review_buffer_closed");
    editor.setStatus("Review Diff Mode stopped.");
};

globalThis.on_review_buffer_activated = (data: any) => {
    if (data.buffer_id === state.reviewBufferId) {
        refreshReviewStreamBuffer();
    }
};

globalThis.on_review_buffer_closed = (data: any) => {
    if (data.buffer_id === state.reviewBufferId) {
        stop_review_diff();
    }
};

// Register Modes and Commands
editor.registerCommand("Review Diff", "Start code review session", "start_review_diff", "global");
editor.registerCommand("Stop Review Diff", "Stop the review session", "stop_review_diff", "review-mode");
editor.registerCommand("Refresh Review Diff", "Refresh the list of changes", "review_refresh", "review-mode");

editor.on("buffer_closed", "on_buffer_closed");

editor.defineMode("review-mode", "normal", [
    ["s", "review_stage_hunk"],
    ["d", "review_discard_hunk"],
    ["u", "review_undo_action"],
    ["n", "review_next_hunk"],
    ["p", "review_prev_hunk"],
    ["r", "review_refresh"],
    ["Enter", "review_drill_down"],
    ["q", "close_buffer"],
], true);

editor.debug("Review Diff plugin loaded");