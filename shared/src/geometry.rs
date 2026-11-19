//! Zone geometry: keeping sub-zones inside their parent's borders.
//! Coordinates are percentages of the base map (0–100).

use crate::models::Point2D;

/// How far (in map percent) a corner may sit outside its parent's edge and
/// still count as inside — so a sub-zone can share its parent's border.
const EDGE_TOLERANCE: f32 = 0.6;

fn dist_to_segment(p: Point2D, a: Point2D, b: Point2D) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 { 0.0 } else { (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0) };
    let (cx, cy) = (a.x + t * dx, a.y + t * dy);
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}

/// Is `p` inside `poly` (or on / right next to its border)?
pub fn contains(poly: &[Point2D], p: Point2D) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if dist_to_segment(p, a, b) <= EDGE_TOLERANCE {
            return true;
        }
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Indexes of the corners of `inner` that fall outside `outer`.
// shortcut: only corners are tested, so a sub-zone edge could still cut across a
// deep notch of a concave parent; upgrade to edge-intersection tests if that bites.
pub fn corners_outside(inner: &[Point2D], outer: &[Point2D]) -> Vec<usize> {
    inner.iter().enumerate().filter(|(_, p)| !contains(outer, **p)).map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(f32, f32)]) -> Vec<Point2D> {
        v.iter().map(|&(x, y)| Point2D { x, y }).collect()
    }

    #[test]
    fn sub_zone_must_sit_inside_parent() {
        // L-shaped parent: the top-right quarter is cut away.
        let parent = pts(&[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (100.0, 50.0), (100.0, 100.0), (0.0, 100.0)]);
        assert!(corners_outside(&pts(&[(10.0, 10.0), (40.0, 10.0), (40.0, 90.0), (10.0, 90.0)]), &parent).is_empty());
        // Sharing the parent's border is fine.
        assert!(corners_outside(&pts(&[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0), (0.0, 50.0)]), &parent).is_empty());
        // A corner in the cut-away quarter is not.
        assert_eq!(corners_outside(&pts(&[(10.0, 10.0), (80.0, 10.0), (80.0, 90.0), (10.0, 90.0)]), &parent), vec![1]);
    }
}
