//! Ergonomic Rust geometry API backed directly by `geos-sys`.
//!
//! `geoxi` is intentionally a higher-level layer than raw GEOS FFI. It owns GEOS
//! handles, returns typed `Result`s, exposes free functions alongside inherent
//! methods, and keeps lower-level escape hatches available through
//! [`Geometry::as_raw`].
//!
//! GEOS is a planar geometry engine. `geoxi` preserves Z/M coordinates where GEOS
//! does, but GEOS topology, predicates, indexing, buffering, overlays, and
//! measurements are XY operations. The [`three_d`] module supplies genuinely
//! three-dimensional counterparts for the places where that matters.
//!
//! # Module map
//!
//! The crate is organised in two layers. Modules holding *types* own a value
//! and its inherent methods; modules holding *operations* expose free
//! functions over those types.
//!
//! | Module | Contents |
//! | --- | --- |
//! | [`error`] | [`Error`] and the crate-wide [`Result`] alias |
//! | [`types`] | [`Coord`], [`Coord3`], [`Bounds`], [`GeometryType`], [`CoordType`] |
//! | [`context`] | [`Context`], the GEOS thread handle |
//! | [`coord_like`] | [`CoordLike`]: use any coordinate type anywhere |
//! | [`buffers`] | Interleaved `f64` buffers, the zero-copy bulk path |
//! | [`geometry`] | [`Geometry`], the owned handle: lifecycle and I/O |
//! | [`options`] | Parameter structs such as [`BufferOptions`] |
//! | [`prepared`] | [`PreparedGeometry`] for repeated predicate evaluation |
//! | [`array`](mod@crate::array) | [`GeometryArray`] batch operations |
//! | [`strtree`] | [`STRtree`] spatial index |
//! | [`constructors`] | Constructors and WKT/WKB/GeoJSON I/O |
//! | [`measure`] | Area, length, and distance measurements |
//! | [`predicates`] | Binary spatial predicates |
//! | [`affinity`] | Affine transforms and coordinate rewriting |
//! | [`ops`] | Unary ops, set ops, linear referencing, coverages |
//! | [`topology`] | Splitting, orientation, validation, equality |
//! | [`accessors`] | Scalar accessors and batch constructors |
//! | [`coords`] | Coordinate extraction and GeoArrow ragged arrays |
//! | [`crs`] | Reprojection against any coordinate transform engine |
//! | [`three_d`] | Native 3D algorithms GEOS does not provide |
//!
//! [`Geometry`]'s own method surface is large, so it is split across several
//! private files (`geometry_access`, `geometry_predicates`, `geometry_measure`,
//! `geometry_ops`, `geometry_linear`) that each contribute one `impl Geometry`
//! block. This is purely an organisational split; all methods are on the same
//! type and documented together.
//!
//! Every public item is re-exported at the crate root, so `use geoxi::*` or
//! `use geoxi::Geometry` both work regardless of which module an item lives in.
//! [`prelude`] carries the everyday subset.
//!
//! # Linking GEOS
//!
//! By default `geoxi` builds GEOS from pinned, checksum-verified source and
//! links it statically. Nothing needs to be installed on the machine beyond a
//! C++ compiler and CMake, which are needed at *build* time only. Building
//! with `-C target-feature=+crt-static` additionally links the C++ runtime
//! statically, producing an executable that depends only on OS libraries and
//! can be shipped to any machine of the same target.
//!
//! Enable the `system` feature to link an
//! already-installed GEOS instead, located via `GEOS_DIR`, vcpkg, or
//! pkg-config. That binary is then no longer self-contained.

pub mod accessors;
pub mod affinity;
pub mod array;
pub mod buffers;
pub mod constructors;
pub mod context;
pub mod coord_like;
pub mod coords;
pub mod crs;
pub mod error;
#[cfg(feature = "geo-types")]
pub mod geo_types;
pub mod geometry;
mod geometry_access;
mod geometry_linear;
mod geometry_measure;
mod geometry_ops;
mod geometry_predicates;
pub mod measure;
pub mod ops;
pub mod options;
pub mod predicates;
pub mod prepared;
pub mod strtree;
pub mod three_d;
pub mod topology;
pub mod types;

mod ffi;

pub use accessors::*;
pub use affinity::*;
pub use array::*;
pub use buffers::Dimensions;
pub use constructors::*;
pub use context::*;
pub use coord_like::CoordLike;
pub use error::*;
pub use geometry::*;
pub use measure::*;
pub use ops::*;
pub use options::*;
pub use predicates::*;
pub use prepared::*;
pub use strtree::*;
pub use topology::*;
pub use types::*;

pub use coords::{
    RaggedArray, RaggedType, count_coordinates, from_ragged_array, get_coordinates,
    get_coordinates_all, get_segments, set_coordinates, to_ragged_array, transform_coords,
};
pub use crs::{CoordTransform, TransformError, reproject, reproject_all, reproject_bounds};
pub use three_d::{
    Bounds3, ConvexHull3, Degeneracy, Sphere, Triangle, Vec3, bounding_sphere, bounds_3d,
    convex_hull_3d, convex_hull_3d_with_tolerance, geometry_convex_hull_3d, length_3d,
    newell_normal, planar_area_3d,
};

/// Common imports for day-to-day use: `use geoxi::prelude::*;`
pub mod prelude {
    pub use crate::array::GeometryArray;
    pub use crate::buffers::Dimensions;
    pub use crate::context::Context;
    pub use crate::coord_like::CoordLike;
    pub use crate::coords::{get_coordinates, set_coordinates, to_ragged_array};
    pub use crate::crs::{CoordTransform, reproject};
    pub use crate::error::{Error, Result};
    pub use crate::geometry::Geometry;
    pub use crate::no_holes;
    pub use crate::options::{BufferOptions, CapStyle, JoinStyle, PrecisionMode};
    pub use crate::prepared::PreparedGeometry;
    pub use crate::strtree::{BinaryPredicate, STRtree};
    pub use crate::three_d::{Bounds3, ConvexHull3, Vec3, convex_hull_3d};
    pub use crate::types::{Bounds, Coord, Coord3, CoordType, GeometryType};
}

/// Everything the internal modules need in scope. Not part of the public API.
pub(crate) mod internal_prelude {
    #[allow(unused_imports)]
    pub(crate) use crate::accessors::*;
    #[allow(unused_imports)]
    pub(crate) use crate::affinity::*;
    #[allow(unused_imports)]
    pub(crate) use crate::array::*;
    #[allow(unused_imports)]
    pub(crate) use crate::constructors::*;
    #[allow(unused_imports)]
    pub(crate) use crate::context::*;
    #[allow(unused_imports)]
    pub(crate) use crate::coord_like::CoordLike;
    #[allow(unused_imports)]
    pub(crate) use crate::error::*;
    #[allow(unused_imports)]
    pub(crate) use crate::ffi::*;
    #[allow(unused_imports)]
    pub(crate) use crate::geometry::*;
    #[allow(unused_imports)]
    pub(crate) use crate::measure::*;
    #[allow(unused_imports)]
    pub(crate) use crate::ops::*;
    #[allow(unused_imports)]
    pub(crate) use crate::options::*;
    #[allow(unused_imports)]
    pub(crate) use crate::predicates::*;
    #[allow(unused_imports)]
    pub(crate) use crate::prepared::*;
    #[allow(unused_imports)]
    pub(crate) use crate::strtree::*;
    #[allow(unused_imports)]
    pub(crate) use crate::topology::*;
    #[allow(unused_imports)]
    pub(crate) use crate::types::*;

    #[allow(unused_imports)]
    pub(crate) use geos_sys::*;
    #[allow(unused_imports)]
    pub(crate) use libc::{c_char, c_double, c_int, c_uchar, c_uint, c_void};
    #[allow(unused_imports)]
    pub(crate) use std::f64::consts::PI;
    #[allow(unused_imports)]
    pub(crate) use std::ffi::{CStr, CString, NulError};
    #[allow(unused_imports)]
    pub(crate) use std::fmt;
    #[allow(unused_imports)]
    pub(crate) use std::ptr::{self, NonNull};
    #[allow(unused_imports)]
    pub(crate) use std::sync::Arc;
}

#[cfg(test)]
mod tests;
