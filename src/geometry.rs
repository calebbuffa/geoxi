//! The owned `Geometry` handle and its full method surface.

use crate::internal_prelude::*;

#[derive(Debug)]
pub struct Geometry {
    pub(crate) ctx: Context,
    pub(crate) ptr: NonNull<GEOSGeometry>,
}

// SAFETY: a geometry owns its GEOS pointer and its context, so it may be moved
// to another thread. It is deliberately not `Sync`: GEOS forbids using one
// context from two threads concurrently, and every method here needs the
// context. Move a geometry between threads, or rebuild it per thread.
unsafe impl Send for Geometry {}

impl Clone for Geometry {
    fn clone(&self) -> Self {
        self.try_clone().expect("GEOS geometry clone failed")
    }
}

impl Drop for Geometry {
    fn drop(&mut self) {
        unsafe { GEOSGeom_destroy_r(self.ctx.raw(), self.ptr.as_ptr()) };
    }
}

impl Geometry {
    /// Take ownership of a raw GEOS geometry pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid geometry allocated against `ctx` and not owned by
    /// any other value. Ownership transfers to the returned [`Geometry`], which
    /// destroys it on drop, so the caller must not free or reuse `ptr`.
    pub unsafe fn from_raw(ctx: Context, ptr: *mut GEOSGeometry) -> Result<Self> {
        let ptr = NonNull::new(ptr).ok_or(Error::Null("creating geometry from raw pointer"))?;
        Ok(Self { ctx, ptr })
    }

    pub const fn as_raw(&self) -> *const GEOSGeometry {
        self.ptr.as_ptr()
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }

    pub fn try_clone(&self) -> Result<Self> {
        let ptr = unsafe { GEOSGeom_clone_r(self.ctx.raw(), self.ptr.as_ptr()) };
        unsafe { Self::from_raw(self.ctx.clone(), ptr) }
    }

    pub fn from_wkt(wkt: &str) -> Result<Self> {
        let ctx = Context::current()?;
        let wkt = CString::new(wkt)?;
        let reader = NonNull::new(unsafe { GEOSWKTReader_create_r(ctx.raw()) })
            .ok_or(Error::Null("creating WKT reader"))?;
        let ptr = unsafe { GEOSWKTReader_read_r(ctx.raw(), reader.as_ptr(), wkt.as_ptr()) };
        unsafe { GEOSWKTReader_destroy_r(ctx.raw(), reader.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("reading WKT"))
    }

    pub fn from_wkb(wkb: &[u8]) -> Result<Self> {
        let ctx = Context::current()?;
        Self::from_wkb_in(&ctx, wkb)
    }

    pub(crate) fn from_wkb_in(ctx: &Context, wkb: &[u8]) -> Result<Self> {
        let ptr = unsafe { GEOSGeomFromWKB_buf_r(ctx.raw(), wkb.as_ptr(), wkb.len()) };
        unsafe { Self::from_raw(ctx.clone(), ptr) }.map_err(|_| Error::Null("reading WKB"))
    }

    pub fn from_hex_wkb(hex: &[u8]) -> Result<Self> {
        let ctx = Context::current()?;
        let ptr = unsafe { GEOSGeomFromHEX_buf_r(ctx.raw(), hex.as_ptr(), hex.len()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("reading hex WKB"))
    }

    pub fn from_geojson(geojson: &str) -> Result<Self> {
        let ctx = Context::current()?;
        let geojson = CString::new(geojson)?;
        let reader = NonNull::new(unsafe { GEOSGeoJSONReader_create() })
            .ok_or(Error::Null("creating GeoJSON reader"))?;
        let ptr = unsafe { GEOSGeoJSONReader_readGeometry(reader.as_ptr(), geojson.as_ptr()) };
        unsafe { GEOSGeoJSONReader_destroy(reader.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("reading GeoJSON"))
    }

    /// Create a 2D point.
    pub fn point(x: f64, y: f64) -> Result<Self> {
        let ctx = Context::current()?;
        let ptr = unsafe { GEOSGeom_createPointFromXY_r(ctx.raw(), x, y) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating point"))
    }

    /// Create a 3D point.
    pub fn point_z(x: f64, y: f64, z: f64) -> Result<Self> {
        Self::point_from(Coord::xyz(x, y, z))
    }

    /// Create a point from any [`CoordLike`].
    ///
    /// The point is 3D if the coordinate carries a z.
    pub fn point_from<C: CoordLike>(coord: C) -> Result<Self> {
        let ctx = Context::current()?;
        let coord = to_coord(&coord);
        let coord_type = coord_type_for(&[coord])?;
        let seq = coord_sequence(&ctx, &[coord], coord_type)?;
        let ptr = unsafe { GEOSGeom_createPoint_r(ctx.raw(), seq.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating point"))
    }

    /// Create a LineString from any slice of [`CoordLike`].
    pub fn line_string<C: CoordLike>(coords: &[C]) -> Result<Self> {
        if coords.len() < 2 {
            return Err(Error::InvalidInput(
                "a LineString requires at least two coordinates",
            ));
        }
        let ctx = Context::current()?;
        let coords = to_coords(coords);
        let coord_type = coord_type_for(&coords)?;
        let seq = coord_sequence(&ctx, &coords, coord_type)?;
        let ptr = unsafe { GEOSGeom_createLineString_r(ctx.raw(), seq.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating LineString"))
    }

    /// Create a LinearRing from any slice of [`CoordLike`].
    pub fn linear_ring<C: CoordLike>(coords: &[C]) -> Result<Self> {
        if coords.len() < 4 {
            return Err(Error::InvalidInput(
                "a LinearRing requires at least four coordinates",
            ));
        }
        let ctx = Context::current()?;
        let coords = to_coords(coords);
        let coord_type = coord_type_for(&coords)?;
        let seq = coord_sequence(&ctx, &coords, coord_type)?;
        let ptr = unsafe { GEOSGeom_createLinearRing_r(ctx.raw(), seq.as_ptr()) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating LinearRing"))
    }

    /// Create a Polygon from a shell and any number of holes.
    ///
    /// Holes may be any iterable of slice-like containers, so `&[Vec<C>]`,
    /// `&[&[C]]`, and `&Vec<Vec<C>>` all work. Pass [`no_holes`] when there are
    /// none.
    ///
    /// [`no_holes`]: crate::no_holes
    pub fn polygon<'h, C, H, I>(shell: &[C], holes: I) -> Result<Self>
    where
        C: CoordLike + 'h,
        H: AsRef<[C]> + 'h,
        I: IntoIterator<Item = &'h H>,
    {
        let ctx = Context::current()?;
        let shell = ring_in_context(&ctx, &to_coords(shell))?;
        let mut raw_holes = Vec::new();
        for hole in holes {
            raw_holes.push(ring_in_context(&ctx, &to_coords(hole.as_ref()))?.into_raw());
        }
        let ptr = unsafe {
            GEOSGeom_createPolygon_r(
                ctx.raw(),
                shell.into_raw(),
                raw_holes.as_mut_ptr(),
                raw_holes.len() as c_uint,
            )
        };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating Polygon"))
    }

    /// Create an axis-aligned rectangle.
    pub fn rectangle(xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> Result<Self> {
        let ctx = Context::current()?;
        let ptr = unsafe { GEOSGeom_createRectangle_r(ctx.raw(), xmin, ymin, xmax, ymax) };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating rectangle"))
    }

    /// Rebuild this geometry in the calling thread's context.
    ///
    /// Only useful after moving a geometry between threads: it keeps its
    /// original context, so operations against locally-built geometries have to
    /// migrate it every time. Calling `rehome` once pays that cost up front.
    /// A no-op (a plain clone) if the geometry is already local.
    pub fn rehome(&self) -> Result<Self> {
        self.to_context(&Context::current()?)
    }

    /// Move this geometry into `ctx`, cloning it if it is already there.
    pub(crate) fn to_context(&self, ctx: &Context) -> Result<Self> {
        if self.ctx.same(ctx) {
            return self.try_clone();
        }
        // GEOS has no cross-context clone, so round-trip through WKB, which
        // preserves Z, M, and SRID exactly.
        let wkb = self.to_wkb()?;
        let mut moved = Self::from_wkb_in(ctx, &wkb)?;
        moved.set_srid(self.srid());
        Ok(moved)
    }

    /// Build a collection from its members.
    ///
    /// Members that were moved in from another thread are migrated first.
    pub fn collection(kind: CollectionType, geometries: Vec<Geometry>) -> Result<Self> {
        let ctx = Context::current()?;
        if geometries.is_empty() {
            let ptr = unsafe { GEOSGeom_createEmptyCollection_r(ctx.raw(), kind as c_int) };
            return unsafe { Self::from_raw(ctx, ptr) }
                .map_err(|_| Error::Null("creating empty collection"));
        }
        let mut raw = Vec::with_capacity(geometries.len());
        for geom in &geometries {
            // Cloning a geometry through a context that did not allocate it is
            // undefined, so foreign members must be migrated first.
            let member = crate::ffi::aligned_to(&ctx, geom)?;
            let cloned = unsafe { GEOSGeom_clone_r(ctx.raw(), member.ptr.as_ptr()) };
            if cloned.is_null() {
                return Err(Error::Null("cloning collection member"));
            }
            raw.push(cloned);
        }
        let ptr = unsafe {
            GEOSGeom_createCollection_r(
                ctx.raw(),
                kind as c_int,
                raw.as_mut_ptr(),
                raw.len() as c_uint,
            )
        };
        unsafe { Self::from_raw(ctx, ptr) }.map_err(|_| Error::Null("creating collection"))
    }

    pub fn to_wkt(&self) -> Result<String> {
        let writer = NonNull::new(unsafe { GEOSWKTWriter_create_r(self.ctx.raw()) })
            .ok_or(Error::Null("creating WKT writer"))?;
        unsafe { GEOSWKTWriter_setTrim_r(self.ctx.raw(), writer.as_ptr(), 1) };
        unsafe { GEOSWKTWriter_setOutputDimension_r(self.ctx.raw(), writer.as_ptr(), 4) };
        let ptr =
            unsafe { GEOSWKTWriter_write_r(self.ctx.raw(), writer.as_ptr(), self.ptr.as_ptr()) };
        unsafe { GEOSWKTWriter_destroy_r(self.ctx.raw(), writer.as_ptr()) };
        self.take_string(ptr, "writing WKT")
    }

    /// Serialize to WKB.
    ///
    /// Z ordinates are preserved: the writer is configured for 3 output
    /// dimensions, unlike the GEOS convenience function which defaults to 2.
    pub fn to_wkb(&self) -> Result<Vec<u8>> {
        self.write_wkb(false)
    }

    fn write_wkb(&self, include_srid: bool) -> Result<Vec<u8>> {
        let writer = NonNull::new(unsafe { GEOSWKBWriter_create_r(self.ctx.raw()) })
            .ok_or(Error::Null("creating WKB writer"))?;
        unsafe {
            GEOSWKBWriter_setOutputDimension_r(self.ctx.raw(), writer.as_ptr(), 3);
            GEOSWKBWriter_setIncludeSRID_r(self.ctx.raw(), writer.as_ptr(), include_srid as c_char);
        }
        let mut size = 0usize;
        let ptr = unsafe {
            GEOSWKBWriter_write_r(
                self.ctx.raw(),
                writer.as_ptr(),
                self.ptr.as_ptr(),
                &mut size,
            )
        };
        unsafe { GEOSWKBWriter_destroy_r(self.ctx.raw(), writer.as_ptr()) };
        self.take_bytes(ptr, size, "writing WKB")
    }

    /// Serialize to hex-encoded WKB. Z ordinates are preserved.
    pub fn to_hex_wkb(&self) -> Result<Vec<u8>> {
        let writer = NonNull::new(unsafe { GEOSWKBWriter_create_r(self.ctx.raw()) })
            .ok_or(Error::Null("creating WKB writer"))?;
        unsafe { GEOSWKBWriter_setOutputDimension_r(self.ctx.raw(), writer.as_ptr(), 3) };
        let mut size = 0usize;
        let ptr = unsafe {
            GEOSWKBWriter_writeHEX_r(
                self.ctx.raw(),
                writer.as_ptr(),
                self.ptr.as_ptr(),
                &mut size,
            )
        };
        unsafe { GEOSWKBWriter_destroy_r(self.ctx.raw(), writer.as_ptr()) };
        self.take_bytes(ptr, size, "writing hex WKB")
    }

    pub fn to_geojson(&self, indent: Option<i32>) -> Result<String> {
        let writer = NonNull::new(unsafe { GEOSGeoJSONWriter_create() })
            .ok_or(Error::Null("creating GeoJSON writer"))?;
        unsafe { GEOSGeoJSONWriter_setOutputDimension(writer.as_ptr(), 3) };
        let ptr = unsafe {
            GEOSGeoJSONWriter_writeGeometry(
                writer.as_ptr(),
                self.ptr.as_ptr(),
                indent.unwrap_or(-1),
            )
        };
        unsafe { GEOSGeoJSONWriter_destroy(writer.as_ptr()) };
        self.take_string(ptr, "writing GeoJSON")
    }

    pub(crate) fn into_raw(self) -> *mut GEOSGeometry {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    pub(crate) fn take_string(&self, ptr: *mut c_char, operation: &'static str) -> Result<String> {
        let ptr = NonNull::new(ptr).ok_or(Error::Null(operation))?;
        let s = unsafe { CStr::from_ptr(ptr.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        unsafe { GEOSFree_r(self.ctx.raw(), ptr.as_ptr().cast()) };
        Ok(s)
    }

    pub(crate) fn take_bytes(
        &self,
        ptr: *mut c_uchar,
        size: usize,
        operation: &'static str,
    ) -> Result<Vec<u8>> {
        let ptr = NonNull::new(ptr).ok_or(Error::Null(operation))?;
        let bytes = unsafe { std::slice::from_raw_parts(ptr.as_ptr(), size) }.to_vec();
        unsafe { GEOSFree_r(self.ctx.raw(), ptr.as_ptr().cast()) };
        Ok(bytes)
    }
}
