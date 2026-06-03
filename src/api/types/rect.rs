use serde::Deserialize;

/// Bounding rectangle returned by `getBoundingClientRect()`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rect {
    pub x:      f64,
    pub y:      f64,
    pub width:  f64,
    pub height: f64,
}

impl Rect {
    /// X coordinate of the right edge.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Y coordinate of the bottom edge.
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Returns `true` if this rect overlaps `other`.
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x      < other.right()
            && self.right()  > other.x
            && self.y        < other.bottom()
            && self.bottom() > other.y
    }

    /// Returns `true` if this rect fully contains `other`.
    pub fn contains(&self, other: &Rect) -> bool {
        self.x       <= other.x
            && self.y        <= other.y
            && self.right()  >= other.right()
            && self.bottom() >= other.bottom()
    }
}
