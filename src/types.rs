//! Coordinate, bounds, and geometry-kind value types.

use crate::internal_prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GeometryType {
    Point,
    LineString,
    LinearRing,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
    GeometryCollection,
    CircularString,
    CompoundCurve,
    CurvePolygon,
    MultiCurve,
    MultiSurface,
    Unknown(i32),
}

impl From<i32> for GeometryType {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Point,
            1 => Self::LineString,
            2 => Self::LinearRing,
            3 => Self::Polygon,
            4 => Self::MultiPoint,
            5 => Self::MultiLineString,
            6 => Self::MultiPolygon,
            7 => Self::GeometryCollection,
            8 => Self::CircularString,
            9 => Self::CompoundCurve,
            10 => Self::CurvePolygon,
            11 => Self::MultiCurve,
            12 => Self::MultiSurface,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CoordType {
    XY,
    XYZ,
    XYM,
    XYZM,
}

impl CoordType {
    pub(crate) const fn has_z(self) -> c_int {
        matches!(self, Self::XYZ | Self::XYZM) as c_int
    }

    pub(crate) const fn has_m(self) -> c_int {
        matches!(self, Self::XYM | Self::XYZM) as c_int
    }

    pub(crate) const fn width(self) -> usize {
        match self {
            Self::XY => 2,
            Self::XYZ | Self::XYM => 3,
            Self::XYZM => 4,
        }
    }
}

/// Pass as the `holes` argument of [`Geometry::polygon`] for a solid polygon.
///
/// Works with any coordinate type; the element type is inferred from the shell.
///
/// ```
/// use geoxi::{Geometry, no_holes};
/// # fn main() -> geoxi::Result<()> {
/// let square = Geometry::polygon(
///     &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)],
///     no_holes(),
/// )?;
/// assert_eq!(square.area()?, 1.0);
/// # Ok(())
/// # }
/// ```
#[inline]
#[must_use]
pub const fn no_holes<C: crate::CoordLike>() -> &'static [&'static [C]] {
    &[]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub m: Option<f64>,
}

/// A dense 3D coordinate.
///
/// `#[repr(C)]`, so it is layout-compatible with `[f64; 3]`. This is what makes
/// the interleaved buffer path ([`Geometry::line_string_from_interleaved`])
/// able to hand a `&[Coord3]` straight to GEOS with no conversion.
///
/// [`Geometry::line_string_from_interleaved`]: crate::Geometry::line_string_from_interleaved
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Coord3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Coord3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AffineOrigin {
    Center,
    Centroid,
    Point(Coord3),
}

impl Coord {
    pub const fn xy(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            z: None,
            m: None,
        }
    }

    pub const fn xyz(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z: Some(z),
            m: None,
        }
    }

    pub const fn xym(x: f64, y: f64, m: f64) -> Self {
        Self {
            x,
            y,
            z: None,
            m: Some(m),
        }
    }

    pub const fn xyzm(x: f64, y: f64, z: f64, m: f64) -> Self {
        Self {
            x,
            y,
            z: Some(z),
            m: Some(m),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub xmin: f64,
    pub ymin: f64,
    pub xmax: f64,
    pub ymax: f64,
}
