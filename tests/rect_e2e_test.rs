use chromiumctl::Rect;

#[test]
fn test_right_equals_x_plus_width() {
    let r = Rect { x: 10.0, y: 20.0, width: 100.0, height: 50.0 };
    assert_eq!(r.right(), 110.0);
}

#[test]
fn test_bottom_equals_y_plus_height() {
    let r = Rect { x: 10.0, y: 20.0, width: 100.0, height: 50.0 };
    assert_eq!(r.bottom(), 70.0);
}

#[test]
fn test_overlaps_returns_true_for_intersecting_rects() {
    let a = Rect { x: 0.0, y: 0.0, width: 100.0, height: 100.0 };
    let b = Rect { x: 50.0, y: 50.0, width: 100.0, height: 100.0 };
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn test_overlaps_returns_false_for_adjacent_rects() {
    let a = Rect { x: 0.0, y: 0.0, width: 100.0, height: 100.0 };
    let b = Rect { x: 100.0, y: 0.0, width: 100.0, height: 100.0 };
    assert!(!a.overlaps(&b));
}

#[test]
fn test_overlaps_returns_false_for_non_overlapping_rects() {
    let a = Rect { x: 0.0, y: 0.0, width: 50.0, height: 50.0 };
    let b = Rect { x: 200.0, y: 200.0, width: 50.0, height: 50.0 };
    assert!(!a.overlaps(&b));
}

#[test]
fn test_contains_returns_true_when_fully_inside() {
    let outer = Rect { x: 0.0, y: 0.0, width: 200.0, height: 200.0 };
    let inner = Rect { x: 50.0, y: 50.0, width: 50.0, height: 50.0 };
    assert!(outer.contains(&inner));
}

#[test]
fn test_contains_returns_false_when_partially_outside() {
    let a = Rect { x: 0.0, y: 0.0, width: 100.0, height: 100.0 };
    let b = Rect { x: 50.0, y: 50.0, width: 100.0, height: 100.0 };
    assert!(!a.contains(&b));
}

#[test]
fn test_contains_returns_true_for_identical_rect() {
    let r = Rect { x: 10.0, y: 10.0, width: 80.0, height: 80.0 };
    assert!(r.contains(&r.clone()));
}
