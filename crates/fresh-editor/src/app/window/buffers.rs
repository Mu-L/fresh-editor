//! Per-window buffer storage. The inner map and split tree are
//! module-private so every add/remove and every multi-field mutation
//! goes through this surface — that's the seam where the "every
//! BufferId reachable from the split tree is in here" invariant
//! (issue #1939) will eventually be enforced.

use fresh_core::BufferId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::model::event::LeafId;
use crate::state::EditorState;
use crate::view::split::{BufferViewState, SplitManager, SplitViewState};

type Splits = (SplitManager, HashMap<LeafId, SplitViewState>);

pub struct WindowBuffers {
    map: HashMap<BufferId, EditorState>,
    splits: Option<Splits>,
}

impl WindowBuffers {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            splits: None,
        }
    }

    // -- single-buffer access --------------------------------------------

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

    /// Read-only handle to the buffer state map, for rendering helpers
    /// that take `&HashMap<BufferId, EditorState>`. Mutation is not
    /// available through this path — go through one of the named
    /// methods.
    pub fn as_map(&self) -> &HashMap<BufferId, EditorState> {
        &self.map
    }

    /// Mutable handle to the buffer state map, for the render path
    /// which needs to walk every buffer and mutate per-render
    /// scratch state. *Doesn't* hand out the splits, so the
    /// "BufferId-in-splits-is-in-buffers" invariant is bounded by
    /// the size of `insert`/`remove`: render won't add or remove
    /// buffers, it just mutates their contents. Prefer the `with_*`
    /// methods at any new call site.
    pub fn as_map_mut(&mut self) -> &mut HashMap<BufferId, EditorState> {
        &mut self.map
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

    // -- splits (read) ---------------------------------------------------

    pub fn splits(&self) -> Option<&Splits> {
        self.splits.as_ref()
    }

    pub fn split_manager(&self) -> Option<&SplitManager> {
        self.splits.as_ref().map(|(m, _)| m)
    }

    pub fn split_view_states(&self) -> Option<&HashMap<LeafId, SplitViewState>> {
        self.splits.as_ref().map(|(_, vs)| vs)
    }

    pub fn has_splits(&self) -> bool {
        self.splits.is_some()
    }

    // -- splits (mut, no buffer interaction) -----------------------------
    //
    // Used alone these are safe; the borrow checker prevents any caller
    // from holding one of these alongside a `get_mut` / `insert` / etc.
    // To touch a buffer state and a split together, use one of the
    // `with_*` methods below.

    pub fn splits_mut(&mut self) -> Option<&mut Splits> {
        self.splits.as_mut()
    }

    pub fn split_manager_mut(&mut self) -> Option<&mut SplitManager> {
        self.splits.as_mut().map(|(m, _)| m)
    }

    pub fn split_view_states_mut(&mut self) -> Option<&mut HashMap<LeafId, SplitViewState>> {
        self.splits.as_mut().map(|(_, vs)| vs)
    }

    pub fn set_splits(&mut self, splits: Splits) {
        self.splits = Some(splits);
    }

    pub fn clear_splits(&mut self) {
        self.splits = None;
    }

    // -- combined mutation (closure-based) -------------------------------
    //
    // These methods own the disjoint sub-borrow internally so callers
    // can't accidentally write a stale `BufferId` into the split tree
    // (or remove a buffer the split tree still points at) — the
    // closure's lifetime is tied to a single owning borrow.

    /// Run `f` with the buffer state and the named split's view state.
    /// Returns `None` if either the buffer or the split is missing.
    pub fn with_buffer_and_split<F, R>(&mut self, buf: BufferId, split: LeafId, f: F) -> Option<R>
    where
        F: FnOnce(&mut EditorState, &mut SplitViewState) -> R,
    {
        let state = self.map.get_mut(&buf)?;
        let (_, vs_map) = self.splits.as_mut()?;
        let vs = vs_map.get_mut(&split)?;
        Some(f(state, vs))
    }

    /// Run `f` with the buffer state and the full per-leaf view-state
    /// map. Used by fold/cursor operations that touch every split
    /// hosting `buf`.
    pub fn with_buffer_and_view_states<F, R>(&mut self, buf: BufferId, f: F) -> Option<R>
    where
        F: FnOnce(&mut EditorState, &mut HashMap<LeafId, SplitViewState>) -> R,
    {
        let state = self.map.get_mut(&buf)?;
        let (_, vs_map) = self.splits.as_mut()?;
        Some(f(state, vs_map))
    }

    /// Run `f` with the buffer state and `buf`'s keyed
    /// `BufferViewState` inside the named split. For buffer-group
    /// panel cursor mutations that target the keyed state rather
    /// than the split's primary cursor.
    pub fn with_buffer_and_split_keyed<F, R>(
        &mut self,
        buf: BufferId,
        split: LeafId,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut EditorState, &mut BufferViewState) -> R,
    {
        let state = self.map.get_mut(&buf)?;
        let (_, vs_map) = self.splits.as_mut()?;
        let vs = vs_map.get_mut(&split)?;
        let keyed = vs.keyed_states.get_mut(&buf)?;
        Some(f(state, keyed))
    }

    /// Run `f` with the named split's view state (mutable) and a
    /// read-only handle to the buffer map. Used by tab-layout and
    /// scrollbar code that walks every buffer's metadata while
    /// mutating one split's tab-scroll offset.
    pub fn with_split_and_buffer_map<F, R>(&mut self, split: LeafId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SplitViewState, &HashMap<BufferId, EditorState>) -> R,
    {
        // Disjoint borrows on the two struct fields.
        let buffer_map = &self.map;
        let (_, vs_map) = self.splits.as_mut()?;
        let vs = vs_map.get_mut(&split)?;
        Some(f(vs, buffer_map))
    }

    /// Run `f` with a single split's view state and the full buffer
    /// map, both mutable. The big-hammer for workspace-restore-style
    /// flows that need to add buffers while configuring tabs/cursors
    /// on a specific split. Closure scope bounds the joint borrow.
    pub fn with_split_and_buffer_map_mut<F, R>(&mut self, split: LeafId, f: F) -> Option<R>
    where
        F: FnOnce(&mut SplitViewState, &mut HashMap<BufferId, EditorState>) -> R,
    {
        let buffer_map = &mut self.map;
        let (_, vs_map) = self.splits.as_mut()?;
        let vs = vs_map.get_mut(&split)?;
        Some(f(vs, buffer_map))
    }

    /// Run `f` with a buffer state (mutable) and a read-only view of
    /// the named split, if present. The status-bar render needs
    /// `&mut state` plus the split's cursor positions to build its
    /// context — splitting the access this way keeps the keyspace
    /// invariant intact (the closure can't insert / remove buffers
    /// while it holds the split ref).
    pub fn with_buffer_mut_and_split_ref<F, R>(
        &mut self,
        buf: BufferId,
        split: LeafId,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut EditorState, Option<&SplitViewState>) -> R,
    {
        let state = self.map.get_mut(&buf)?;
        let vs = self
            .splits
            .as_ref()
            .and_then(|(_, vs_map)| vs_map.get(&split));
        Some(f(state, vs))
    }

    /// Run `f` with mutable refs to the buffer map, the split
    /// manager, and the per-leaf view state map. The render path
    /// and per-frame plugin-state snapshot need all three live at
    /// once — closure scope bounds the joint borrow.
    pub fn with_all_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(
            &mut HashMap<BufferId, EditorState>,
            &mut SplitManager,
            &mut HashMap<LeafId, SplitViewState>,
        ) -> R,
    {
        let buffer_map = &mut self.map;
        let (mgr, vs_map) = self.splits.as_mut()?;
        Some(f(buffer_map, mgr, vs_map))
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
