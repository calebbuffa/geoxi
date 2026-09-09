//! Linear referencing, normalisation, precision, and preparation.

use crate::internal_prelude::*;

impl Geometry {
    pub fn line_locate_point(&self, point: &Geometry, normalized: bool) -> Result<f64> {
        let out = if normalized {
            unsafe {
                GEOSProjectNormalized_r(self.ctx.raw(), self.ptr.as_ptr(), point.ptr.as_ptr())
            }
        } else {
            unsafe { GEOSProject_r(self.ctx.raw(), self.ptr.as_ptr(), point.ptr.as_ptr()) }
        };
        if out.is_nan() {
            Err(Error::Failure("projecting point on line"))
        } else {
            Ok(out)
        }
    }

    pub fn line_interpolate_point(&self, distance: f64, normalized: bool) -> Result<Self> {
        let ptr = if normalized {
            unsafe { GEOSInterpolateNormalized_r(self.ctx.raw(), self.ptr.as_ptr(), distance) }
        } else {
            unsafe { GEOSInterpolate_r(self.ctx.raw(), self.ptr.as_ptr(), distance) }
        };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("interpolating point on line"))
    }

    pub fn line_merge(&self, directed: bool) -> Result<Self> {
        if directed {
            unary_ptr(self, GEOSLineMergeDirected_r, "directed line merge")
        } else {
            unary_ptr(self, GEOSLineMerge_r, "line merge")
        }
    }

    pub fn line_substring(&self, start_fraction: f64, end_fraction: f64) -> Result<Self> {
        unary_geom(
            self,
            |ctx, g| unsafe { GEOSLineSubstring_r(ctx, g, start_fraction, end_fraction) },
            "line substring",
        )
    }

    pub fn normalize(&mut self) -> Result<()> {
        // `GEOSNormalize` returns 0 on success and -1 on failure.
        check_zero_code(
            unsafe { GEOSNormalize_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "normalizing geometry",
        )
    }

    pub fn normalized(&self) -> Result<Self> {
        let mut out = self.try_clone()?;
        out.normalize()?;
        Ok(out)
    }

    pub fn orient_polygons(&mut self, exterior_cw: bool) -> Result<()> {
        // Unlike most GEOS entry points this one returns 0 on success and -1 on
        // failure, so it cannot go through `check_code`.
        let code = unsafe {
            GEOSOrientPolygons_r(self.ctx.raw(), self.ptr.as_ptr(), exterior_cw as c_int)
        };
        check_zero_code(code, "orienting polygons")
    }

    pub fn set_precision(&self, grid_size: f64, mode: PrecisionMode) -> Result<Self> {
        let ptr = unsafe {
            GEOSGeom_setPrecision_r(self.ctx.raw(), self.ptr.as_ptr(), grid_size, mode as c_int)
        };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("setting precision"))
    }

    pub fn precision(&self) -> f64 {
        unsafe { GEOSGeom_getPrecision_r(self.ctx.raw(), self.ptr.as_ptr()) }
    }

    pub fn prepared(&self) -> Result<PreparedGeometry> {
        PreparedGeometry::new(self)
    }
}
