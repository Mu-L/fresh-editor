//! Per-window buffer storage. The inner map is module-private so
//! every add/remove goes through this surface — that's the seam where
//! the "every BufferId reachable from the split tree is in here"
//! invariant (issue #1939) will eventually be enforced.

use fresh_core::BufferId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::state::EditorState;

pub struct WindowBuffers {
    map: HashMap<BufferId, EditorState>,
}

impl WindowBuffers {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, id: &BufferId) -> Option<&EditorState> {
        self.map.get(id)
    }

    pub fn get_mut(&mut self, id: &BufferId) -> Option<&mut EditorState> {
        self.map.get_mut(id)
    }

    pub fn insert(&mut self, id: BufferId, state: EditorState) -> Option<EditorState> {
        self.map.insert(id, state)
    }

    pub fn remove(&mut self, id: &BufferId) -> Option<EditorState> {
        self.map.remove(id)
    }

    pub fn contains_key(&self, id: &BufferId) -> bool {
        self.map.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, BufferId, EditorState> {
        self.map.iter()
    }

    /// Owned snapshot of every buffer id — for callers that need to
    /// mutate `self` while iterating.
    pub fn ids(&self) -> Vec<BufferId> {
        self.map.keys().copied().collect()
    }

    pub fn find_id<F>(&self, mut predicate: F) -> Option<BufferId>
    where
        F: FnMut(BufferId, &EditorState) -> bool,
    {
        self.map
            .iter()
            .find(|(id, state)| predicate(**id, state))
            .map(|(id, _)| *id)
    }

    pub fn count_where<F>(&self, mut predicate: F) -> usize
    where
        F: FnMut(BufferId, &EditorState) -> bool,
    {
        self.map
            .iter()
            .filter(|(id, state)| predicate(**id, state))
            .count()
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.map
            .values()
            .filter_map(|state| state.buffer.file_path().map(PathBuf::from))
            .collect()
    }

    pub fn languages(&self) -> HashSet<String> {
        self.map
            .values()
            .map(|state| state.language.clone())
            .collect()
    }

    pub fn any_needs_semantic_redraw(&self) -> bool {
        self.map.values().any(|state| {
            state
                .reference_highlight_overlay
                .needs_redraw()
                .is_some_and(|remaining| remaining.is_zero())
        })
    }
}

impl Default for WindowBuffers {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a WindowBuffers {
    type Item = (&'a BufferId, &'a EditorState);
    type IntoIter = std::collections::hash_map::Iter<'a, BufferId, EditorState>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a> IntoIterator for &'a mut WindowBuffers {
    type Item = (&'a BufferId, &'a mut EditorState);
    type IntoIter = std::collections::hash_map::IterMut<'a, BufferId, EditorState>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }
}
