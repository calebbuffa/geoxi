//! Whole-geometry conversion to and from [`geo_types`].
//!
//! Enabled by the `geo-types` feature. Point-level conversion is handled by the
//! [`CoordLike`] impls for `geo_types::Coord` and `geo_types::Point`, which let
//! you pass those types straight into any constructor. This module covers the
//! structural cases - polygons with holes, collections, and so on - where a
//! coordinate-by-coordinate trait cannot express the shape.
//!
//! These conversions are **not** zero-copy and cannot be: a GEOS geometry is an
//! opaque C++ object behind a pointer, with no Rust-visible layout. Each
//! direction walks the structure and rebuilds it. Prefer staying in one
//! representation when performance matters.
//!
//! ```
//! # #[cfg(feature = "geo-types")] {
//! use geo_types::{line_string, Geometry as GeoGeometry};
//! use geoxi::Geometry;
//!
//! # fn main() -> geoxi::Result<()> {
//! let source: GeoGeometry<f64> = line_string![(x: 0.0, y: 0.0), (x: 3.0, y: 4.0)].into();
//! let geom = Geometry::from_geo(&source)?;
//! assert_eq!(geom.length()?, 5.0);
//!
//! let back = geom.to_geo()?;
//! assert_eq!(back, source);
//! # Ok(())
//! # }
//! # }
//! ```

use crate::internal_prelude::*;
use geo_types::{
    Coord as GeoCoord, Geometry as GeoGeometry, GeometryCollection, LineString, MultiLineString,
    MultiPoint, MultiPolygon, Point, Polygon,
};

fn unsupported(what: &'static str) -> Error {
    Error::InvalidInput(what)
}

impl Geometry {
    /// Convert a [`geo_types`] geometry into a geoxi geometry.
    ///
    /// Z and M are dropped: `geo-types` is strictly 2D. Build from a 3D
    /// [`CoordLike`] type instead if you need to preserve elevation.
    pub fn from_geo(geometry: &GeoGeometry<f64>) -> Result<Self> {
        match geometry {
            GeoGeometry::Point(p) => Geometry::point_from(*p),
            GeoGeometry::Line(l) => Geometry::line_string(&[l.start, l.end]),
            GeoGeometry::LineString(ls) => Geometry::line_string(&ls.0),
            GeoGeometry::Polygon(p) => polygon_from_geo(p),
            GeoGeometry::MultiPoint(mp) => {
                let parts =
                    mp.0.iter()
                        .map(|p| Geometry::point_from(*p))
                        .collect::<Result<Vec<_>>>()?;
                Geometry::collection(CollectionType::MultiPoint, parts)
            }
            GeoGeometry::MultiLineString(mls) => {
                let parts = mls
                    .0
                    .iter()
                    .map(|ls| Geometry::line_string(&ls.0))
                    .collect::<Result<Vec<_>>>()?;
                Geometry::collection(CollectionType::MultiLineString, parts)
            }
            GeoGeometry::MultiPolygon(mp) => {
                let parts =
                    mp.0.iter()
                        .map(polygon_from_geo)
                        .collect::<Result<Vec<_>>>()?;
                Geometry::collection(CollectionType::MultiPolygon, parts)
            }
            GeoGeometry::GeometryCollection(gc) => {
                let parts =
                    gc.0.iter()
                        .map(Geometry::from_geo)
                        .collect::<Result<Vec<_>>>()?;
                Geometry::collection(CollectionType::GeometryCollection, parts)
            }
            GeoGeometry::Rect(r) => polygon_from_geo(&r.to_polygon()),
            GeoGeometry::Triangle(t) => polygon_from_geo(&t.to_polygon()),
        }
    }

    /// Convert this geometry into a [`geo_types`] geometry.
    ///
    /// Z and M are dropped. Returns [`Error::InvalidInput`] for curved types
    /// (CircularString, CompoundCurve, CurvePolygon, MultiCurve, MultiSurface),
    /// which `geo-types` cannot represent.
    pub fn to_geo(&self) -> Result<GeoGeometry<f64>> {
        Ok(match self.geometry_type()? {
            GeometryType::Point => {
                if self.is_empty()? {
                    return Err(unsupported("geo-types cannot represent an empty Point"));
                }
                GeoGeometry::Point(self.coords::<Point<f64>>()?[0])
            }
            GeometryType::LineString => GeoGeometry::LineString(self.to_geo_line_string()?),
            GeometryType::LinearRing => GeoGeometry::LineString(self.to_geo_line_string()?),
            GeometryType::Polygon => GeoGeometry::Polygon(self.to_geo_polygon()?),
            GeometryType::MultiPoint => {
                let mut points = Vec::with_capacity(self.num_geometries()?);
                for part in self.parts()? {
                    if !part.is_empty()? {
                        points.push(part.coords::<Point<f64>>()?[0]);
                    }
                }
                GeoGeometry::MultiPoint(MultiPoint(points))
            }
            GeometryType::MultiLineString => {
                let mut lines = Vec::with_capacity(self.num_geometries()?);
                for part in self.parts()? {
                    lines.push(part.to_geo_line_string()?);
                }
                GeoGeometry::MultiLineString(MultiLineString(lines))
            }
            GeometryType::MultiPolygon => {
                let mut polygons = Vec::with_capacity(self.num_geometries()?);
                for part in self.parts()? {
                    polygons.push(part.to_geo_polygon()?);
                }
                GeoGeometry::MultiPolygon(MultiPolygon(polygons))
            }
            GeometryType::GeometryCollection => {
                let mut parts = Vec::with_capacity(self.num_geometries()?);
                for part in self.parts()? {
                    parts.push(part.to_geo()?);
                }
                GeoGeometry::GeometryCollection(GeometryCollection(parts))
            }
            other => {
                return Err(unsupported(match other {
                    GeometryType::CircularString => "geo-types cannot represent a CircularString",
                    GeometryType::CompoundCurve => "geo-types cannot represent a CompoundCurve",
                    GeometryType::CurvePolygon => "geo-types cannot represent a CurvePolygon",
                    GeometryType::MultiCurve => "geo-types cannot represent a MultiCurve",
                    GeometryType::MultiSurface => "geo-types cannot represent a MultiSurface",
                    _ => "geometry type is not representable in geo-types",
                }));
            }
        })
    }

    fn to_geo_line_string(&self) -> Result<LineString<f64>> {
        Ok(LineString(self.coords::<GeoCoord<f64>>()?))
    }

    fn to_geo_polygon(&self) -> Result<Polygon<f64>> {
        if self.is_empty()? {
            return Ok(Polygon::new(LineString(Vec::new()), Vec::new()));
        }
        let shell = self.exterior_ring()?.to_geo_line_string()?;
        let mut holes = Vec::with_capacity(self.num_interior_rings()?);
        for i in 0..self.num_interior_rings()? {
            holes.push(self.interior_ring_n(i)?.to_geo_line_string()?);
        }
        Ok(Polygon::new(shell, holes))
    }
}

fn polygon_from_geo(polygon: &Polygon<f64>) -> Result<Geometry> {
    let holes: Vec<&[GeoCoord<f64>]> = polygon.interiors().iter().map(|r| &r.0[..]).collect();
    Geometry::polygon(&polygon.exterior().0, &holes)
}

impl TryFrom<&GeoGeometry<f64>> for Geometry {
    type Error = Error;
    fn try_from(value: &GeoGeometry<f64>) -> Result<Self> {
        Geometry::from_geo(value)
    }
}

impl TryFrom<&Geometry> for GeoGeometry<f64> {
    type Error = Error;
    fn try_from(value: &Geometry) -> Result<Self> {
        value.to_geo()
    }
}
