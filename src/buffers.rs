//! Interleaved coordinate buffers: the bulk-throughput path.
//!
//! [`CoordLike`] is zero-allocation but still walks coordinates one at a time.
//! When your data already lives in a packed `[x, y, x, y, …]` or
//! `[x, y, z, x, y, z, …]` buffer - which is how most file formats, GPU vertex
//! buffers, and columnar array formats store points - this module hands the whole
//! buffer to GEOS in a single call.
//!
//! `Coord3` and `Vec3` are `#[repr(C)]` and therefore layout-identical to
//! `[f64; 3]`, so a `&[Coord3]` can be reinterpreted as an interleaved buffer
//! with no copy on the Rust side.
//!
//! ```
//! use geoxi::{Dimensions, Geometry};
//!
//! # fn main() -> geoxi::Result<()> {
//! let buffer = [0.0, 0.0, 0.0, 3.0, 4.0, 12.0];
//! let line = Geometry::line_string_from_interleaved(&buffer, Dimensions::Xyz)?;
//! assert_eq!(line.length()?, 5.0);
//!
//! let mut out = vec![0.0; 6];
//! line.write_interleaved(&mut out, Dimensions::Xyz)?;
//! assert_eq!(out, buffer);
//! # Ok(())
//! # }
//! ```

use crate::internal_prelude::*;
use geos_sys::{
    GEOSCoordSeq_copyFromBuffer_r, GEOSCoordSeq_copyToBuffer_r, GEOSCoordSeq_destroy_r,
    GEOSCoordSequence, GEOSGeom_createLineString_r, GEOSGeom_createLinearRing_r,
    GEOSGeom_getCoordSeq_r,
};
use std::ptr::NonNull;

/// How many components each coordinate occupies in an interleaved buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimensions {
    /// `[x, y, x, y, …]`
    Xy,
    /// `[x, y, z, x, y, z, …]`
    Xyz,
    /// `[x, y, m, x, y, m, …]`
    Xym,
    /// `[x, y, z, m, x, y, z, m, …]`
    Xyzm,
}

impl Dimensions {
    /// Components per coordinate: 2, 3, or 4.
    #[inline]
    #[must_use]
    pub const fn stride(self) -> usize {
        match self {
            Dimensions::Xy => 2,
            Dimensions::Xyz | Dimensions::Xym => 3,
            Dimensions::Xyzm => 4,
        }
    }

    #[inline]
    const fn has_z(self) -> c_int {
        matches!(self, Dimensions::Xyz | Dimensions::Xyzm) as c_int
    }

    #[inline]
    const fn has_m(self) -> c_int {
        matches!(self, Dimensions::Xym | Dimensions::Xyzm) as c_int
    }
}

fn sequence_from_buffer(
    ctx: &Context,
    buffer: &[f64],
    dims: Dimensions,
) -> Result<(NonNull<GEOSCoordSequence>, usize)> {
    let stride = dims.stride();
    if !buffer.len().is_multiple_of(stride) {
        return Err(Error::InvalidInput(
            "interleaved buffer length is not a multiple of the coordinate stride",
        ));
    }
    let count = buffer.len() / stride;
    let seq = NonNull::new(unsafe {
        GEOSCoordSeq_copyFromBuffer_r(
            ctx.raw(),
            buffer.as_ptr(),
            count as c_uint,
            dims.has_z(),
            dims.has_m(),
        )
    })
    .ok_or(Error::Null("copying interleaved buffer into GEOS"))?;
    Ok((seq, count))
}

impl Geometry {
    /// Build a LineString from a packed interleaved buffer.
    ///
    /// One FFI call for the whole buffer, regardless of length.
    pub fn line_string_from_interleaved(buffer: &[f64], dims: Dimensions) -> Result<Self> {
        let ctx = Context::current()?;
        let (seq, count) = sequence_from_buffer(&ctx, buffer, dims)?;
        if count < 2 {
            unsafe { GEOSCoordSeq_destroy_r(ctx.raw(), seq.as_ptr()) };
            return Err(Error::InvalidInput(
                "a LineString requires at least two coordinates",
            ));
        }
        let ptr = unsafe { GEOSGeom_createLineString_r(ctx.raw(), seq.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating LineString"))
    }

    /// Build a LinearRing from a packed interleaved buffer.
    ///
    /// The buffer must be closed: its last coordinate must equal its first.
    pub fn linear_ring_from_interleaved(buffer: &[f64], dims: Dimensions) -> Result<Self> {
        let ctx = Context::current()?;
        let (seq, count) = sequence_from_buffer(&ctx, buffer, dims)?;
        if count < 4 {
            unsafe { GEOSCoordSeq_destroy_r(ctx.raw(), seq.as_ptr()) };
            return Err(Error::InvalidInput(
                "a LinearRing requires at least four coordinates",
            ));
        }
        let ptr = unsafe { GEOSGeom_createLinearRing_r(ctx.raw(), seq.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating LinearRing"))
    }

    /// Number of `f64`s [`write_interleaved`] needs for this geometry.
    ///
    /// [`write_interleaved`]: Self::write_interleaved
    pub fn interleaved_len(&self, dims: Dimensions) -> Result<usize> {
        Ok(self.num_coordinates()? * dims.stride())
    }

    /// Write this geometry's coordinates into a caller-owned buffer.
    ///
    /// Bring your own allocation: nothing is allocated here. `buffer` must be at
    /// least [`interleaved_len`] long. Only valid for geometries with a single
    /// coordinate sequence (Point, LineString, LinearRing).
    ///
    /// [`interleaved_len`]: Self::interleaved_len
    pub fn write_interleaved(&self, buffer: &mut [f64], dims: Dimensions) -> Result<usize> {
        let needed = self.interleaved_len(dims)?;
        if buffer.len() < needed {
            return Err(Error::InvalidInput(
                "interleaved buffer is too small for this geometry",
            ));
        }
        let seq = NonNull::new(unsafe {
            GEOSGeom_getCoordSeq_r(self.ctx.raw(), self.ptr.as_ptr()) as *mut GEOSCoordSequence
        })
        .ok_or(Error::Null("reading coordinate sequence"))?;
        check_code(
            unsafe {
                GEOSCoordSeq_copyToBuffer_r(
                    self.ctx.raw(),
                    seq.as_ptr(),
                    buffer.as_mut_ptr(),
                    dims.has_z(),
                    dims.has_m(),
                )
            },
            "copying coordinates into interleaved buffer",
        )?;
        Ok(needed)
    }

    /// Write this geometry's coordinates into a freshly allocated buffer.
    pub fn to_interleaved(&self, dims: Dimensions) -> Result<Vec<f64>> {
        let mut buffer = vec![0.0; self.interleaved_len(dims)?];
        self.write_interleaved(&mut buffer, dims)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xyz_buffer_round_trips_exactly() {
        let buffer = [0.0, 0.0, 1.0, 3.0, 4.0, 2.0, 6.0, 8.0, 3.0];
        let line = Geometry::line_string_from_interleaved(&buffer, Dimensions::Xyz).unwrap();

        assert!(line.has_z().unwrap());
        assert_eq!(line.length().unwrap(), 10.0);
        assert_eq!(line.to_interleaved(Dimensions::Xyz).unwrap(), buffer);
    }

    #[test]
    fn xy_buffer_drops_z_and_matches_the_coordlike_path() {
        let buffer = [0.0, 0.0, 3.0, 4.0];
        let fast = Geometry::line_string_from_interleaved(&buffer, Dimensions::Xy).unwrap();
        let slow = Geometry::line_string(&[(0.0, 0.0), (3.0, 4.0)]).unwrap();

        assert!(!fast.has_z().unwrap());
        assert!(fast.equals(&slow).unwrap());
    }

    #[test]
    fn coord3_slice_reinterprets_as_a_buffer_with_no_copy() {
        let coords = [
            Coord3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Coord3 {
                x: 3.0,
                y: 4.0,
                z: 0.0,
            },
        ];
        // #[repr(C)] guarantees this is sound.
        let buffer: &[f64] =
            unsafe { std::slice::from_raw_parts(coords.as_ptr().cast::<f64>(), coords.len() * 3) };
        let line = Geometry::line_string_from_interleaved(buffer, Dimensions::Xyz).unwrap();
        assert_eq!(line.length().unwrap(), 5.0);
    }

    #[test]
    fn ragged_buffers_and_short_rings_are_rejected() {
        assert!(Geometry::line_string_from_interleaved(&[0.0, 0.0, 1.0], Dimensions::Xy).is_err());
        assert!(Geometry::line_string_from_interleaved(&[0.0, 0.0], Dimensions::Xy).is_err());
        assert!(
            Geometry::linear_ring_from_interleaved(&[0.0, 0.0, 1.0, 1.0], Dimensions::Xy).is_err()
        );
    }

    #[test]
    fn undersized_output_buffer_is_rejected() {
        let line = Geometry::line_string(&[(0.0, 0.0), (1.0, 1.0)]).unwrap();
        let mut small = [0.0; 3];
        assert!(line.write_interleaved(&mut small, Dimensions::Xy).is_err());
    }
}
