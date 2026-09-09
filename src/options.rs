//! Option and result structs for parameterised operations.

use crate::internal_prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum CollectionType {
    MultiPoint = 4,
    MultiLineString = 5,
    MultiPolygon = 6,
    GeometryCollection = 7,
}

#[derive(Debug, Clone, Copy)]
pub enum CapStyle {
    Round = GEOSBufCapStyles_GEOSBUF_CAP_ROUND as isize,
    Flat = GEOSBufCapStyles_GEOSBUF_CAP_FLAT as isize,
    Square = GEOSBufCapStyles_GEOSBUF_CAP_SQUARE as isize,
}

#[derive(Debug, Clone, Copy)]
pub enum JoinStyle {
    Round = GEOSBufJoinStyles_GEOSBUF_JOIN_ROUND as isize,
    Mitre = GEOSBufJoinStyles_GEOSBUF_JOIN_MITRE as isize,
    Bevel = GEOSBufJoinStyles_GEOSBUF_JOIN_BEVEL as isize,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferOptions {
    pub distance: f64,
    pub quad_segs: i32,
    pub cap_style: CapStyle,
    pub join_style: JoinStyle,
    pub mitre_limit: f64,
    pub single_sided: bool,
}

impl BufferOptions {
    pub const fn new(distance: f64) -> Self {
        Self {
            distance,
            quad_segs: 8,
            cap_style: CapStyle::Round,
            join_style: JoinStyle::Round,
            mitre_limit: 5.0,
            single_sided: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PrecisionMode {
    ValidOutput = GEOSPrecisionRules_GEOS_PREC_VALID_OUTPUT as isize,
    NoTopo = GEOSPrecisionRules_GEOS_PREC_NO_TOPO as isize,
    KeepCollapsed = GEOSPrecisionRules_GEOS_PREC_KEEP_COLLAPSED as isize,
}

#[derive(Debug)]
pub struct MinimumBoundingCircle {
    pub circle: Geometry,
    pub center: Geometry,
    pub radius: f64,
}
