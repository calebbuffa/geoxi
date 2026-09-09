//! Affine transforms and coordinate rewriting.

use crate::internal_prelude::*;

pub fn has_z(geometry: &Geometry) -> Result<bool> {
    geometry.has_z()
}
pub fn has_m(geometry: &Geometry) -> Result<bool> {
    geometry.has_m()
}
pub fn coord_seq<C: CoordLike>(geometry: &Geometry) -> Result<Vec<C>> {
    geometry.coords()
}
pub fn force_2d(geometry: &Geometry) -> Result<Geometry> {
    geometry.force_2d()
}
pub fn force_3d(geometry: &Geometry, z: f64) -> Result<Geometry> {
    geometry.force_3d(z)
}
pub fn get_parts(geometry: &Geometry) -> Result<GeometryArray> {
    Ok(GeometryArray::new(geometry.parts()?))
}
pub fn get_rings(geometry: &Geometry) -> Result<GeometryArray> {
    Ok(GeometryArray::new(geometry.rings()?))
}
pub fn transform_xy<F>(geometry: &Geometry, f: F) -> Result<Geometry>
where
    F: Fn(f64, f64) -> Result<(f64, f64)> + Send + Sync + 'static,
{
    geometry.transform_xy(f)
}
pub fn transform_xyz<F>(geometry: &Geometry, f: F) -> Result<Geometry>
where
    F: Fn(f64, f64, f64) -> Result<(f64, f64, f64)> + Send + Sync + 'static,
{
    geometry.transform_xyz(f)
}
pub fn affine_transform(geometry: &Geometry, matrix: &[f64]) -> Result<Geometry> {
    match matrix {
        [a, b, d, e, xoff, yoff] => {
            let (a, b, d, e, xoff, yoff) = (*a, *b, *d, *e, *xoff, *yoff);
            if geometry.has_z()? {
                geometry.transform_xyz(move |x, y, z| {
                    Ok((a * x + b * y + xoff, d * x + e * y + yoff, z))
                })
            } else {
                geometry.transform_xy(move |x, y| Ok((a * x + b * y + xoff, d * x + e * y + yoff)))
            }
        }
        [a, b, c, d, e, f, g, h, i, xoff, yoff, zoff] => {
            let (a, b, c, d, e, f, g, h, i, xoff, yoff, zoff) =
                (*a, *b, *c, *d, *e, *f, *g, *h, *i, *xoff, *yoff, *zoff);
            if geometry.has_z()? {
                geometry.transform_xyz(move |x, y, z| {
                    Ok((
                        a * x + b * y + c * z + xoff,
                        d * x + e * y + f * z + yoff,
                        g * x + h * y + i * z + zoff,
                    ))
                })
            } else {
                geometry.transform_xy(move |x, y| Ok((a * x + b * y + xoff, d * x + e * y + yoff)))
            }
        }
        _ => Err(Error::InvalidInput("matrix expects 6 or 12 coefficients")),
    }
}
pub fn rotate(
    geometry: &Geometry,
    angle: f64,
    origin: AffineOrigin,
    use_radians: bool,
) -> Result<Geometry> {
    if geometry.is_empty()? {
        return geometry.try_clone();
    }
    let angle = if use_radians {
        angle
    } else {
        angle * PI / 180.0
    };
    let mut cosp = angle.cos();
    let mut sinp = angle.sin();
    if cosp.abs() < 2.5e-16 {
        cosp = 0.0;
    }
    if sinp.abs() < 2.5e-16 {
        sinp = 0.0;
    }
    let (x0, y0) = affine_origin_xy(geometry, origin)?;
    let matrix = [
        cosp,
        -sinp,
        sinp,
        cosp,
        x0 - x0 * cosp + y0 * sinp,
        y0 - x0 * sinp - y0 * cosp,
    ];
    affine_transform(geometry, &matrix)
}
pub fn scale(
    geometry: &Geometry,
    xfact: f64,
    yfact: f64,
    zfact: f64,
    origin: AffineOrigin,
) -> Result<Geometry> {
    if geometry.is_empty()? {
        return geometry.try_clone();
    }
    let (x0, y0, z0) = affine_origin_xyz(geometry, origin)?;
    let matrix = [
        xfact,
        0.0,
        0.0,
        yfact,
        0.0,
        0.0,
        0.0,
        0.0,
        zfact,
        x0 - x0 * xfact,
        y0 - y0 * yfact,
        z0 - z0 * zfact,
    ];
    affine_transform(geometry, &matrix)
}
pub fn skew(
    geometry: &Geometry,
    xs: f64,
    ys: f64,
    origin: AffineOrigin,
    use_radians: bool,
) -> Result<Geometry> {
    if geometry.is_empty()? {
        return geometry.try_clone();
    }
    let (xs, ys) = if use_radians {
        (xs, ys)
    } else {
        (xs * PI / 180.0, ys * PI / 180.0)
    };
    let mut tanx = xs.tan();
    let mut tany = ys.tan();
    if tanx.abs() < 2.5e-16 {
        tanx = 0.0;
    }
    if tany.abs() < 2.5e-16 {
        tany = 0.0;
    }
    let (x0, y0) = affine_origin_xy(geometry, origin)?;
    let matrix = [1.0, tanx, tany, 1.0, -y0 * tanx, -x0 * tany];
    affine_transform(geometry, &matrix)
}
pub fn translate(geometry: &Geometry, xoff: f64, yoff: f64, zoff: f64) -> Result<Geometry> {
    if geometry.has_z()? {
        let matrix = [
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, xoff, yoff, zoff,
        ];
        affine_transform(geometry, &matrix)
    } else {
        let matrix = [1.0, 0.0, 0.0, 1.0, xoff, yoff];
        affine_transform(geometry, &matrix)
    }
}
