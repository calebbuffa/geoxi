//! Structural accessors: type, SRID, parts, rings, coordinates.

use crate::internal_prelude::*;

impl Geometry {
    pub fn geometry_type(&self) -> Result<GeometryType> {
        let value = unsafe { GEOSGeomTypeId_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if value < 0 {
            return Err(Error::Failure("reading geometry type"));
        }
        Ok(GeometryType::from(value))
    }

    pub fn type_name(&self) -> Result<String> {
        let ptr = unsafe { GEOSGeomType_r(self.ctx.raw(), self.ptr.as_ptr()) };
        self.take_string(ptr, "reading geometry type name")
    }

    pub fn srid(&self) -> i32 {
        unsafe { GEOSGetSRID_r(self.ctx.raw(), self.ptr.as_ptr()) }
    }

    pub fn set_srid(&mut self, srid: i32) {
        unsafe { GEOSSetSRID_r(self.ctx.raw(), self.ptr.as_ptr(), srid) }
    }

    pub fn coordinate_dimension(&self) -> Result<i32> {
        let dim = unsafe { GEOSGeom_getCoordinateDimension_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if dim <= 0 {
            Err(Error::Failure("reading coordinate dimension"))
        } else {
            Ok(dim)
        }
    }

    pub fn dimensions(&self) -> Result<i32> {
        let dim = unsafe { GEOSGeom_getDimensions_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if dim < 0 {
            Err(Error::Failure("reading topological dimension"))
        } else {
            Ok(dim)
        }
    }

    pub fn num_coordinates(&self) -> Result<usize> {
        let n = unsafe { GEOSGetNumCoordinates_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if n < 0 {
            Err(Error::Failure("counting coordinates"))
        } else {
            Ok(n as usize)
        }
    }

    pub fn num_geometries(&self) -> Result<usize> {
        let n = unsafe { GEOSGetNumGeometries_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if n < 0 {
            Err(Error::Failure("counting geometries"))
        } else {
            Ok(n as usize)
        }
    }

    pub fn geometry_n(&self, index: usize) -> Result<Self> {
        let ptr = unsafe { GEOSGetGeometryN_r(self.ctx.raw(), self.ptr.as_ptr(), index as c_int) };
        let clone = unsafe { GEOSGeom_clone_r(self.ctx.raw(), ptr) };
        unsafe { Self::from_raw(self.ctx.clone(), clone) }
            .map_err(|_| Error::Null("reading collection member"))
    }

    pub fn parts(&self) -> Result<Vec<Self>> {
        let count = self.num_geometries()?;
        (0..count).map(|index| self.geometry_n(index)).collect()
    }

    pub fn rings(&self) -> Result<Vec<Self>> {
        let mut rings = Vec::new();
        if self.geometry_type()? == GeometryType::Polygon {
            rings.push(self.exterior_ring()?);
            for index in 0..self.num_interior_rings()? {
                rings.push(self.interior_ring_n(index)?);
            }
        } else {
            for part in self.parts()? {
                rings.extend(part.rings()?);
            }
        }
        Ok(rings)
    }

    pub fn num_points(&self) -> Result<usize> {
        let n = unsafe { GEOSGeomGetNumPoints_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if n < 0 {
            Err(Error::Failure("counting points"))
        } else {
            Ok(n as usize)
        }
    }

    pub fn point_n(&self, index: usize) -> Result<Self> {
        let ptr = unsafe { GEOSGeomGetPointN_r(self.ctx.raw(), self.ptr.as_ptr(), index as c_int) };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("reading point by index"))
    }

    pub fn start_point(&self) -> Result<Self> {
        let ptr = unsafe { GEOSGeomGetStartPoint_r(self.ctx.raw(), self.ptr.as_ptr()) };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("reading start point"))
    }

    pub fn end_point(&self) -> Result<Self> {
        let ptr = unsafe { GEOSGeomGetEndPoint_r(self.ctx.raw(), self.ptr.as_ptr()) };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
            .map_err(|_| Error::Null("reading end point"))
    }

    pub fn exterior_ring(&self) -> Result<Self> {
        let ptr = unsafe { GEOSGetExteriorRing_r(self.ctx.raw(), self.ptr.as_ptr()) };
        let clone = unsafe { GEOSGeom_clone_r(self.ctx.raw(), ptr) };
        unsafe { Self::from_raw(self.ctx.clone(), clone) }
            .map_err(|_| Error::Null("reading exterior ring"))
    }

    pub fn num_interior_rings(&self) -> Result<usize> {
        let n = unsafe { GEOSGetNumInteriorRings_r(self.ctx.raw(), self.ptr.as_ptr()) };
        if n < 0 {
            Err(Error::Failure("counting interior rings"))
        } else {
            Ok(n as usize)
        }
    }

    pub fn interior_ring_n(&self, index: usize) -> Result<Self> {
        let ptr =
            unsafe { GEOSGetInteriorRingN_r(self.ctx.raw(), self.ptr.as_ptr(), index as c_int) };
        let clone = unsafe { GEOSGeom_clone_r(self.ctx.raw(), ptr) };
        unsafe { Self::from_raw(self.ctx.clone(), clone) }
            .map_err(|_| Error::Null("reading interior ring"))
    }

    pub fn bounds(&self) -> Result<Bounds> {
        let mut xmin = 0.0;
        let mut ymin = 0.0;
        let mut xmax = 0.0;
        let mut ymax = 0.0;
        check_code(
            unsafe {
                GEOSGeom_getExtent_r(
                    self.ctx.raw(),
                    self.ptr.as_ptr(),
                    &mut xmin,
                    &mut ymin,
                    &mut xmax,
                    &mut ymax,
                )
            },
            "reading bounds",
        )?;
        Ok(Bounds {
            xmin,
            ymin,
            xmax,
            ymax,
        })
    }

    pub fn x(&self) -> Result<f64> {
        scalar_coord(self, GEOSGeomGetX_r, "reading X")
    }

    pub fn y(&self) -> Result<f64> {
        scalar_coord(self, GEOSGeomGetY_r, "reading Y")
    }

    pub fn z(&self) -> Result<f64> {
        scalar_coord(self, GEOSGeomGetZ_r, "reading Z")
    }

    pub fn m(&self) -> Result<f64> {
        scalar_coord(self, GEOSGeomGetM_r, "reading M")
    }

    pub fn has_z(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSHasZ_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "has_z",
        )
    }

    pub fn has_m(&self) -> Result<bool> {
        predicate(
            unsafe { GEOSHasM_r(self.ctx.raw(), self.ptr.as_ptr()) },
            "has_m",
        )
    }

    /// Read this geometry's coordinate sequence into any [`CoordLike`] type.
    ///
    /// Only valid for geometries that have a single sequence (Point,
    /// LineString, LinearRing). Z and M are dropped if the target type does not
    /// carry them.
    pub fn coords<C: CoordLike>(&self) -> Result<Vec<C>> {
        let seq = NonNull::new(unsafe {
            GEOSGeom_getCoordSeq_r(self.ctx.raw(), self.ptr.as_ptr()) as *mut GEOSCoordSequence
        })
        .ok_or(Error::Null("reading coordinate sequence"))?;
        let mut size = 0;
        check_code(
            unsafe { GEOSCoordSeq_getSize_r(self.ctx.raw(), seq.as_ptr(), &mut size) },
            "reading coordinate count",
        )?;
        let mut dims = 0;
        check_code(
            unsafe { GEOSCoordSeq_getDimensions_r(self.ctx.raw(), seq.as_ptr(), &mut dims) },
            "reading coordinate dimensions",
        )?;
        let mut coords = Vec::with_capacity(size as usize);
        for i in 0..size {
            let mut x = 0.0;
            let mut y = 0.0;
            check_code(
                unsafe { GEOSCoordSeq_getXY_r(self.ctx.raw(), seq.as_ptr(), i, &mut x, &mut y) },
                "reading coordinate xy",
            )?;
            let z = if dims >= 3 {
                let mut z = 0.0;
                check_code(
                    unsafe { GEOSCoordSeq_getZ_r(self.ctx.raw(), seq.as_ptr(), i, &mut z) },
                    "reading coordinate z",
                )?;
                Some(z)
            } else {
                None
            };
            let m = if dims >= 4 {
                let mut m = 0.0;
                check_code(
                    unsafe { GEOSCoordSeq_getM_r(self.ctx.raw(), seq.as_ptr(), i, &mut m) },
                    "reading coordinate m",
                )?;
                Some(m)
            } else {
                None
            };
            coords.push(C::from_xyzm(x, y, z, m));
        }
        Ok(coords)
    }

    pub fn force_2d(&self) -> Result<Self> {
        self.transform_xy(|x, y| Ok((x, y)))
    }

    pub fn force_3d(&self, z: f64) -> Result<Self> {
        self.transform_xyz(move |x, y, _| Ok((x, y, z)))
    }

    pub fn transform_xy<F>(&self, f: F) -> Result<Self>
    where
        F: Fn(f64, f64) -> Result<(f64, f64)> + Send + Sync + 'static,
    {
        let trampoline = Box::new(XYTrampoline { f, err: None });
        let raw = Box::into_raw(trampoline);
        let ptr = unsafe {
            GEOSGeom_transformXY_r(
                self.ctx.raw(),
                self.ptr.as_ptr(),
                Some(transform_xy_callback::<F>),
                raw.cast(),
            )
        };
        let trampoline = unsafe { Box::from_raw(raw) };
        if let Some(err) = trampoline.err {
            Err(err)
        } else {
            unsafe { Self::from_raw(self.ctx.clone(), ptr) }
                .map_err(|_| Error::Null("transforming XY"))
        }
    }

    pub fn transform_xyz<F>(&self, f: F) -> Result<Self>
    where
        F: Fn(f64, f64, f64) -> Result<(f64, f64, f64)> + Send + Sync + 'static,
    {
        let trampoline = Box::new(XYZTrampoline { f, err: None });
        let raw = Box::into_raw(trampoline);
        let ptr = unsafe {
            GEOSGeom_transformXYZ_r(
                self.ctx.raw(),
                self.ptr.as_ptr(),
                Some(transform_xyz_callback::<F>),
                raw.cast(),
            )
        };
        let trampoline = unsafe { Box::from_raw(raw) };
        if let Some(err) = trampoline.err {
            Err(err)
        } else {
            unsafe { Self::from_raw(self.ctx.clone(), ptr) }
                .map_err(|_| Error::Null("transforming XYZ"))
        }
    }
}
