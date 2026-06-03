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
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x      < other.right()
            && self.right()  > other.x
            && self.y        < other.bottom()
            && self.bottom() > other.y
    }

    pub fn contains(&self, other: &Rect) -> bool {
        self.x       <= other.x
            && self.y        <= other.y
            && self.right()  >= other.right()
            && self.bottom() >= other.bottom()
    }
}
