//! Free-function constructors and WKT/WKB/GeoJSON I/O.

use crate::internal_prelude::*;

pub fn point(x: f64, y: f64) -> Result<Geometry> {
    Geometry::point(x, y)
}
pub fn point_z(x: f64, y: f64, z: f64) -> Result<Geometry> {
    Geometry::point_z(x, y, z)
}
pub fn line_string(coords: &[Coord]) -> Result<Geometry> {
    Geometry::line_string(coords)
}
pub fn linearring(coords: &[Coord]) -> Result<Geometry> {
    Geometry::linear_ring(coords)
}
pub fn polygon(shell: &[Coord], holes: &[Vec<Coord>]) -> Result<Geometry> {
    Geometry::polygon(shell, holes)
}
pub fn box_(xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> Result<Geometry> {
    Geometry::rectangle(xmin, ymin, xmax, ymax)
}
pub fn multipoints(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::MultiPoint, geometries)
}
pub fn multilinestrings(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::MultiLineString, geometries)
}
pub fn multipolygons(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::MultiPolygon, geometries)
}
pub fn geometrycollections(geometries: Vec<Geometry>) -> Result<Geometry> {
    Geometry::collection(CollectionType::GeometryCollection, geometries)
}
pub fn from_wkt(wkt: &str) -> Result<Geometry> {
    Geometry::from_wkt(wkt)
}
pub fn from_wkb(wkb: &[u8]) -> Result<Geometry> {
    Geometry::from_wkb(wkb)
}
pub fn from_geojson(geojson: &str) -> Result<Geometry> {
    Geometry::from_geojson(geojson)
}
pub fn to_wkt(geometry: &Geometry) -> Result<String> {
    geometry.to_wkt()
}
pub fn to_wkb(geometry: &Geometry) -> Result<Vec<u8>> {
    geometry.to_wkb()
}
pub fn to_geojson(geometry: &Geometry, indent: Option<i32>) -> Result<String> {
    geometry.to_geojson(indent)
}
