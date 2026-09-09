//! Measurements: area, length, and the distance family.

use crate::internal_prelude::*;

impl Geometry {
    pub fn area(&self) -> Result<f64> {
        scalar_measure(self, GEOSArea_r, "area")
    }

    pub fn length(&self) -> Result<f64> {
        scalar_measure(self, GEOSLength_r, "length")
    }

    pub fn distance(&self, other: &Geometry) -> Result<f64> {
        binary_measure(self, other, GEOSDistance_r, "distance")
    }

    pub fn distance_indexed(&self, other: &Geometry) -> Result<f64> {
        binary_measure(self, other, GEOSDistanceIndexed_r, "indexed distance")
    }

    pub fn hausdorff_distance(&self, other: &Geometry) -> Result<f64> {
        binary_measure(self, other, GEOSHausdorffDistance_r, "Hausdorff distance")
    }

    pub fn hausdorff_distance_densify(
        &self,
        other: &Geometry,
        densify_fraction: f64,
    ) -> Result<f64> {
        let mut out = 0.0;
        check_code(
            unsafe {
                GEOSHausdorffDistanceDensify_r(
                    self.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    densify_fraction,
                    &mut out,
                )
            },
            "Hausdorff distance with densification",
        )?;
        Ok(out)
    }

    pub fn frechet_distance(&self, other: &Geometry) -> Result<f64> {
        binary_measure(self, other, GEOSFrechetDistance_r, "Frechet distance")
    }

    pub fn frechet_distance_densify(&self, other: &Geometry, densify_fraction: f64) -> Result<f64> {
        let mut out = 0.0;
        check_code(
            unsafe {
                GEOSFrechetDistanceDensify_r(
                    self.ctx.raw(),
                    self.ptr.as_ptr(),
                    other.ptr.as_ptr(),
                    densify_fraction,
                    &mut out,
                )
            },
            "Frechet distance with densification",
        )?;
        Ok(out)
    }

    pub fn minimum_clearance(&self) -> Result<f64> {
        let mut out = 0.0;
        check_code(
            unsafe { GEOSMinimumClearance_r(self.ctx.raw(), self.ptr.as_ptr(), &mut out) },
            "minimum clearance",
        )?;
        Ok(out)
    }
}
