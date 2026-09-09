//! Internal FFI plumbing shared by the public modules.

use crate::internal_prelude::*;

/// Read a [`CoordLike`] into the canonical [`Coord`] representation.
#[inline]
pub(crate) fn to_coord<C: CoordLike>(coord: &C) -> Coord {
    Coord {
        x: coord.x(),
        y: coord.y(),
        z: coord.z(),
        m: coord.m(),
    }
}

/// Read a slice of [`CoordLike`] into canonical [`Coord`]s.
///
/// One allocation for the whole slice; the per-element accessors inline away.
#[inline]
pub(crate) fn to_coords<C: CoordLike>(coords: &[C]) -> Vec<Coord> {
    coords.iter().map(to_coord).collect()
}

pub(crate) fn coord_type_for(coords: &[Coord]) -> Result<CoordType> {
    let has_z = coords.iter().any(|c| c.z.is_some());
    let has_m = coords.iter().any(|c| c.m.is_some());
    if coords
        .iter()
        .any(|c| c.z.is_some() != has_z || c.m.is_some() != has_m)
    {
        return Err(Error::InvalidInput(
            "all coordinates must have consistent Z/M dimensions",
        ));
    }
    Ok(match (has_z, has_m) {
        (false, false) => CoordType::XY,
        (true, false) => CoordType::XYZ,
        (false, true) => CoordType::XYM,
        (true, true) => CoordType::XYZM,
    })
}

pub(crate) struct XYTrampoline<F> {
    pub(crate) f: F,
    pub(crate) err: Option<Error>,
}

pub(crate) struct XYZTrampoline<F> {
    pub(crate) f: F,
    pub(crate) err: Option<Error>,
}

pub(crate) unsafe extern "C" fn transform_xy_callback<F>(
    x: *mut c_double,
    y: *mut c_double,
    userdata: *mut c_void,
) -> c_int
where
    F: Fn(f64, f64) -> Result<(f64, f64)> + Send + Sync + 'static,
{
    unsafe {
        let trampoline = &mut *(userdata as *mut XYTrampoline<F>);
        match (trampoline.f)(*x, *y) {
            Ok((nx, ny)) => {
                *x = nx;
                *y = ny;
                1
            }
            Err(err) => {
                trampoline.err = Some(err);
                0
            }
        }
    }
}

pub(crate) unsafe extern "C" fn transform_xyz_callback<F>(
    x: *mut c_double,
    y: *mut c_double,
    z: *mut c_double,
    userdata: *mut c_void,
) -> c_int
where
    F: Fn(f64, f64, f64) -> Result<(f64, f64, f64)> + Send + Sync + 'static,
{
    unsafe {
        let trampoline = &mut *(userdata as *mut XYZTrampoline<F>);
        match (trampoline.f)(*x, *y, *z) {
            Ok((nx, ny, nz)) => {
                *x = nx;
                *y = ny;
                *z = nz;
                1
            }
            Err(err) => {
                trampoline.err = Some(err);
                0
            }
        }
    }
}

pub(crate) fn affine_origin_xy(geometry: &Geometry, origin: AffineOrigin) -> Result<(f64, f64)> {
    Ok(match origin {
        AffineOrigin::Center => {
            let b = geometry.bounds()?;
            ((b.xmax + b.xmin) / 2.0, (b.ymax + b.ymin) / 2.0)
        }
        AffineOrigin::Centroid => {
            let c = geometry.centroid()?;
            (c.x()?, c.y()?)
        }
        AffineOrigin::Point(p) => (p.x, p.y),
    })
}

pub(crate) fn affine_origin_xyz(
    geometry: &Geometry,
    origin: AffineOrigin,
) -> Result<(f64, f64, f64)> {
    Ok(match origin {
        AffineOrigin::Center => {
            let b = geometry.bounds()?;
            ((b.xmax + b.xmin) / 2.0, (b.ymax + b.ymin) / 2.0, 0.0)
        }
        AffineOrigin::Centroid => {
            let c = geometry.centroid()?;
            let z = if geometry.has_z()? {
                c.z().unwrap_or(0.0)
            } else {
                0.0
            };
            (c.x()?, c.y()?, z)
        }
        AffineOrigin::Point(p) => (p.x, p.y, p.z),
    })
}

pub(crate) fn coord_sequence(
    ctx: &Context,
    coords: &[Coord],
    coord_type: CoordType,
) -> Result<NonNull<GEOSCoordSequence>> {
    let mut data = Vec::with_capacity(coords.len() * coord_type.width());
    for coord in coords {
        data.push(coord.x);
        data.push(coord.y);
        if let Some(z) = coord.z {
            data.push(z);
        }
        if let Some(m) = coord.m {
            data.push(m);
        }
    }
    NonNull::new(unsafe {
        GEOSCoordSeq_copyFromBuffer_r(
            ctx.raw(),
            data.as_ptr(),
            coords.len() as c_uint,
            coord_type.has_z(),
            coord_type.has_m(),
        )
    })
    .ok_or(Error::Null("creating coordinate sequence"))
}

pub(crate) fn ring_in_context(ctx: &Context, coords: &[Coord]) -> Result<Geometry> {
    if coords.len() < 4 {
        return Err(Error::InvalidInput(
            "a polygon ring requires at least four coordinates",
        ));
    }
    let coord_type = coord_type_for(coords)?;
    let seq = coord_sequence(ctx, coords, coord_type)?;
    let ptr = unsafe { GEOSGeom_createLinearRing_r(ctx.raw(), seq.as_ptr()) };
    unsafe { Geometry::from_raw(ctx.clone(), ptr) }
        .map_err(|_| Error::Null("creating polygon ring"))
}

pub(crate) fn check_code(code: c_int, operation: &'static str) -> Result<()> {
    if code == 0 {
        Err(Error::Failure(operation))
    } else {
        Ok(())
    }
}

/// For the handful of GEOS entry points that follow the C convention of
/// returning `0` on success and `-1` on failure.
pub(crate) fn check_zero_code(code: c_int, operation: &'static str) -> Result<()> {
    if code == -1 {
        Err(Error::Failure(operation))
    } else {
        Ok(())
    }
}

pub(crate) fn predicate(value: c_char, operation: &'static str) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        v => Err(Error::Predicate {
            operation,
            value: v,
        }),
    }
}

pub(crate) fn unary_ptr(
    geom: &Geometry,
    f: unsafe extern "C" fn(GEOSContextHandle_t, *const GEOSGeometry) -> *mut GEOSGeometry,
    operation: &'static str,
) -> Result<Geometry> {
    let ptr = unsafe { f(geom.ctx.raw(), geom.ptr.as_ptr()) };
    unsafe { Geometry::from_raw(geom.ctx.clone(), ptr) }.map_err(|_| Error::Null(operation))
}

pub(crate) fn unary_geom(
    geom: &Geometry,
    f: impl FnOnce(GEOSContextHandle_t, *const GEOSGeometry) -> *mut GEOSGeometry,
    operation: &'static str,
) -> Result<Geometry> {
    let ptr = f(geom.ctx.raw(), geom.ptr.as_ptr());
    unsafe { Geometry::from_raw(geom.ctx.clone(), ptr) }.map_err(|_| Error::Null(operation))
}

/// Ensure `geom` belongs to `ctx`, migrating it if not.
pub(crate) fn aligned_to<'a>(
    ctx: &Context,
    geom: &'a Geometry,
) -> Result<std::borrow::Cow<'a, Geometry>> {
    if geom.ctx.same(ctx) {
        Ok(std::borrow::Cow::Borrowed(geom))
    } else {
        Ok(std::borrow::Cow::Owned(geom.to_context(ctx)?))
    }
}

/// Ensure two operands share a context, migrating `right` into `left`'s if not.
///
/// GEOS is undefined if a geometry is passed to a context that did not allocate
/// it, and geometries built without an explicit context each own a private one,
/// so this check cannot be skipped.
pub(crate) fn aligned<'a>(
    left: &Geometry,
    right: &'a Geometry,
) -> Result<std::borrow::Cow<'a, Geometry>> {
    aligned_to(&left.ctx, right)
}

pub(crate) fn binary_ptr(
    left: &Geometry,
    right: &Geometry,
    f: unsafe extern "C" fn(
        GEOSContextHandle_t,
        *const GEOSGeometry,
        *const GEOSGeometry,
    ) -> *mut GEOSGeometry,
    operation: &'static str,
) -> Result<Geometry> {
    let right = aligned(left, right)?;
    let ptr = unsafe { f(left.ctx.raw(), left.ptr.as_ptr(), right.ptr.as_ptr()) };
    unsafe { Geometry::from_raw(left.ctx.clone(), ptr) }.map_err(|_| Error::Null(operation))
}

pub(crate) fn binary_geom(
    left: &Geometry,
    right: &Geometry,
    f: impl FnOnce(GEOSContextHandle_t, *const GEOSGeometry, *const GEOSGeometry) -> *mut GEOSGeometry,
    operation: &'static str,
) -> Result<Geometry> {
    let right = aligned(left, right)?;
    let ptr = f(left.ctx.raw(), left.ptr.as_ptr(), right.ptr.as_ptr());
    unsafe { Geometry::from_raw(left.ctx.clone(), ptr) }.map_err(|_| Error::Null(operation))
}

pub(crate) fn binary_predicate(
    left: &Geometry,
    right: &Geometry,
    f: unsafe extern "C" fn(
        GEOSContextHandle_t,
        *const GEOSGeometry,
        *const GEOSGeometry,
    ) -> c_char,
    operation: &'static str,
) -> Result<bool> {
    let right = aligned(left, right)?;
    predicate(
        unsafe { f(left.ctx.raw(), left.ptr.as_ptr(), right.ptr.as_ptr()) },
        operation,
    )
}

pub(crate) fn scalar_measure(
    geom: &Geometry,
    f: unsafe extern "C" fn(GEOSContextHandle_t, *const GEOSGeometry, *mut c_double) -> c_int,
    operation: &'static str,
) -> Result<f64> {
    let mut out = 0.0;
    check_code(
        unsafe { f(geom.ctx.raw(), geom.ptr.as_ptr(), &mut out) },
        operation,
    )?;
    Ok(out)
}

pub(crate) fn binary_measure(
    left: &Geometry,
    right: &Geometry,
    f: unsafe extern "C" fn(
        GEOSContextHandle_t,
        *const GEOSGeometry,
        *const GEOSGeometry,
        *mut c_double,
    ) -> c_int,
    operation: &'static str,
) -> Result<f64> {
    let right = aligned(left, right)?;
    let mut out = 0.0;
    check_code(
        unsafe {
            f(
                left.ctx.raw(),
                left.ptr.as_ptr(),
                right.ptr.as_ptr(),
                &mut out,
            )
        },
        operation,
    )?;
    Ok(out)
}

pub(crate) fn scalar_coord(
    geom: &Geometry,
    f: unsafe extern "C" fn(GEOSContextHandle_t, *const GEOSGeometry, *mut c_double) -> c_int,
    operation: &'static str,
) -> Result<f64> {
    let mut out = 0.0;
    check_code(
        unsafe { f(geom.ctx.raw(), geom.ptr.as_ptr(), &mut out) },
        operation,
    )?;
    Ok(out)
}
