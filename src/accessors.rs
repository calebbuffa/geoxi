//! Scalar accessors and batch constructors.

use crate::internal_prelude::*;

pub fn get_geometry(geometry: &Geometry, index: usize) -> Result<Geometry> {
    geometry.geometry_n(index)
}
pub fn get_num_geometries(geometry: &Geometry) -> Result<usize> {
    geometry.num_geometries()
}
pub fn get_num_coordinates(geometry: &Geometry) -> Result<usize> {
    geometry.num_coordinates()
}
pub fn get_num_points(geometry: &Geometry) -> Result<usize> {
    geometry.num_points()
}
pub fn get_num_interior_rings(geometry: &Geometry) -> Result<usize> {
    geometry.num_interior_rings()
}
pub fn get_exterior_ring(geometry: &Geometry) -> Result<Geometry> {
    geometry.exterior_ring()
}
pub fn get_interior_ring(geometry: &Geometry, index: usize) -> Result<Geometry> {
    geometry.interior_ring_n(index)
}
pub fn get_point(geometry: &Geometry, index: usize) -> Result<Geometry> {
    geometry.point_n(index)
}
pub fn get_x(geometry: &Geometry) -> Result<f64> {
    geometry.x()
}
pub fn get_y(geometry: &Geometry) -> Result<f64> {
    geometry.y()
}
pub fn get_z(geometry: &Geometry) -> Result<f64> {
    geometry.z()
}
pub fn get_m(geometry: &Geometry) -> Result<f64> {
    geometry.m()
}
pub fn get_srid(geometry: &Geometry) -> i32 {
    geometry.srid()
}
pub fn set_srid(geometry: &Geometry, srid: i32) -> Result<Geometry> {
    let mut out = geometry.try_clone()?;
    out.set_srid(srid);
    Ok(out)
}
pub fn get_precision(geometry: &Geometry) -> f64 {
    geometry.precision()
}
pub fn set_precision(geometry: &Geometry, grid_size: f64, mode: PrecisionMode) -> Result<Geometry> {
    geometry.set_precision(grid_size, mode)
}
pub fn get_type_id(geometry: &Geometry) -> Result<GeometryType> {
    geometry.geometry_type()
}
pub fn get_dimensions(geometry: &Geometry) -> Result<i32> {
    geometry.dimensions()
}
pub fn get_coordinate_dimension(geometry: &Geometry) -> Result<i32> {
    geometry.coordinate_dimension()
}
pub fn prepare(geometry: &Geometry) -> Result<PreparedGeometry> {
    geometry.prepared()
}
pub fn intersects_xy(geometry: &Geometry, x: f64, y: f64) -> Result<bool> {
    let point = Geometry::point(x, y)?;
    geometry.intersects(&point)
}
pub fn contains_xy(geometry: &Geometry, x: f64, y: f64) -> Result<bool> {
    let point = Geometry::point(x, y)?;
    geometry.contains(&point)
}
pub fn is_ccw_geometry(geometry: &Geometry) -> Result<bool> {
    is_ccw(geometry)
}

/// Batch constructors that build one geometry per input element.
pub fn points(coords: &[Coord]) -> Result<Vec<Geometry>> {
    coords.iter().map(|c| Geometry::point_from(*c)).collect()
}
pub fn linestrings(rings: &[Vec<Coord>]) -> Result<Vec<Geometry>> {
    rings
        .iter()
        .map(|ring| Geometry::line_string(ring))
        .collect()
}
pub fn linearrings(rings: &[Vec<Coord>]) -> Result<Vec<Geometry>> {
    rings
        .iter()
        .map(|ring| Geometry::linear_ring(ring))
        .collect()
}
pub fn polygons(shells: &[Vec<Coord>]) -> Result<Vec<Geometry>> {
    shells
        .iter()
        .map(|shell| Geometry::polygon(shell, no_holes()))
        .collect()
}

/// An empty geometry of the requested type.
pub fn empty(geometry_type: GeometryType) -> Result<Geometry> {
    let ctx = Context::current()?;
    let ptr = match geometry_type {
        GeometryType::Point => unsafe { GEOSGeom_createEmptyPoint_r(ctx.raw()) },
        GeometryType::LineString | GeometryType::LinearRing => unsafe {
            GEOSGeom_createEmptyLineString_r(ctx.raw())
        },
        GeometryType::Polygon => unsafe { GEOSGeom_createEmptyPolygon_r(ctx.raw()) },
        GeometryType::MultiPoint => unsafe {
            GEOSGeom_createEmptyCollection_r(ctx.raw(), CollectionType::MultiPoint as c_int)
        },
        GeometryType::MultiLineString => unsafe {
            GEOSGeom_createEmptyCollection_r(ctx.raw(), CollectionType::MultiLineString as c_int)
        },
        GeometryType::MultiPolygon => unsafe {
            GEOSGeom_createEmptyCollection_r(ctx.raw(), CollectionType::MultiPolygon as c_int)
        },
        GeometryType::GeometryCollection => unsafe {
            GEOSGeom_createEmptyCollection_r(ctx.raw(), CollectionType::GeometryCollection as c_int)
        },
        _ => {
            return Err(Error::InvalidInput(
                "empty geometries are unavailable for curved types",
            ));
        }
    };
    unsafe { Geometry::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating empty geometry"))
}

/// Alias for [`box_`].
pub fn bbox(xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> Result<Geometry> {
    box_(xmin, ymin, xmax, ymax)
}

/// Alias for [`line_merge`].
pub fn linemerge(line: &Geometry, directed: bool) -> Result<Geometry> {
    line_merge(line, directed)
}

/// Alias for [`delaunay_triangles`].
pub fn triangulate(geometry: &Geometry, tolerance: f64, only_edges: bool) -> Result<Geometry> {
    delaunay_triangles(geometry, tolerance, only_edges)
}

/// Alias for [`voronoi_polygons`].
pub fn voronoi_diagram(
    geometry: &Geometry,
    envelope: Option<&Geometry>,
    tolerance: f64,
    only_edges: bool,
) -> Result<Geometry> {
    voronoi_polygons(geometry, envelope, tolerance, only_edges)
}

/// Alias for [`crate::coords::transform_coords`].
pub fn transform_coordseq<F>(geometry: &Geometry, f: F) -> Result<Geometry>
where
    F: FnMut(Coord) -> Result<Coord>,
{
    crate::coords::transform_coords(geometry, f)
}

/// Symmetric difference of every geometry, folded pairwise.
pub fn symmetric_difference_all(geometries: Vec<Geometry>) -> Result<Option<Geometry>> {
    let mut iter = geometries.into_iter();
    let Some(mut accumulated) = iter.next() else {
        return Ok(None);
    };
    for geometry in iter {
        accumulated = accumulated.symmetric_difference(&geometry)?;
    }
    Ok(Some(accumulated))
}

/// Whether a value is a usable geometry. Rust's type system already guarantees
/// this, so this always succeeds for a live [`Geometry`].
pub fn is_geometry(geometry: &Geometry) -> Result<bool> {
    geometry.geometry_type().map(|_| true)
}

/// Whether an optional geometry is absent.
pub fn is_missing(geometry: Option<&Geometry>) -> bool {
    geometry.is_none()
}

/// Whether a geometry has an attached prepared index. `geoxi` models preparation
/// as a distinct [`PreparedGeometry`] value, so this is always `false` for a
/// plain [`Geometry`].
pub fn is_prepared(_geometry: &Geometry) -> bool {
    false
}

/// Long-form alias for [`CapStyle`].
pub type BufferCapStyle = CapStyle;
/// Long-form alias for [`JoinStyle`].
pub type BufferJoinStyle = JoinStyle;
