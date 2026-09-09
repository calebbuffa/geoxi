//! The [`CoordLike`] trait: use your own coordinate types directly.
//!
//! Every `geoxi` constructor that accepts coordinates is generic over
//! `CoordLike`, so there is no conversion step and no adapter function to call.
//! Pass a slice of whatever you already have:
//!
//! ```
//! use geoxi::Geometry;
//!
//! # fn main() -> geoxi::Result<()> {
//! // Tuples and arrays work out of the box.
//! let a = Geometry::line_string(&[(0.0, 0.0), (1.0, 1.0)])?;
//! let b = Geometry::line_string(&[[0.0, 0.0, 5.0], [1.0, 1.0, 6.0]])?;
//!
//! // With the `glam` feature, so do `DVec2` / `DVec3`; with `nalgebra`,
//! // `Point2` / `Point3` / `Vector2` / `Vector3`.
//! # let _ = (a, b);
//! # Ok(())
//! # }
//! ```
//!
//! Implement it for your own type and it becomes first-class too:
//!
//! ```
//! use geoxi::{CoordLike, Geometry};
//!
//! #[derive(Clone, Copy)]
//! struct Station { lon: f64, lat: f64, elevation: f64 }
//!
//! impl CoordLike for Station {
//!     fn x(&self) -> f64 { self.lon }
//!     fn y(&self) -> f64 { self.lat }
//!     fn z(&self) -> Option<f64> { Some(self.elevation) }
//!     fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
//!         Station { lon: x, lat: y, elevation: z.unwrap_or(0.0) }
//!     }
//! }
//!
//! # fn main() -> geoxi::Result<()> {
//! let track = Geometry::line_string(&[
//!     Station { lon: 0.0, lat: 0.0, elevation: 100.0 },
//!     Station { lon: 1.0, lat: 1.0, elevation: 120.0 },
//! ])?;
//! assert!(track.has_z()?);
//!
//! // Read them back out as the same type.
//! let stations: Vec<Station> = track.coords()?;
//! assert_eq!(stations.len(), 2);
//! # Ok(())
//! # }
//! ```
//!
//! # On "zero-copy"
//!
//! This trait is **zero-allocation**, not zero-copy: components are read
//! through `x()`/`y()`/`z()`/`m()` and written back through [`from_xyzm`], with
//! no intermediate `Vec` and no boxing. The accessors are `#[inline]` and
//! compile away.
//!
//! Genuine zero-copy is a different mechanism and lives in [`crate::buffers`],
//! which hands a packed `f64` buffer straight to GEOS's
//! `copyFromBuffer`/`copyToBuffer` entry points without touching the values at
//! all.
//!
//! [`from_xyzm`]: CoordLike::from_xyzm

use crate::types::{Coord, Coord3};

/// A type that can be read as, and rebuilt from, a coordinate.
///
/// `x` and `y` are required; `z` and `m` are optional and default to absent.
/// Returning `Some(z)` from a coordinate makes the resulting geometry 3D.
pub trait CoordLike: Copy {
    /// The x (easting / longitude) component.
    fn x(&self) -> f64;

    /// The y (northing / latitude) component.
    fn y(&self) -> f64;

    /// The z (height) component, if this type carries one.
    #[inline]
    fn z(&self) -> Option<f64> {
        None
    }

    /// The m (measure) component, if this type carries one.
    #[inline]
    fn m(&self) -> Option<f64> {
        None
    }

    /// Rebuild a coordinate from its components.
    ///
    /// Components the type cannot represent are dropped.
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, m: Option<f64>) -> Self;
}

impl CoordLike for Coord {
    #[inline]
    fn x(&self) -> f64 {
        self.x
    }
    #[inline]
    fn y(&self) -> f64 {
        self.y
    }
    #[inline]
    fn z(&self) -> Option<f64> {
        self.z
    }
    #[inline]
    fn m(&self) -> Option<f64> {
        self.m
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, m: Option<f64>) -> Self {
        Coord { x, y, z, m }
    }
}

impl CoordLike for Coord3 {
    #[inline]
    fn x(&self) -> f64 {
        self.x
    }
    #[inline]
    fn y(&self) -> f64 {
        self.y
    }
    #[inline]
    fn z(&self) -> Option<f64> {
        Some(self.z)
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
        Coord3::new(x, y, z.unwrap_or(0.0))
    }
}

impl CoordLike for crate::three_d::Vec3 {
    #[inline]
    fn x(&self) -> f64 {
        self.x
    }
    #[inline]
    fn y(&self) -> f64 {
        self.y
    }
    #[inline]
    fn z(&self) -> Option<f64> {
        Some(self.z)
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
        crate::three_d::Vec3::new(x, y, z.unwrap_or(0.0))
    }
}

impl CoordLike for (f64, f64) {
    #[inline]
    fn x(&self) -> f64 {
        self.0
    }
    #[inline]
    fn y(&self) -> f64 {
        self.1
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
        (x, y)
    }
}

impl CoordLike for (f64, f64, f64) {
    #[inline]
    fn x(&self) -> f64 {
        self.0
    }
    #[inline]
    fn y(&self) -> f64 {
        self.1
    }
    #[inline]
    fn z(&self) -> Option<f64> {
        Some(self.2)
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
        (x, y, z.unwrap_or(0.0))
    }
}

impl CoordLike for [f64; 2] {
    #[inline]
    fn x(&self) -> f64 {
        self[0]
    }
    #[inline]
    fn y(&self) -> f64 {
        self[1]
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
        [x, y]
    }
}

impl CoordLike for [f64; 3] {
    #[inline]
    fn x(&self) -> f64 {
        self[0]
    }
    #[inline]
    fn y(&self) -> f64 {
        self[1]
    }
    #[inline]
    fn z(&self) -> Option<f64> {
        Some(self[2])
    }
    #[inline]
    fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
        [x, y, z.unwrap_or(0.0)]
    }
}

#[cfg(feature = "glam")]
mod glam_impls {
    use super::CoordLike;
    use glam::{DVec2, DVec3};

    impl CoordLike for DVec2 {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
            DVec2::new(x, y)
        }
    }

    impl CoordLike for DVec3 {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn z(&self) -> Option<f64> {
            Some(self.z)
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
            DVec3::new(x, y, z.unwrap_or(0.0))
        }
    }
}

#[cfg(feature = "nalgebra")]
mod nalgebra_impls {
    use super::CoordLike;
    use nalgebra::{Point2, Point3, Vector2, Vector3};

    impl CoordLike for Point2<f64> {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
            Point2::new(x, y)
        }
    }

    impl CoordLike for Point3<f64> {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn z(&self) -> Option<f64> {
            Some(self.z)
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
            Point3::new(x, y, z.unwrap_or(0.0))
        }
    }

    impl CoordLike for Vector2<f64> {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
            Vector2::new(x, y)
        }
    }

    impl CoordLike for Vector3<f64> {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn z(&self) -> Option<f64> {
            Some(self.z)
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, z: Option<f64>, _m: Option<f64>) -> Self {
            Vector3::new(x, y, z.unwrap_or(0.0))
        }
    }
}

#[cfg(feature = "geo-types")]
mod geo_types_impls {
    use super::CoordLike;
    use geo_types::{Coord as GeoCoord, Point};

    impl CoordLike for GeoCoord<f64> {
        #[inline]
        fn x(&self) -> f64 {
            self.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.y
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
            GeoCoord { x, y }
        }
    }

    impl CoordLike for Point<f64> {
        #[inline]
        fn x(&self) -> f64 {
            self.0.x
        }
        #[inline]
        fn y(&self) -> f64 {
            self.0.y
        }
        #[inline]
        fn from_xyzm(x: f64, y: f64, _z: Option<f64>, _m: Option<f64>) -> Self {
            Point::new(x, y)
        }
    }
}
