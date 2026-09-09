//! Prepared geometries for repeated predicate evaluation.

use crate::internal_prelude::*;

pub struct PreparedGeometry {
    pub(crate) source: Geometry,
    pub(crate) ptr: NonNull<GEOSPreparedGeometry>,
}

// SAFETY: see the note on `Geometry`. Movable across threads, never shared.
unsafe impl Send for PreparedGeometry {}

impl PreparedGeometry {
    pub fn new(geometry: &Geometry) -> Result<Self> {
        let source = geometry.try_clone()?;
        let ptr = NonNull::new(unsafe {
            GEOSPrepare_r(source.ctx.raw(), source.ptr.as_ptr()).cast_mut()
        })
        .ok_or(Error::Null("preparing geometry"))?;
        Ok(Self { source, ptr })
    }

    pub fn contains(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedContains_r, "prepared contains")
    }
    pub fn contains_properly(&self, other: &Geometry) -> Result<bool> {
        self.pred(
            other,
            GEOSPreparedContainsProperly_r,
            "prepared contains properly",
        )
    }
    pub fn covered_by(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedCoveredBy_r, "prepared covered_by")
    }
    pub fn covers(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedCovers_r, "prepared covers")
    }
    pub fn crosses(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedCrosses_r, "prepared crosses")
    }
    pub fn disjoint(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedDisjoint_r, "prepared disjoint")
    }
    pub fn intersects(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedIntersects_r, "prepared intersects")
    }
    pub fn overlaps(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedOverlaps_r, "prepared overlaps")
    }
    pub fn touches(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedTouches_r, "prepared touches")
    }
    pub fn within(&self, other: &Geometry) -> Result<bool> {
        self.pred(other, GEOSPreparedWithin_r, "prepared within")
    }
    pub fn contains_xy(&self, x: f64, y: f64) -> Result<bool> {
        predicate(
            unsafe { GEOSPreparedContainsXY_r(self.source.ctx.raw(), self.ptr.as_ptr(), x, y) },
            "prepared contains XY",
        )
    }
    pub fn intersects_xy(&self, x: f64, y: f64) -> Result<bool> {
        predicate(
            unsafe { GEOSPreparedIntersectsXY_r(self.source.ctx.raw(), self.ptr.as_ptr(), x, y) },
            "prepared intersects XY",
        )
    }
    pub fn distance(&self, other: &Geometry) -> Result<f64> {
        let other = crate::ffi::aligned_to(&self.source.ctx, other)?;
        let mut out = 0.0;
        check_code(
            unsafe {
                GEOSPreparedDistance_r(
                    self.source.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    &mut out,
                )
            },
            "prepared distance",
        )?;
        Ok(out)
    }
    pub fn dwithin(&self, other: &Geometry, distance: f64) -> Result<bool> {
        let other = crate::ffi::aligned_to(&self.source.ctx, other)?;
        predicate(
            unsafe {
                GEOSPreparedDistanceWithin_r(
                    self.source.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    distance,
                )
            },
            "prepared dwithin",
        )
    }

    fn pred(
        &self,
        other: &Geometry,
        f: unsafe extern "C" fn(
            GEOSContextHandle_t,
            *const GEOSPreparedGeometry,
            *const GEOSGeometry,
        ) -> c_char,
        operation: &'static str,
    ) -> Result<bool> {
        let other = crate::ffi::aligned_to(&self.source.ctx, other)?;
        predicate(
            unsafe { f(self.source.ctx.raw(), self.ptr.as_ptr(), other.ptr.as_ptr()) },
            operation,
        )
    }
}

impl Drop for PreparedGeometry {
    fn drop(&mut self) {
        unsafe { GEOSPreparedGeom_destroy_r(self.source.ctx.raw(), self.ptr.as_ptr()) };
    }
}
