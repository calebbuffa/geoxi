//! Binary spatial predicates and relate patterns.

use crate::internal_prelude::*;

pub fn contains(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.contains(b)
}
pub fn contains_properly(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.prepared()?.contains_properly(b)
}
pub fn covered_by(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.covered_by(b)
}
pub fn covers(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.covers(b)
}
pub fn crosses(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.crosses(b)
}
pub fn disjoint(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.disjoint(b)
}
pub fn dwithin(a: &Geometry, b: &Geometry, distance: f64) -> Result<bool> {
    a.dwithin(b, distance)
}
pub fn equals(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.equals(b)
}
pub fn equals_exact(a: &Geometry, b: &Geometry, tolerance: f64) -> Result<bool> {
    a.equals_exact(b, tolerance)
}
pub fn equals_identical(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.equals_identical(b)
}
pub fn intersects(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.intersects(b)
}
pub fn overlaps(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.overlaps(b)
}
pub fn relate(a: &Geometry, b: &Geometry) -> Result<String> {
    a.relate(b)
}
pub fn relate_pattern(a: &Geometry, b: &Geometry, pattern: &str) -> Result<bool> {
    a.relate_pattern(b, pattern)
}
pub fn touches(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.touches(b)
}
pub fn within(a: &Geometry, b: &Geometry) -> Result<bool> {
    a.within(b)
}
