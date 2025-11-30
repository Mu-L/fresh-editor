use clap::Parser;
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        poll as event_poll, read as event_read, Event as CrosstermEvent, KeyEvent, KeyEventKind,
        KeyboardEnhancementFlags, MouseEvent, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fresh::{
    app::script_control::ScriptControlMode, app::Editor, config, services::signal_handler,
};
use ratatui::Terminal;
use std::{
    io::{self, stdout},
    path::PathBuf,
    time::Duration,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// A high-performance terminal text editor
#[derive(Parser, Debug)]
#[command(name = "fresh")]
#[command(about = "A terminal text editor with multi-cursor support", long_about = None)]
#[command(version)]
struct Args {
    /// File to open
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Disable plugin loading
    #[arg(long)]
    no_plugins: bool,

    /// Path to configuration file
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Path to log file for editor diagnostics
    #[arg(long, value_name = "PATH", default_value = "/tmp/editor.log")]
    log_file: PathBuf,

    /// Enable event logging to the specified file
    #[arg(long, value_name = "LOG_FILE")]
    event_log: Option<PathBuf>,

    /// Enable script control mode (accepts JSON commands via stdin, outputs to stdout)
    #[arg(long)]
    script_mode: bool,

    /// Terminal width for script control mode (default: 80)
    #[arg(long, default_value = "80")]
    script_width: u16,

    /// Terminal height for script control mode (default: 24)
    #[arg(long, default_value = "24")]
    script_height: u16,

    /// Print script control mode command schema and exit
    #[arg(long)]
    script_schema: bool,

    /// Don't restore previous session (start fresh)
    #[arg(long)]
    no_session: bool,

    /// Run the application in graphical user interface (GUI) mode using eframe/egui.
    /// Note: This requires the application to be compiled with the 'gui' feature.
    #[arg(long)]
    gui: bool,
}

fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Handle --script-schema flag
    if args.script_schema {
        println!("{}", fresh::app::script_control::get_command_schema());
        return Ok(());
    }

    // Handle GUI mode
    if args.gui {
        return run_gui(&args);
    }

    // Handle script control mode
    if args.script_mode {
        // Initialize tracing for script mode - log to stderr so it doesn't interfere with JSON output on stdout
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(io::stderr))
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
            .init();

        return run_script_control_mode(&args);
    }

    // Initialize tracing - log to a file to avoid interfering with terminal UI
    // Fall back to no logging if the log file can't be created
    if let Ok(log_file) = std::fs::File::create(&args.log_file) {
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::sync::Arc::new(log_file)))
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
            .init();
    }

    tracing::info!("Editor starting");

    // Install signal handlers for SIGTERM and SIGINT
    signal_handler::install_signal_handlers();
    tracing::info!("Signal handlers installed");

    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = stdout().execute(SetCursorStyle::DefaultUserShape);
        let _ = stdout().execute(PopKeyboardEnhancementFlags);
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(panic);
    }));

    // Load configuration
    let config = if let Some(config_path) = &args.config {
        match config::Config::load_from_file(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "Error: Failed to load config from {}: {}",
                    config_path.display(),
                    e
                );
                return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
            }
        }
    } else {
        config::Config::default()
    };

    // Set up terminal first
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    // Enable keyboard enhancement flags to support Shift+Up/Down and other modifier combinations
    // This uses the Kitty keyboard protocol for better key detection in supported terminals
    let keyboard_flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
    let _ = stdout().execute(PushKeyboardEnhancementFlags(keyboard_flags));
    tracing::info!("Enabled keyboard enhancement flags: {:?}", keyboard_flags);

    // Enable mouse support
    let _ = crossterm::execute!(stdout(), crossterm::event::EnableMouseCapture);
    tracing::info!("Enabled mouse capture");

    // Enable blinking block cursor for the primary cursor in active split
    let _ = stdout().execute(SetCursorStyle::BlinkingBlock);
    tracing::info!("Enabled blinking block cursor");

    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Clear the terminal to ensure proper initialization
    terminal.clear()?;

    let size = terminal.size()?;
    tracing::info!("Terminal size: {}x{}", size.width, size.height);

    // Determine if the provided path is a directory or file
    let (working_dir, file_to_open, show_file_explorer) = if let Some(path) = &args.file {
        if path.is_dir() {
            // Path is a directory: use as working dir, don't open any file, show file explorer
            (Some(path.clone()), None, true)
        } else {
            // Path is a file: use current dir as working dir, open the file, don't auto-show explorer
            (None, Some(path.clone()), false)
        }
    } else {
        // No path provided: use current dir, no file, don't auto-show explorer
        (None, None, false)
    };

    // Create editor with actual terminal size and working directory
    let mut editor = if args.no_plugins {
        Editor::with_plugins_disabled(config, size.width, size.height, working_dir)?
    } else {
        Editor::with_working_dir(config, size.width, size.height, working_dir)?
    };

    // Enable event log streaming if requested
    if let Some(log_path) = &args.event_log {
        tracing::trace!("Event logging enabled: {}", log_path.display());
        editor.enable_event_streaming(log_path)?;
    }

    // Try to restore previous session (unless --no-session flag is set or a file was specified)
    let session_enabled = !args.no_session && file_to_open.is_none();
    if session_enabled {
        match editor.try_restore_session() {
            Ok(true) => {
                tracing::info!("Session restored successfully");
            }
            Ok(false) => {
                tracing::debug!("No previous session found");
            }
            Err(e) => {
                tracing::warn!("Failed to restore session: {}", e);
            }
        }
    }

    // Open file if provided (this takes precedence over session)
    if let Some(path) = &file_to_open {
        editor.open_file(path)?;
    }

    // Show file explorer if directory was provided
    if show_file_explorer {
        editor.show_file_explorer();
    }

    // Run the editor
    let result = run_event_loop(&mut editor, &mut terminal, session_enabled);

    // Clean up terminal
    let _ = crossterm::execute!(stdout(), crossterm::event::DisableMouseCapture);
    let _ = stdout().execute(SetCursorStyle::DefaultUserShape);
    let _ = stdout().execute(PopKeyboardEnhancementFlags);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

/// Run the editor in script control mode
fn run_script_control_mode(args: &Args) -> io::Result<()> {
    // Create script control mode instance
    let mut control = if let Some(path) = &args.file {
        if path.is_dir() {
            ScriptControlMode::with_working_dir(
                args.script_width,
                args.script_height,
                path.clone(),
            )?
        } else {
            let mut ctrl = ScriptControlMode::new(args.script_width, args.script_height)?;
            // Open the file if provided
            ctrl.open_file(path)?;
            ctrl
        }
    } else {
        ScriptControlMode::new(args.script_width, args.script_height)?
    };

    control.run()
}

/// Main event loop
fn run_event_loop(
    editor: &mut Editor,
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    session_enabled: bool,
) -> io::Result<()> {
    use std::time::Instant;

    const FRAME_DURATION: Duration = Duration::from_millis(16); // 60fps
    let mut last_render = Instant::now();
    let mut needs_render = true;
    let mut pending_event: Option<CrosstermEvent> = None; // For events read during coalescing

    loop {
        if editor.process_async_messages() {
            needs_render = true;
        }

        if editor.should_quit() {
            // Save session before quitting (if enabled)
            if session_enabled {
                if let Err(e) = editor.save_session() {
                    tracing::warn!("Failed to save session: {}", e);
                } else {
                    tracing::debug!("Session saved successfully");
                }
            }
            break;
        }

        // Render at most 60fps
        if needs_render && last_render.elapsed() >= FRAME_DURATION {
            terminal.draw(|frame| editor.render(frame))?;
            last_render = Instant::now();
            needs_render = false;
        }

        // Get next event
        let event = if let Some(e) = pending_event.take() {
            Some(e)
        } else {
            let timeout = if pending_event.is_some() || needs_render {
                FRAME_DURATION.saturating_sub(last_render.elapsed())
            } else {
                Duration::from_millis(50)
            };
            if event_poll(timeout)? {
                Some(event_read()?)
            } else {
                None
            }
        };

        let Some(event) = event else { continue };

        // Coalesce mouse moves - skip stale ones, keep clicks/keys
        let (event, next) = coalesce_mouse_moves(event)?;
        pending_event = next;

        match event {
            CrosstermEvent::Key(key_event) => {
                // Only process key press events to avoid duplicate events on Windows
                // (Windows sends both Press and Release events, while Linux/macOS only send Press)
                if key_event.kind == KeyEventKind::Press {
                    handle_key_event(editor, key_event)?;
                    needs_render = true;
                }
            }
            CrosstermEvent::Mouse(mouse_event) => {
                if handle_mouse_event(editor, mouse_event)? {
                    needs_render = true;
                }
            }
            CrosstermEvent::Resize(w, h) => {
                editor.resize(w, h);
                needs_render = true;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Handle a keyboard event
fn handle_key_event(editor: &mut Editor, key_event: KeyEvent) -> io::Result<()> {
    // Debug trace the full key event
    tracing::debug!(
        "Key event received: code={:?}, modifiers={:?}, kind={:?}, state={:?}",
        key_event.code,
        key_event.modifiers,
        key_event.kind,
        key_event.state
    );

    // Log the keystroke
    let key_code = format!("{:?}", key_event.code);
    let modifiers = format!("{:?}", key_event.modifiers);
    editor.log_keystroke(&key_code, &modifiers);

    // Delegate to the editor's handle_key method
    editor.handle_key(key_event.code, key_event.modifiers)?;

    Ok(())
}

/// Handle a mouse event
/// Returns true if a re-render is needed
fn handle_mouse_event(editor: &mut Editor, mouse_event: MouseEvent) -> io::Result<bool> {
    tracing::debug!(
        "Mouse event received: kind={:?}, column={}, row={}, modifiers={:?}",
        mouse_event.kind,
        mouse_event.column,
        mouse_event.row,
        mouse_event.modifiers
    );

    // Delegate to the editor's handle_mouse method
    editor.handle_mouse(mouse_event)
}

/// Skip stale mouse move events, return the latest one.
/// If we read a non-move event while draining, return it as pending.
fn coalesce_mouse_moves(
    event: CrosstermEvent,
) -> io::Result<(CrosstermEvent, Option<CrosstermEvent>)> {
    use crossterm::event::MouseEventKind;

    // Only coalesce mouse moves
    if !matches!(&event, CrosstermEvent::Mouse(m) if m.kind == MouseEventKind::Moved) {
        return Ok((event, None));
    }

    let mut latest = event;
    while event_poll(Duration::ZERO)? {
        let next = event_read()?;
        if matches!(&next, CrosstermEvent::Mouse(m) if m.kind == MouseEventKind::Moved) {
            latest = next; // Newer move, skip the old one
        } else {
            return Ok((latest, Some(next))); // Hit a click/key, save it
        }
    }
    Ok((latest, None))
}

/// Run the application in GUI mode using eframe/egui.
/// This function is only available when compiled with the 'gui' feature.
#[cfg(feature = "gui")]
fn run_gui(args: &Args) -> io::Result<()> {
    use eframe::egui;
    use egui_ratatui::RataguiBackend;
    use ratatui::Terminal;
    use soft_ratatui::embedded_graphics_unicodefonts::{
        mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
    };
    use soft_ratatui::{EmbeddedGraphics, SoftBackend};

    // Initialize tracing for GUI mode
    if let Ok(log_file) = std::fs::File::create(&args.log_file) {
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::sync::Arc::new(log_file)))
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
            .init();
    }

    tracing::info!("Starting GUI mode");

    // Install signal handlers
    signal_handler::install_signal_handlers();

    // Load configuration
    let editor_config = if let Some(config_path) = &args.config {
        match config::Config::load_from_file(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "Error: Failed to load config from {}: {}",
                    config_path.display(),
                    e
                );
                return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
            }
        }
    } else {
        config::Config::default()
    };

    // Determine working directory and file to open
    let (working_dir, file_to_open, _show_file_explorer) = if let Some(path) = &args.file {
        if path.is_dir() {
            (Some(path.clone()), None, true)
        } else {
            (None, Some(path.clone()), false)
        }
    } else {
        (None, None, false)
    };

    // Initial terminal size (will be resized based on window)
    let initial_cols = 100u16;
    let initial_rows = 40u16;

    // Create fonts for soft_ratatui
    let font_regular = mono_8x13_atlas();
    let font_bold = mono_8x13_bold_atlas();
    let font_italic = mono_8x13_italic_atlas();

    // Create soft backend
    let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
        initial_cols,
        initial_rows,
        font_regular,
        Some(font_bold),
        Some(font_italic),
    );

    // Create ratatui backend wrapper for egui
    let backend = RataguiBackend::new("fresh_gui", soft_backend);
    let terminal = Terminal::new(backend)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Create editor
    let editor = if args.no_plugins {
        Editor::with_plugins_disabled(editor_config, initial_cols, initial_rows, working_dir)?
    } else {
        Editor::with_working_dir(editor_config, initial_cols, initial_rows, working_dir)?
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Fresh Editor",
        options,
        Box::new(move |_cc| {
            let mut app = GuiApp::new(terminal, editor);
            // Open file if provided
            if let Some(path) = file_to_open {
                if let Err(e) = app.editor.open_file(&path) {
                    tracing::error!("Failed to open file: {}", e);
                }
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

/// GUI application using egui_ratatui to render the editor.
#[cfg(feature = "gui")]
struct GuiApp {
    terminal: Terminal<egui_ratatui::RataguiBackend<soft_ratatui::EmbeddedGraphics>>,
    editor: Editor,
    last_size: (u16, u16),
}

#[cfg(feature = "gui")]
impl GuiApp {
    fn new(
        terminal: Terminal<egui_ratatui::RataguiBackend<soft_ratatui::EmbeddedGraphics>>,
        editor: Editor,
    ) -> Self {
        Self {
            terminal,
            editor,
            last_size: (100, 40),
        }
    }

    fn handle_keyboard_input(&mut self, ctx: &eframe::egui::Context) {
        use crossterm::event::{KeyCode, KeyModifiers};

        ctx.input(|input| {
            for event in &input.events {
                if let eframe::egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } = event
                {
                    if !pressed {
                        continue;
                    }

                    let mut key_modifiers = KeyModifiers::empty();
                    if modifiers.ctrl {
                        key_modifiers |= KeyModifiers::CONTROL;
                    }
                    if modifiers.shift {
                        key_modifiers |= KeyModifiers::SHIFT;
                    }
                    if modifiers.alt {
                        key_modifiers |= KeyModifiers::ALT;
                    }

                    let key_code = match key {
                        eframe::egui::Key::A => KeyCode::Char('a'),
                        eframe::egui::Key::B => KeyCode::Char('b'),
                        eframe::egui::Key::C => KeyCode::Char('c'),
                        eframe::egui::Key::D => KeyCode::Char('d'),
                        eframe::egui::Key::E => KeyCode::Char('e'),
                        eframe::egui::Key::F => KeyCode::Char('f'),
                        eframe::egui::Key::G => KeyCode::Char('g'),
                        eframe::egui::Key::H => KeyCode::Char('h'),
                        eframe::egui::Key::I => KeyCode::Char('i'),
                        eframe::egui::Key::J => KeyCode::Char('j'),
                        eframe::egui::Key::K => KeyCode::Char('k'),
                        eframe::egui::Key::L => KeyCode::Char('l'),
                        eframe::egui::Key::M => KeyCode::Char('m'),
                        eframe::egui::Key::N => KeyCode::Char('n'),
                        eframe::egui::Key::O => KeyCode::Char('o'),
                        eframe::egui::Key::P => KeyCode::Char('p'),
                        eframe::egui::Key::Q => KeyCode::Char('q'),
                        eframe::egui::Key::R => KeyCode::Char('r'),
                        eframe::egui::Key::S => KeyCode::Char('s'),
                        eframe::egui::Key::T => KeyCode::Char('t'),
                        eframe::egui::Key::U => KeyCode::Char('u'),
                        eframe::egui::Key::V => KeyCode::Char('v'),
                        eframe::egui::Key::W => KeyCode::Char('w'),
                        eframe::egui::Key::X => KeyCode::Char('x'),
                        eframe::egui::Key::Y => KeyCode::Char('y'),
                        eframe::egui::Key::Z => KeyCode::Char('z'),
                        eframe::egui::Key::Num0 => KeyCode::Char('0'),
                        eframe::egui::Key::Num1 => KeyCode::Char('1'),
                        eframe::egui::Key::Num2 => KeyCode::Char('2'),
                        eframe::egui::Key::Num3 => KeyCode::Char('3'),
                        eframe::egui::Key::Num4 => KeyCode::Char('4'),
                        eframe::egui::Key::Num5 => KeyCode::Char('5'),
                        eframe::egui::Key::Num6 => KeyCode::Char('6'),
                        eframe::egui::Key::Num7 => KeyCode::Char('7'),
                        eframe::egui::Key::Num8 => KeyCode::Char('8'),
                        eframe::egui::Key::Num9 => KeyCode::Char('9'),
                        eframe::egui::Key::Enter => KeyCode::Enter,
                        eframe::egui::Key::Escape => KeyCode::Esc,
                        eframe::egui::Key::Tab => KeyCode::Tab,
                        eframe::egui::Key::Backspace => KeyCode::Backspace,
                        eframe::egui::Key::Delete => KeyCode::Delete,
                        eframe::egui::Key::ArrowUp => KeyCode::Up,
                        eframe::egui::Key::ArrowDown => KeyCode::Down,
                        eframe::egui::Key::ArrowLeft => KeyCode::Left,
                        eframe::egui::Key::ArrowRight => KeyCode::Right,
                        eframe::egui::Key::Home => KeyCode::Home,
                        eframe::egui::Key::End => KeyCode::End,
                        eframe::egui::Key::PageUp => KeyCode::PageUp,
                        eframe::egui::Key::PageDown => KeyCode::PageDown,
                        eframe::egui::Key::Space => KeyCode::Char(' '),
                        eframe::egui::Key::F1 => KeyCode::F(1),
                        eframe::egui::Key::F2 => KeyCode::F(2),
                        eframe::egui::Key::F3 => KeyCode::F(3),
                        eframe::egui::Key::F4 => KeyCode::F(4),
                        eframe::egui::Key::F5 => KeyCode::F(5),
                        eframe::egui::Key::F6 => KeyCode::F(6),
                        eframe::egui::Key::F7 => KeyCode::F(7),
                        eframe::egui::Key::F8 => KeyCode::F(8),
                        eframe::egui::Key::F9 => KeyCode::F(9),
                        eframe::egui::Key::F10 => KeyCode::F(10),
                        eframe::egui::Key::F11 => KeyCode::F(11),
                        eframe::egui::Key::F12 => KeyCode::F(12),
                        _ => continue,
                    };

                    // Apply shift to letter keys
                    let key_code = if modifiers.shift {
                        match key_code {
                            KeyCode::Char(c) if c.is_ascii_lowercase() => {
                                KeyCode::Char(c.to_ascii_uppercase())
                            }
                            other => other,
                        }
                    } else {
                        key_code
                    };

                    if let Err(e) = self.editor.handle_key(key_code, key_modifiers) {
                        tracing::error!("Error handling key: {}", e);
                    }
                }
            }

            // Handle text input for characters not covered by key events
            for event in &input.events {
                if let eframe::egui::Event::Text(text) = event {
                    for c in text.chars() {
                        // Skip control characters and already-handled keys
                        if c.is_control() || c.is_ascii_alphabetic() || c.is_ascii_digit() || c == ' ' {
                            continue;
                        }
                        if let Err(e) = self.editor.handle_key(KeyCode::Char(c), KeyModifiers::empty()) {
                            tracing::error!("Error handling text input: {}", e);
                        }
                    }
                }
            }
        });
    }
}

#[cfg(feature = "gui")]
impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Process async messages from the editor
        self.editor.process_async_messages();

        // Handle keyboard input
        self.handle_keyboard_input(ctx);

        // Check if we should quit
        if self.editor.should_quit() {
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            return;
        }

        // Render the editor to the terminal
        if let Err(e) = self.terminal.draw(|frame| {
            self.editor.render(frame);
        }) {
            tracing::error!("Failed to draw terminal: {}", e);
        }

        // Display the terminal in egui
        eframe::egui::CentralPanel::default()
            .frame(eframe::egui::Frame::NONE)
            .show(ctx, |ui| {
                // Calculate new size based on available space
                let available = ui.available_size();
                // Font is 8x13 pixels
                let cols = (available.x / 8.0) as u16;
                let rows = (available.y / 13.0) as u16;

                // Resize if needed
                if (cols, rows) != self.last_size && cols > 0 && rows > 0 {
                    self.last_size = (cols, rows);
                    self.editor.resize(cols, rows);

                    // Resize the soft backend
                    self.terminal.backend_mut().soft_backend.resize(cols, rows);
                }

                ui.add(self.terminal.backend_mut());
            });

        // Request continuous repainting for smooth updates
        ctx.request_repaint();
    }
}

/// Fallback stub for run_gui when the 'gui' feature is NOT compiled.
#[cfg(not(feature = "gui"))]
fn run_gui(_args: &Args) -> io::Result<()> {
    eprintln!(
        "Error: The GUI feature was not compiled. Please rebuild with 'cargo build --features gui'."
    );
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "GUI feature not available",
    ))
}
