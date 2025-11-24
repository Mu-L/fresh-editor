use crate::event::Event;
use crate::keybindings::Action;
use crate::state::EditorState;

/// View-centric rewrite placeholder: action pipeline to be rebuilt.
pub fn action_to_events(
    _state: &mut EditorState,
    _action: Action,
    _tab_size: usize,
    _auto_indent: bool,
    _estimated_line_length: usize,
) -> Option<Vec<Event>> {
    None
}
