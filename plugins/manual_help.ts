/// <reference path="../types/fresh.d.ts" />

import { VirtualBufferFactory, AsciiAnimation, FRESH_ART } from "./lib/index.ts";



const MANUAL_MODE = "help-manual";
const SHORTCUTS_MODE = "help-keyboard";

// ANSI color codes for styling
const COLORS = {
  RESET: "\x1b[0m",
  BOLD: "\x1b[1m",
  DIM: "\x1b[2m",
  UNDERLINE: "\x1b[4m",

  // Foreground colors
  CYAN: "\x1b[36m",
  GREEN: "\x1b[32m",
  YELLOW: "\x1b[33m",
  BLUE: "\x1b[34m",
  MAGENTA: "\x1b[35m",
  WHITE: "\x1b[37m",
  BRIGHT_CYAN: "\x1b[96m",
  BRIGHT_GREEN: "\x1b[92m",
  BRIGHT_YELLOW: "\x1b[93m",
  BRIGHT_BLUE: "\x1b[94m",
  BRIGHT_MAGENTA: "\x1b[95m",
};

const createEntriesFromLines = (lines: string[]): TextPropertyEntry[] =>
  lines.map((line) => ({
    text: `${line}\n`,
    properties: {},
  }));

// Animation state
let titleAnimation: AsciiAnimation | null = null;
let manualBufferId: number | null = null;

const buildManualEntries = (titleLines: string[]): TextPropertyEntry[] => {
  const C = COLORS;
  const manualText = [
    "",
    ...titleLines,
    "",
    `${C.BOLD}${C.BRIGHT_YELLOW}The Terminal Text Editor${C.RESET}`,
    "",
    `${C.BOLD}${C.BRIGHT_GREEN}Easy To Use${C.RESET} | ${C.BOLD}${C.BRIGHT_MAGENTA}TypeScript Extensible${C.RESET} | ${C.BOLD}${C.BRIGHT_BLUE}Light And Fast${C.RESET} | ${C.BOLD}${C.BRIGHT_YELLOW}Huge File Support${C.RESET}`,
    "",
    `${C.BOLD}${C.BRIGHT_CYAN}╔════════════════════════════════════════════════════════════╗${C.RESET}`,
    `${C.BOLD}${C.BRIGHT_CYAN}║${C.RESET}                    ${C.BOLD}${C.BRIGHT_YELLOW}FEATURE OVERVIEW${C.RESET}                        ${C.BOLD}${C.BRIGHT_CYAN}║${C.RESET}`,
    `${C.BOLD}${C.BRIGHT_CYAN}╚════════════════════════════════════════════════════════════╝${C.RESET}`,
    "",
    `${C.BOLD}${C.BRIGHT_GREEN}📁 File Management${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.CYAN}•${C.RESET} Open, save, save-as, new file, close buffer, revert`,
    `  ${C.CYAN}•${C.RESET} File explorer sidebar with create/delete/rename`,
    `  ${C.CYAN}•${C.RESET} Tab-based buffer switching with scroll overflow`,
    `  ${C.CYAN}•${C.RESET} Auto-revert when files change on disk`,
    `  ${C.CYAN}•${C.RESET} Git file finder (fuzzy search tracked files)`,
    "",
    `${C.BOLD}${C.BRIGHT_MAGENTA}✏️  Editing${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.MAGENTA}•${C.RESET} Unlimited undo/redo with full history`,
    `  ${C.MAGENTA}•${C.RESET} Multi-cursor editing (add above/below, next match)`,
    `  ${C.MAGENTA}•${C.RESET} Block/column selection mode`,
    `  ${C.MAGENTA}•${C.RESET} Word, line, and expanding selection`,
    `  ${C.MAGENTA}•${C.RESET} Smart indent/dedent and auto-indentation`,
    `  ${C.MAGENTA}•${C.RESET} Toggle line comments (language-aware)`,
    `  ${C.MAGENTA}•${C.RESET} Clipboard with system integration`,
    "",
    `${C.BOLD}${C.BRIGHT_BLUE}🔍 Search & Replace${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.BLUE}•${C.RESET} Incremental search with match highlighting`,
    `  ${C.BLUE}•${C.RESET} Search within selection`,
    `  ${C.BLUE}•${C.RESET} Replace and replace-all`,
    `  ${C.BLUE}•${C.RESET} Interactive query-replace (y/n/!/q per match)`,
    `  ${C.BLUE}•${C.RESET} Git grep across entire repository`,
    "",
    `${C.BOLD}${C.BRIGHT_YELLOW}🧭 Navigation${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.YELLOW}•${C.RESET} Go to line, go to matching bracket`,
    `  ${C.YELLOW}•${C.RESET} Word and document movement`,
    `  ${C.YELLOW}•${C.RESET} Position history (back/forward)`,
    `  ${C.YELLOW}•${C.RESET} Numbered bookmarks (0-9) with jump`,
    `  ${C.YELLOW}•${C.RESET} Jump to next/previous error`,
    "",
    `${C.BOLD}${C.BRIGHT_CYAN}🖥️  Views & Layout${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.CYAN}•${C.RESET} Horizontal and vertical split panes`,
    `  ${C.CYAN}•${C.RESET} Resizable splits with keyboard/mouse`,
    `  ${C.CYAN}•${C.RESET} Toggle line numbers, line wrap, hidden files`,
    `  ${C.CYAN}•${C.RESET} ANSI art background images with blend control`,
    `  ${C.CYAN}•${C.RESET} Markdown compose/preview mode`,
    "",
    `${C.BOLD}${C.BRIGHT_GREEN}🤖 Language Server (LSP)${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.GREEN}•${C.RESET} Go to definition, find references`,
    `  ${C.GREEN}•${C.RESET} Hover documentation and signature help`,
    `  ${C.GREEN}•${C.RESET} Code actions and quick fixes`,
    `  ${C.GREEN}•${C.RESET} Rename symbol across project`,
    `  ${C.GREEN}•${C.RESET} Real-time diagnostics (errors/warnings)`,
    `  ${C.GREEN}•${C.RESET} Autocompletion with snippets`,
    "",
    `${C.BOLD}${C.BRIGHT_MAGENTA}🎯 Productivity${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.MAGENTA}•${C.RESET} Command palette (${C.YELLOW}Ctrl+P${C.RESET}) for all actions`,
    `  ${C.MAGENTA}•${C.RESET} Full menu bar with mouse/keyboard navigation`,
    `  ${C.MAGENTA}•${C.RESET} Keyboard macros (record/playback, slots 0-9)`,
    `  ${C.MAGENTA}•${C.RESET} Git log viewer with diff display`,
    `  ${C.MAGENTA}•${C.RESET} Diagnostics panel for all errors`,
    "",
    `${C.BOLD}${C.BRIGHT_BLUE}🔌 Plugins & Extensibility${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.BLUE}•${C.RESET} TypeScript plugins in sandboxed Deno runtime`,
    `  ${C.BLUE}•${C.RESET} Color highlighter (hex/rgb colors in code)`,
    `  ${C.BLUE}•${C.RESET} TODO/FIXME highlighter`,
    `  ${C.BLUE}•${C.RESET} Git merge conflict resolver`,
    `  ${C.BLUE}•${C.RESET} Path autocomplete`,
    `  ${C.BLUE}•${C.RESET} Customizable keymaps (JSON config)`,
    "",
    `${C.BOLD}${C.BRIGHT_CYAN}╔════════════════════════════════════════════════════════════╗${C.RESET}`,
    `${C.BOLD}${C.BRIGHT_CYAN}║${C.RESET}                    ${C.BOLD}${C.BRIGHT_YELLOW}GETTING STARTED${C.RESET}                         ${C.BOLD}${C.BRIGHT_CYAN}║${C.RESET}`,
    `${C.BOLD}${C.BRIGHT_CYAN}╚════════════════════════════════════════════════════════════╝${C.RESET}`,
    "",
    `${C.BOLD}${C.BRIGHT_GREEN}⭐ Quick Start${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.CYAN}•${C.RESET} ${C.BOLD}Open Files:${C.RESET} Press ${C.YELLOW}Ctrl+O${C.RESET} to browse and open any file`,
    `  ${C.CYAN}•${C.RESET} ${C.BOLD}Command Palette:${C.RESET} Hit ${C.YELLOW}Ctrl+P${C.RESET} - your Swiss Army knife!`,
    `  ${C.CYAN}•${C.RESET} ${C.BOLD}Use the Mouse!${C.RESET} Scroll bars, menus, tabs all work`,
    `  ${C.CYAN}•${C.RESET} ${C.BOLD}Menu Bar:${C.RESET} ${C.YELLOW}Alt+F${C.RESET} for File, ${C.YELLOW}Alt+E${C.RESET} for Edit, etc.`,
    "",
    `${C.BOLD}${C.BRIGHT_YELLOW}⭐ Pro Tips${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  ${C.YELLOW}•${C.RESET} ${C.BOLD}Multi-cursor:${C.RESET} ${C.YELLOW}Ctrl+D${C.RESET} adds cursor at next match`,
    `  ${C.YELLOW}•${C.RESET} ${C.BOLD}Search:${C.RESET} ${C.YELLOW}Ctrl+F${C.RESET} finds, ${C.YELLOW}F3${C.RESET}/${C.YELLOW}Shift+F3${C.RESET} navigates matches`,
    `  ${C.YELLOW}•${C.RESET} ${C.BOLD}File Explorer:${C.RESET} ${C.YELLOW}Ctrl+B${C.RESET} toggles sidebar`,
    `  ${C.YELLOW}•${C.RESET} ${C.BOLD}Splits:${C.RESET} Use View menu or command palette`,
    "",
    `${C.BOLD}${C.WHITE}📚 Documentation${C.RESET}`,
    `${C.DIM}────────────────────────────────────────────────────────────${C.RESET}`,
    `  • ${C.GREEN}README.md${C.RESET} - Quick start guide`,
    `  • ${C.GREEN}docs/USER_GUIDE.md${C.RESET} - Comprehensive documentation`,
    `  • ${C.GREEN}docs/PLUGIN_DEVELOPMENT.md${C.RESET} - Build your own plugins`,
    "",
    `${C.DIM}Press ${C.YELLOW}q${C.RESET}${C.DIM} or ${C.YELLOW}Esc${C.RESET}${C.DIM} to close | ${C.YELLOW}Shift+F1${C.RESET}${C.DIM} for keyboard shortcuts${C.RESET}`,
  ];
  return createEntriesFromLines(manualText);
};




const openVirtualBuffer = async (
  name: string,
  entries: TextPropertyEntry[],
  mode: string,
): Promise<void> => {
  try {
    await VirtualBufferFactory.create({
      name,
      mode,
      entries,
      showLineNumbers: false,
      editingDisabled: true,
      readOnly: true,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    editor.setStatus(`Failed to open ${name}: ${message}`);
  }
};

const openManual = async (): Promise<void> => {
  // Create animation for the FRESH title
  titleAnimation = new AsciiAnimation(FRESH_ART);

  // Get initial animated frame (starts pixelated, wave at far left)
  const initialTitleLines = titleAnimation.render();
  const entries = buildManualEntries(initialTitleLines);

  try {
    manualBufferId = await VirtualBufferFactory.create({
      name: "*Fresh Manual*",
      mode: MANUAL_MODE,
      entries,
      showLineNumbers: false,
      editingDisabled: true,
      readOnly: true,
    });

    // Start animation loop
    titleAnimation.start((titleLines) => {
      if (manualBufferId !== null) {
        const updatedEntries = buildManualEntries(titleLines);
        VirtualBufferFactory.updateContent(manualBufferId, updatedEntries);
      }
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    editor.setStatus(`Failed to open Fresh Manual: ${message}`);
  }
};

const openShortcuts = async (bindings: { key: string; action: string }[]): Promise<void> => {
  const entries = buildShortcutEntries(bindings);
  await openVirtualBuffer("*Keyboard Shortcuts*", entries, SHORTCUTS_MODE);
};

editor.defineMode(
  MANUAL_MODE,
  null,
  [
    ["q", "manual_help_close"],
    ["Escape", "manual_help_close"],
  ],
  true,
);

editor.defineMode(
  SHORTCUTS_MODE,
  null,
  [
    ["q", "manual_help_close"],
    ["Escape", "manual_help_close"],
  ],
  true,
);

globalThis.manual_help_close = () => {
  // Stop the title animation if running
  if (titleAnimation) {
    titleAnimation.stop();
    titleAnimation = null;
  }
  manualBufferId = null;

  const bufferId = editor.getActiveBufferId();
  editor.closeBuffer(bufferId);
};

globalThis.onManualPage = async (): Promise<boolean> => {
  await openManual();
  return true;
};

globalThis.onKeyboardShortcuts = async (args: {
  bindings: { key: string; action: string }[];
}): Promise<boolean> => {
  await openShortcuts(args.bindings);
  return true;
};

editor.on("manual_page", "onManualPage");
editor.on("keyboard_shortcuts", "onKeyboardShortcuts");

editor.registerCommand(
  "Show Fresh Manual",
  "Open the Fresh manual (virtual buffer)",
  "show_help",
  "normal",
);

editor.registerCommand(
  "Keyboard Shortcuts",
  "Show the keyboard shortcuts list (virtual buffer)",
  "keyboard_shortcuts",
  "normal",
);

editor.debug("Manual/help plugin initialized");
