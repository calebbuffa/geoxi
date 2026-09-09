//! Spatial predicates and DE-9IM relate.

use crate::internal_prelude::*;

impl Geometry {
    pub fn is_empty(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSisEmpty_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "is_empty",
        )
    }

    pub fn is_simple(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSisSimple_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "is_simple",
        )
    }

    pub fn is_ring(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSisRing_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "is_ring",
        )
    }

    pub fn is_closed(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSisClosed_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "is_closed",
        )
    }

    pub fn is_valid(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSisValid_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "is_valid",
        )
    }

    pub fn is_valid_reason(&self) -> Result<String> {
        let ptr = unsafe { GEOSisValidReason_r(self.ctx.raw(), self.ptr.as_ptr()) };
        self.take_string(ptr, "reading validity reason")
    }

    pub fn relate(&self, other: &Geometry) -> Result<String> {
        let ptr = unsafe { GEOSRelate_r(self.ctx.raw(), self.ptr.as_ptr(), other.ptr.as_ptr()) };
        self.take_string(ptr, "relating geometries")
    }

    pub fn relate_pattern(&self, other: &Geometry, pattern: &str) -> Result<bool> {
        let pattern = CString::new(pattern)?;
        predicate(
            unsafe {
                GEOSRelatePattern_r(
                    self.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    pattern.as_ptr(),
                )
            },
            "relate_pattern",
        )
    }

    pub fn intersects(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSIntersects_r, "intersects")
    }

    pub fn disjoint(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSDisjoint_r, "disjoint")
    }

    pub fn touches(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSTouches_r, "touches")
    }

    pub fn crosses(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSCrosses_r, "crosses")
    }

    pub fn within(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSWithin_r, "within")
    }

    pub fn contains(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSContains_r, "contains")
    }

    pub fn overlaps(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSOverlaps_r, "overlaps")
    }

    pub fn equals(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSEquals_r, "equals")
    }

    pub fn equals_exact(&self, other: &Geometry, tolerance: f64) -> Result<bool> {
        predicate(
            unsafe {
                GEOSEqualsExact_r(
                    self.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    tolerance,
                )
            },
            "equals_exact",
        )
    }

    pub fn equals_identical(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSEqualsIdentical_r, "equals_identical")
    }

    pub fn covers(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSCovers_r, "covers")
    }

    pub fn covered_by(&self, other: &Geometry) -> Result<bool> {
        binary_predicate(self, other, GEOSCoveredBy_r, "covered_by")
    }

    pub fn dwithin(&self, other: &Geometry, distance: f64) -> Result<bool> {
        predicate(
            unsafe {
                GEOSDistanceWithin_r(
                    self.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    distance,
                )
            },
            "dwithin",
        )
    }
}
