use crate::cursor::ViewPosition;

/// A selection in view coordinates (start and end positions)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: ViewPosition,
    pub end: ViewPosition,
}

impl Selection {
    /// Create a new selection
    pub fn new(start: ViewPosition, end: ViewPosition) -> Self {
        Self { start, end }
    }

    /// Create from a tuple (for compatibility with existing code)
    pub fn from_tuple(tuple: (ViewPosition, ViewPosition)) -> Self {
        Self {
            start: tuple.0,
            end: tuple.1,
        }
    }

    /// Convert to tuple (for compatibility with existing code)
    pub fn to_tuple(self) -> (ViewPosition, ViewPosition) {
        (self.start, self.end)
    }

    /// Return a normalized selection where start <= end
    pub fn normalized(&self) -> Self {
        if self.start <= self.end {
            *self
        } else {
            Self {
                start: self.end,
                end: self.start,
            }
        }
    }

    /// Check if the selection is empty (start == end)
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the length in view coordinates (approximation)
    pub fn len(&self) -> usize {
        if self.start.view_line == self.end.view_line {
            self.end.column.saturating_sub(self.start.column)
        } else {
            // Multi-line selection: approximate
            let line_diff = self.end.view_line.saturating_sub(self.start.view_line);
            line_diff * 80 + self.end.column
        }
    }
}
