//! Scalar measurements: area, length, distances, clearance.

use crate::internal_prelude::*;

pub fn area(geometry: &Geometry) -> Result<f64> {
    geometry.area()
}
pub fn distance(a: &Geometry, b: &Geometry) -> Result<f64> {
    a.distance(b)
}
pub fn bounds(geometry: &Geometry) -> Result<Bounds> {
    geometry.bounds()
}
pub fn length(geometry: &Geometry) -> Result<f64> {
    geometry.length()
}
pub fn hausdorff_distance(a: &Geometry, b: &Geometry, densify: Option<f64>) -> Result<f64> {
    densify.map_or_else(
        || a.hausdorff_distance(b),
        |d| a.hausdorff_distance_densify(b, d),
    )
}
pub fn frechet_distance(a: &Geometry, b: &Geometry, densify: Option<f64>) -> Result<f64> {
    densify.map_or_else(
        || a.frechet_distance(b),
        |d| a.frechet_distance_densify(b, d),
    )
}
pub fn minimum_clearance(geometry: &Geometry) -> Result<f64> {
    geometry.minimum_clearance()
}
pub fn minimum_bounding_radius(geometry: &Geometry) -> Result<f64> {
    Ok(geometry.minimum_bounding_circle()?.radius)
}
