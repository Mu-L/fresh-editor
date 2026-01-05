//! Pure action and context types (WASM-compatible)
//!
//! These types define editor actions and contexts without any
//! platform-specific dependencies.

use std::collections::HashMap;

/// Context in which a keybinding is active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyContext {
    /// Global bindings that work in all contexts (checked first with highest priority)
    Global,
    /// Normal editing mode
    Normal,
    /// Prompt/minibuffer is active
    Prompt,
    /// Popup window is visible
    Popup,
    /// File explorer has focus
    FileExplorer,
    /// Menu bar is active
    Menu,
    /// Terminal has focus
    Terminal,
    /// Settings modal is active
    Settings,
}

impl KeyContext {
    /// Check if a context should allow input
    pub fn allows_text_input(&self) -> bool {
        matches!(self, Self::Normal | Self::Prompt)
    }

    /// Parse context from a "when" string
    pub fn from_when_clause(when: &str) -> Option<Self> {
        Some(match when.trim() {
            "global" => Self::Global,
            "prompt" => Self::Prompt,
            "popup" => Self::Popup,
            "fileExplorer" | "file_explorer" => Self::FileExplorer,
            "normal" => Self::Normal,
            "menu" => Self::Menu,
            "terminal" => Self::Terminal,
            "settings" => Self::Settings,
            _ => return None,
        })
    }

    /// Convert context to "when" clause string
    pub fn to_when_clause(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Normal => "normal",
            Self::Prompt => "prompt",
            Self::Popup => "popup",
            Self::FileExplorer => "fileExplorer",
            Self::Menu => "menu",
            Self::Terminal => "terminal",
            Self::Settings => "settings",
        }
    }
}

/// High-level actions that can be performed in the editor
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    // Character input
    InsertChar(char),
    InsertNewline,
    InsertTab,

    // Basic movement
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveWordLeft,
    MoveWordRight,
    MoveLineStart,
    MoveLineEnd,
    MovePageUp,
    MovePageDown,
    MoveDocumentStart,
    MoveDocumentEnd,

    // Selection movement (extends selection while moving)
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectWordLeft,
    SelectWordRight,
    SelectLineStart,
    SelectLineEnd,
    SelectDocumentStart,
    SelectDocumentEnd,
    SelectPageUp,
    SelectPageDown,
    SelectAll,
    SelectWord,
    SelectLine,
    ExpandSelection,

    // Block/rectangular selection (column-wise)
    BlockSelectLeft,
    BlockSelectRight,
    BlockSelectUp,
    BlockSelectDown,

    // Editing
    DeleteBackward,
    DeleteForward,
    DeleteWordBackward,
    DeleteWordForward,
    DeleteLine,
    DeleteToLineEnd,
    DeleteToLineStart,
    TransposeChars,
    OpenLine,

    // View
    Recenter,

    // Selection
    SetMark,

    // Clipboard
    Copy,
    CopyWithTheme(String),
    Cut,
    Paste,

    // Vi-style yank (copy without selection, then restore cursor)
    YankWordForward,
    YankWordBackward,
    YankToLineEnd,
    YankToLineStart,

    // Multi-cursor
    AddCursorAbove,
    AddCursorBelow,
    AddCursorNextMatch,
    RemoveSecondaryCursors,

    // File operations
    Save,
    SaveAs,
    Open,
    SwitchProject,
    New,
    Close,
    CloseTab,
    Quit,
    Revert,
    ToggleAutoRevert,
    FormatBuffer,

    // Navigation
    GotoLine,
    GoToMatchingBracket,
    JumpToNextError,
    JumpToPreviousError,

    // Smart editing
    SmartHome,
    DedentSelection,
    ToggleComment,

    // Bookmarks
    SetBookmark(char),
    JumpToBookmark(char),
    ClearBookmark(char),
    ListBookmarks,

    // Search options
    ToggleSearchCaseSensitive,
    ToggleSearchWholeWord,
    ToggleSearchRegex,
    ToggleSearchConfirmEach,

    // Macros
    StartMacroRecording,
    StopMacroRecording,
    PlayMacro(char),
    ToggleMacroRecording(char),
    ShowMacro(char),
    ListMacros,
    PromptRecordMacro,
    PromptPlayMacro,
    PlayLastMacro,

    // Bookmarks (prompt-based)
    PromptSetBookmark,
    PromptJumpToBookmark,

    // Undo/redo
    Undo,
    Redo,

    // View
    ScrollUp,
    ScrollDown,
    ShowHelp,
    ShowKeyboardShortcuts,
    ShowWarnings,
    ShowLspStatus,
    ClearWarnings,
    CommandPalette,
    ToggleLineWrap,
    ToggleComposeMode,
    SetComposeWidth,
    SelectTheme,
    SelectKeybindingMap,
    SelectCursorStyle,
    SelectLocale,

    // Buffer/tab navigation
    NextBuffer,
    PrevBuffer,
    SwitchToPreviousTab,
    SwitchToTabByName,

    // Tab scrolling
    ScrollTabsLeft,
    ScrollTabsRight,

    // Position history navigation
    NavigateBack,
    NavigateForward,

    // Split view operations
    SplitHorizontal,
    SplitVertical,
    CloseSplit,
    NextSplit,
    PrevSplit,
    IncreaseSplitSize,
    DecreaseSplitSize,
    ToggleMaximizeSplit,

    // Prompt mode actions
    PromptConfirm,
    PromptCancel,
    PromptBackspace,
    PromptDelete,
    PromptMoveLeft,
    PromptMoveRight,
    PromptMoveStart,
    PromptMoveEnd,
    PromptSelectPrev,
    PromptSelectNext,
    PromptPageUp,
    PromptPageDown,
    PromptAcceptSuggestion,
    PromptMoveWordLeft,
    PromptMoveWordRight,
    // Advanced prompt editing (word operations, clipboard)
    PromptDeleteWordForward,
    PromptDeleteWordBackward,
    PromptDeleteToLineEnd,
    PromptCopy,
    PromptCut,
    PromptPaste,
    // Prompt selection actions
    PromptMoveLeftSelecting,
    PromptMoveRightSelecting,
    PromptMoveHomeSelecting,
    PromptMoveEndSelecting,
    PromptSelectWordLeft,
    PromptSelectWordRight,
    PromptSelectAll,

    // File browser actions
    FileBrowserToggleHidden,

    // Popup mode actions
    PopupSelectNext,
    PopupSelectPrev,
    PopupPageUp,
    PopupPageDown,
    PopupConfirm,
    PopupCancel,

    // File explorer operations
    ToggleFileExplorer,
    // Menu bar visibility
    ToggleMenuBar,
    FocusFileExplorer,
    FocusEditor,
    FileExplorerUp,
    FileExplorerDown,
    FileExplorerPageUp,
    FileExplorerPageDown,
    FileExplorerExpand,
    FileExplorerCollapse,
    FileExplorerOpen,
    FileExplorerRefresh,
    FileExplorerNewFile,
    FileExplorerNewDirectory,
    FileExplorerDelete,
    FileExplorerRename,
    FileExplorerToggleHidden,
    FileExplorerToggleGitignored,

    // LSP operations
    LspCompletion,
    LspGotoDefinition,
    LspReferences,
    LspRename,
    LspHover,
    LspSignatureHelp,
    LspCodeActions,
    LspRestart,
    LspStop,
    ToggleInlayHints,
    ToggleMouseHover,

    // View toggles
    ToggleLineNumbers,
    ToggleMouseCapture,
    ToggleDebugHighlights, // Debug mode: show highlight/overlay byte ranges
    SetBackground,
    SetBackgroundBlend,

    // Buffer settings (per-buffer overrides)
    SetTabSize,
    SetLineEnding,
    ToggleIndentationStyle,
    ToggleTabIndicators,
    ResetBufferSettings,

    // Config operations
    DumpConfig,

    // Search and replace
    Search,
    FindInSelection,
    FindNext,
    FindPrevious,
    FindSelectionNext,     // Quick find next occurrence of selection (Ctrl+F3)
    FindSelectionPrevious, // Quick find previous occurrence of selection (Ctrl+Shift+F3)
    Replace,
    QueryReplace, // Interactive replace (y/n/!/q for each match)

    // Menu navigation
    MenuActivate,     // Open menu bar (Alt or F10)
    MenuClose,        // Close menu (Esc)
    MenuLeft,         // Navigate to previous menu
    MenuRight,        // Navigate to next menu
    MenuUp,           // Navigate to previous item in menu
    MenuDown,         // Navigate to next item in menu
    MenuExecute,      // Execute selected menu item (Enter)
    MenuOpen(String), // Open a specific menu by name (e.g., "File", "Edit")

    // Keybinding map switching
    SwitchKeybindingMap(String), // Switch to a named keybinding map (e.g., "default", "emacs", "vscode")

    // Plugin custom actions
    PluginAction(String),

    // Settings operations
    OpenSettings,        // Open the settings modal
    CloseSettings,       // Close the settings modal
    SettingsSave,        // Save settings changes
    SettingsReset,       // Reset current setting to default
    SettingsToggleFocus, // Toggle focus between category and settings panels
    SettingsActivate,    // Activate/toggle the current setting
    SettingsSearch,      // Start search in settings
    SettingsHelp,        // Show settings help overlay
    SettingsIncrement,   // Increment number value or next dropdown option
    SettingsDecrement,   // Decrement number value or previous dropdown option

    // Terminal operations
    OpenTerminal,          // Open a new terminal in the current split
    CloseTerminal,         // Close the current terminal
    FocusTerminal,         // Focus the terminal buffer (if viewing terminal, focus input)
    TerminalEscape,        // Escape from terminal mode back to editor
    ToggleKeyboardCapture, // Toggle keyboard capture mode (all keys go to terminal)
    TerminalPaste,         // Paste clipboard contents into terminal as a single batch

    // Shell command operations
    ShellCommand,        // Run shell command on buffer/selection, output to new buffer
    ShellCommandReplace, // Run shell command on buffer/selection, replace content

    // Case conversion
    ToUpperCase, // Convert selection to uppercase
    ToLowerCase, // Convert selection to lowercase

    // Input calibration
    CalibrateInput, // Open the input calibration wizard

    // No-op
    None,
}

impl Action {
    fn with_char(
        args: &HashMap<String, serde_json::Value>,
        make_action: impl FnOnce(char) -> Self,
    ) -> Option<Self> {
        if let Some(serde_json::Value::String(value)) = args.get("char") {
            value.chars().next().map(make_action)
        } else {
            None
        }
    }

    /// Parse action from string (used when loading from config)
    pub fn from_str(s: &str, args: &HashMap<String, serde_json::Value>) -> Option<Self> {
        Some(match s {
            "insert_char" => return Self::with_char(args, Self::InsertChar),
            "insert_newline" => Self::InsertNewline,
            "insert_tab" => Self::InsertTab,

            "move_left" => Self::MoveLeft,
            "move_right" => Self::MoveRight,
            "move_up" => Self::MoveUp,
            "move_down" => Self::MoveDown,
            "move_word_left" => Self::MoveWordLeft,
            "move_word_right" => Self::MoveWordRight,
            "move_line_start" => Self::MoveLineStart,
            "move_line_end" => Self::MoveLineEnd,
            "move_page_up" => Self::MovePageUp,
            "move_page_down" => Self::MovePageDown,
            "move_document_start" => Self::MoveDocumentStart,
            "move_document_end" => Self::MoveDocumentEnd,

            "select_left" => Self::SelectLeft,
            "select_right" => Self::SelectRight,
            "select_up" => Self::SelectUp,
            "select_down" => Self::SelectDown,
            "select_word_left" => Self::SelectWordLeft,
            "select_word_right" => Self::SelectWordRight,
            "select_line_start" => Self::SelectLineStart,
            "select_line_end" => Self::SelectLineEnd,
            "select_document_start" => Self::SelectDocumentStart,
            "select_document_end" => Self::SelectDocumentEnd,
            "select_page_up" => Self::SelectPageUp,
            "select_page_down" => Self::SelectPageDown,
            "select_all" => Self::SelectAll,
            "select_word" => Self::SelectWord,
            "select_line" => Self::SelectLine,
            "expand_selection" => Self::ExpandSelection,

            // Block/rectangular selection
            "block_select_left" => Self::BlockSelectLeft,
            "block_select_right" => Self::BlockSelectRight,
            "block_select_up" => Self::BlockSelectUp,
            "block_select_down" => Self::BlockSelectDown,

            "delete_backward" => Self::DeleteBackward,
            "delete_forward" => Self::DeleteForward,
            "delete_word_backward" => Self::DeleteWordBackward,
            "delete_word_forward" => Self::DeleteWordForward,
            "delete_line" => Self::DeleteLine,
            "delete_to_line_end" => Self::DeleteToLineEnd,
            "delete_to_line_start" => Self::DeleteToLineStart,
            "transpose_chars" => Self::TransposeChars,
            "open_line" => Self::OpenLine,
            "recenter" => Self::Recenter,
            "set_mark" => Self::SetMark,

            "copy" => Self::Copy,
            "copy_with_theme" => {
                // Empty theme = open theme picker prompt
                let theme = args.get("theme").and_then(|v| v.as_str()).unwrap_or("");
                Self::CopyWithTheme(theme.to_string())
            }
            "cut" => Self::Cut,
            "paste" => Self::Paste,

            // Vi-style yank actions
            "yank_word_forward" => Self::YankWordForward,
            "yank_word_backward" => Self::YankWordBackward,
            "yank_to_line_end" => Self::YankToLineEnd,
            "yank_to_line_start" => Self::YankToLineStart,

            "add_cursor_above" => Self::AddCursorAbove,
            "add_cursor_below" => Self::AddCursorBelow,
            "add_cursor_next_match" => Self::AddCursorNextMatch,
            "remove_secondary_cursors" => Self::RemoveSecondaryCursors,

            "save" => Self::Save,
            "save_as" => Self::SaveAs,
            "open" => Self::Open,
            "switch_project" => Self::SwitchProject,
            "new" => Self::New,
            "close" => Self::Close,
            "close_tab" => Self::CloseTab,
            "quit" => Self::Quit,
            "revert" => Self::Revert,
            "toggle_auto_revert" => Self::ToggleAutoRevert,
            "format_buffer" => Self::FormatBuffer,
            "goto_line" => Self::GotoLine,
            "goto_matching_bracket" => Self::GoToMatchingBracket,
            "jump_to_next_error" => Self::JumpToNextError,
            "jump_to_previous_error" => Self::JumpToPreviousError,

            "smart_home" => Self::SmartHome,
            "dedent_selection" => Self::DedentSelection,
            "toggle_comment" => Self::ToggleComment,

            "set_bookmark" => return Self::with_char(args, Self::SetBookmark),
            "jump_to_bookmark" => return Self::with_char(args, Self::JumpToBookmark),
            "clear_bookmark" => return Self::with_char(args, Self::ClearBookmark),

            "list_bookmarks" => Self::ListBookmarks,

            "toggle_search_case_sensitive" => Self::ToggleSearchCaseSensitive,
            "toggle_search_whole_word" => Self::ToggleSearchWholeWord,
            "toggle_search_regex" => Self::ToggleSearchRegex,
            "toggle_search_confirm_each" => Self::ToggleSearchConfirmEach,

            "start_macro_recording" => Self::StartMacroRecording,
            "stop_macro_recording" => Self::StopMacroRecording,
            "play_macro" => return Self::with_char(args, Self::PlayMacro),
            "toggle_macro_recording" => return Self::with_char(args, Self::ToggleMacroRecording),

            "show_macro" => return Self::with_char(args, Self::ShowMacro),

            "list_macros" => Self::ListMacros,
            "prompt_record_macro" => Self::PromptRecordMacro,
            "prompt_play_macro" => Self::PromptPlayMacro,
            "play_last_macro" => Self::PlayLastMacro,
            "prompt_set_bookmark" => Self::PromptSetBookmark,
            "prompt_jump_to_bookmark" => Self::PromptJumpToBookmark,

            "undo" => Self::Undo,
            "redo" => Self::Redo,

            "scroll_up" => Self::ScrollUp,
            "scroll_down" => Self::ScrollDown,
            "show_help" => Self::ShowHelp,
            "keyboard_shortcuts" => Self::ShowKeyboardShortcuts,
            "show_warnings" => Self::ShowWarnings,
            "show_lsp_status" => Self::ShowLspStatus,
            "clear_warnings" => Self::ClearWarnings,
            "command_palette" => Self::CommandPalette,
            "toggle_line_wrap" => Self::ToggleLineWrap,
            "toggle_compose_mode" => Self::ToggleComposeMode,
            "set_compose_width" => Self::SetComposeWidth,

            "next_buffer" => Self::NextBuffer,
            "prev_buffer" => Self::PrevBuffer,

            "navigate_back" => Self::NavigateBack,
            "navigate_forward" => Self::NavigateForward,

            "split_horizontal" => Self::SplitHorizontal,
            "split_vertical" => Self::SplitVertical,
            "close_split" => Self::CloseSplit,
            "next_split" => Self::NextSplit,
            "prev_split" => Self::PrevSplit,
            "increase_split_size" => Self::IncreaseSplitSize,
            "decrease_split_size" => Self::DecreaseSplitSize,
            "toggle_maximize_split" => Self::ToggleMaximizeSplit,

            "prompt_confirm" => Self::PromptConfirm,
            "prompt_cancel" => Self::PromptCancel,
            "prompt_backspace" => Self::PromptBackspace,
            "prompt_move_left" => Self::PromptMoveLeft,
            "prompt_move_right" => Self::PromptMoveRight,
            "prompt_move_start" => Self::PromptMoveStart,
            "prompt_move_end" => Self::PromptMoveEnd,
            "prompt_select_prev" => Self::PromptSelectPrev,
            "prompt_select_next" => Self::PromptSelectNext,
            "prompt_page_up" => Self::PromptPageUp,
            "prompt_page_down" => Self::PromptPageDown,
            "prompt_accept_suggestion" => Self::PromptAcceptSuggestion,
            "prompt_delete_word_forward" => Self::PromptDeleteWordForward,
            "prompt_delete_word_backward" => Self::PromptDeleteWordBackward,
            "prompt_delete_to_line_end" => Self::PromptDeleteToLineEnd,
            "prompt_copy" => Self::PromptCopy,
            "prompt_cut" => Self::PromptCut,
            "prompt_paste" => Self::PromptPaste,
            "prompt_move_left_selecting" => Self::PromptMoveLeftSelecting,
            "prompt_move_right_selecting" => Self::PromptMoveRightSelecting,
            "prompt_move_home_selecting" => Self::PromptMoveHomeSelecting,
            "prompt_move_end_selecting" => Self::PromptMoveEndSelecting,
            "prompt_select_word_left" => Self::PromptSelectWordLeft,
            "prompt_select_word_right" => Self::PromptSelectWordRight,
            "prompt_select_all" => Self::PromptSelectAll,
            "file_browser_toggle_hidden" => Self::FileBrowserToggleHidden,
            "prompt_move_word_left" => Self::PromptMoveWordLeft,
            "prompt_move_word_right" => Self::PromptMoveWordRight,
            "prompt_delete" => Self::PromptDelete,

            "popup_select_next" => Self::PopupSelectNext,
            "popup_select_prev" => Self::PopupSelectPrev,
            "popup_page_up" => Self::PopupPageUp,
            "popup_page_down" => Self::PopupPageDown,
            "popup_confirm" => Self::PopupConfirm,
            "popup_cancel" => Self::PopupCancel,

            "toggle_file_explorer" => Self::ToggleFileExplorer,
            "toggle_menu_bar" => Self::ToggleMenuBar,
            "focus_file_explorer" => Self::FocusFileExplorer,
            "focus_editor" => Self::FocusEditor,
            "file_explorer_up" => Self::FileExplorerUp,
            "file_explorer_down" => Self::FileExplorerDown,
            "file_explorer_page_up" => Self::FileExplorerPageUp,
            "file_explorer_page_down" => Self::FileExplorerPageDown,
            "file_explorer_expand" => Self::FileExplorerExpand,
            "file_explorer_collapse" => Self::FileExplorerCollapse,
            "file_explorer_open" => Self::FileExplorerOpen,
            "file_explorer_refresh" => Self::FileExplorerRefresh,
            "file_explorer_new_file" => Self::FileExplorerNewFile,
            "file_explorer_new_directory" => Self::FileExplorerNewDirectory,
            "file_explorer_delete" => Self::FileExplorerDelete,
            "file_explorer_rename" => Self::FileExplorerRename,
            "file_explorer_toggle_hidden" => Self::FileExplorerToggleHidden,
            "file_explorer_toggle_gitignored" => Self::FileExplorerToggleGitignored,

            "lsp_completion" => Self::LspCompletion,
            "lsp_goto_definition" => Self::LspGotoDefinition,
            "lsp_references" => Self::LspReferences,
            "lsp_rename" => Self::LspRename,
            "lsp_hover" => Self::LspHover,
            "lsp_signature_help" => Self::LspSignatureHelp,
            "lsp_code_actions" => Self::LspCodeActions,
            "lsp_restart" => Self::LspRestart,
            "lsp_stop" => Self::LspStop,
            "toggle_inlay_hints" => Self::ToggleInlayHints,
            "toggle_mouse_hover" => Self::ToggleMouseHover,

            "toggle_line_numbers" => Self::ToggleLineNumbers,
            "toggle_mouse_capture" => Self::ToggleMouseCapture,
            "toggle_debug_highlights" => Self::ToggleDebugHighlights,
            "set_background" => Self::SetBackground,
            "set_background_blend" => Self::SetBackgroundBlend,
            "select_theme" => Self::SelectTheme,
            "select_keybinding_map" => Self::SelectKeybindingMap,
            "select_locale" => Self::SelectLocale,

            // Buffer settings
            "set_tab_size" => Self::SetTabSize,
            "set_line_ending" => Self::SetLineEnding,
            "toggle_indentation_style" => Self::ToggleIndentationStyle,
            "toggle_tab_indicators" => Self::ToggleTabIndicators,
            "reset_buffer_settings" => Self::ResetBufferSettings,

            "dump_config" => Self::DumpConfig,

            "search" => Self::Search,
            "find_in_selection" => Self::FindInSelection,
            "find_next" => Self::FindNext,
            "find_previous" => Self::FindPrevious,
            "find_selection_next" => Self::FindSelectionNext,
            "find_selection_previous" => Self::FindSelectionPrevious,
            "replace" => Self::Replace,
            "query_replace" => Self::QueryReplace,

            "menu_activate" => Self::MenuActivate,
            "menu_close" => Self::MenuClose,
            "menu_left" => Self::MenuLeft,
            "menu_right" => Self::MenuRight,
            "menu_up" => Self::MenuUp,
            "menu_down" => Self::MenuDown,
            "menu_execute" => Self::MenuExecute,
            "menu_open" => {
                let name = args.get("name")?.as_str()?;
                Self::MenuOpen(name.to_string())
            }

            "switch_keybinding_map" => {
                let map_name = args.get("map")?.as_str()?;
                Self::SwitchKeybindingMap(map_name.to_string())
            }

            // Terminal actions
            "open_terminal" => Self::OpenTerminal,
            "close_terminal" => Self::CloseTerminal,
            "focus_terminal" => Self::FocusTerminal,
            "terminal_escape" => Self::TerminalEscape,
            "toggle_keyboard_capture" => Self::ToggleKeyboardCapture,
            "terminal_paste" => Self::TerminalPaste,

            // Shell command actions
            "shell_command" => Self::ShellCommand,
            "shell_command_replace" => Self::ShellCommandReplace,

            // Case conversion
            "to_upper_case" => Self::ToUpperCase,
            "to_lower_case" => Self::ToLowerCase,

            // Input calibration
            "calibrate_input" => Self::CalibrateInput,

            // Settings actions
            "open_settings" => Self::OpenSettings,
            "close_settings" => Self::CloseSettings,
            "settings_save" => Self::SettingsSave,
            "settings_reset" => Self::SettingsReset,
            "settings_toggle_focus" => Self::SettingsToggleFocus,
            "settings_activate" => Self::SettingsActivate,
            "settings_search" => Self::SettingsSearch,
            "settings_help" => Self::SettingsHelp,
            "settings_increment" => Self::SettingsIncrement,
            "settings_decrement" => Self::SettingsDecrement,

            _ => return None,
        })
    }
}
