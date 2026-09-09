//! Coordinate-reference-system interop.
//!
//! `geoxi` deliberately does not depend on any CRS engine. Instead it defines the
//! [`CoordTransform`] trait, which any transformer can implement - including
//! `proxi`'s PROJ-backed transformers.
//!
//! Implementations receive whole batches, so a PROJ binding can use its fastest
//! structure-of-arrays path (`proj_trans_generic`) instead of paying per-point
//! FFI overhead.
//!
//! # Bridging `proxi`
//!
//! ```ignore
//! use geoxi::crs::{CoordTransform, TransformError};
//! use proxi::{AngularUnits, Atomicity, CoordBatch, Direction, Transformer};
//!
//! struct ProxiTransform<'a>(&'a mut Transformer);
//!
//! impl CoordTransform for ProxiTransform<'_> {
//!     fn transform_batch(
//!         &mut self,
//!         x: &mut [f64],
//!         y: &mut [f64],
//!         z: Option<&mut [f64]>,
//!     ) -> Result<(), TransformError> {
//!         let batch = CoordBatch::new(x, y)
//!             .and_then(|b| match z {
//!                 Some(z) => b.with_z(z),
//!                 None => Ok(b),
//!             })
//!             .map_err(|err| TransformError::new(err.to_string()))?;
//!
//!         self.0
//!             .transform_batch(
//!                 batch,
//!                 Direction::Forward,
//!                 AngularUnits::Auto,
//!                 Atomicity::All,
//!             )
//!             .map(|_| ())
//!             .map_err(|err| TransformError::new(err.to_string()))
//!     }
//!
//!     fn preserves_z(&self) -> bool {
//!         true
//!     }
//! }
//!
//! let reprojected = geoxi::crs::reproject(&geometry, &mut ProxiTransform(&mut transform))?;
//! ```

use crate::{CollectionType, Coord, Error, Geometry, GeometryType, Result};

/// An error raised by a [`CoordTransform`] implementation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("coordinate transform failed: {message}")]
pub struct TransformError {
    message: String,
}

impl TransformError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A batched coordinate transformer.
///
/// Implement this for any CRS engine - `proxi`, a custom affine datum shift, an
/// analytic projection - and every `geoxi` reprojection helper works with it.
pub trait CoordTransform {
    /// Transform a batch of coordinates in place.
    ///
    /// `x`, `y`, and (when present) `z` are guaranteed to have equal lengths.
    /// The Z slice is only supplied when the geometry actually carries Z and
    /// [`CoordTransform::preserves_z`] returns `true`.
    fn transform_batch(
        &mut self,
        x: &mut [f64],
        y: &mut [f64],
        z: Option<&mut [f64]>,
    ) -> std::result::Result<(), TransformError>;

    /// Whether the transformer handles Z. When `false`, Z values pass through
    /// untouched and are never handed to [`CoordTransform::transform_batch`].
    fn preserves_z(&self) -> bool {
        false
    }

    /// Optional SRID to stamp onto reprojected output.
    fn target_srid(&self) -> Option<i32> {
        None
    }
}

impl<T: CoordTransform + ?Sized> CoordTransform for &mut T {
    fn transform_batch(
        &mut self,
        x: &mut [f64],
        y: &mut [f64],
        z: Option<&mut [f64]>,
    ) -> std::result::Result<(), TransformError> {
        (**self).transform_batch(x, y, z)
    }

    fn preserves_z(&self) -> bool {
        (**self).preserves_z()
    }

    fn target_srid(&self) -> Option<i32> {
        (**self).target_srid()
    }
}

impl<T: CoordTransform + ?Sized> CoordTransform for Box<T> {
    fn transform_batch(
        &mut self,
        x: &mut [f64],
        y: &mut [f64],
        z: Option<&mut [f64]>,
    ) -> std::result::Result<(), TransformError> {
        (**self).transform_batch(x, y, z)
    }

    fn preserves_z(&self) -> bool {
        (**self).preserves_z()
    }

    fn target_srid(&self) -> Option<i32> {
        (**self).target_srid()
    }
}

/// A [`CoordTransform`] built from a closure over a whole batch.
pub struct BatchTransform<F> {
    transform: F,
    preserves_z: bool,
    srid: Option<i32>,
}

impl<F> BatchTransform<F>
where
    F: FnMut(&mut [f64], &mut [f64], Option<&mut [f64]>) -> std::result::Result<(), TransformError>,
{
    pub fn new(transform: F) -> Self {
        Self {
            transform,
            preserves_z: false,
            srid: None,
        }
    }

    pub fn with_z(mut self) -> Self {
        self.preserves_z = true;
        self
    }

    pub fn with_srid(mut self, srid: i32) -> Self {
        self.srid = Some(srid);
        self
    }
}

impl<F> CoordTransform for BatchTransform<F>
where
    F: FnMut(&mut [f64], &mut [f64], Option<&mut [f64]>) -> std::result::Result<(), TransformError>,
{
    fn transform_batch(
        &mut self,
        x: &mut [f64],
        y: &mut [f64],
        z: Option<&mut [f64]>,
    ) -> std::result::Result<(), TransformError> {
        (self.transform)(x, y, z)
    }

    fn preserves_z(&self) -> bool {
        self.preserves_z
    }

    fn target_srid(&self) -> Option<i32> {
        self.srid
    }
}

/// A structure-of-arrays coordinate batch, laid out for zero-copy handoff to
/// PROJ-style batch APIs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CoordBatch {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Option<Vec<f64>>,
    pub m: Option<Vec<f64>>,
}

impl CoordBatch {
    /// Split coordinates into parallel arrays.
    pub fn from_coords(coords: &[Coord]) -> Self {
        let has_z = coords.iter().any(|coord| coord.z.is_some());
        let has_m = coords.iter().any(|coord| coord.m.is_some());
        Self {
            x: coords.iter().map(|coord| coord.x).collect(),
            y: coords.iter().map(|coord| coord.y).collect(),
            z: has_z.then(|| coords.iter().map(|c| c.z.unwrap_or(0.0)).collect()),
            m: has_m.then(|| coords.iter().map(|c| c.m.unwrap_or(0.0)).collect()),
        }
    }

    /// Recombine the parallel arrays into coordinates.
    pub fn to_coords(&self) -> Vec<Coord> {
        (0..self.len())
            .map(|index| Coord {
                x: self.x[index],
                y: self.y[index],
                z: self.z.as_ref().map(|z| z[index]),
                m: self.m.as_ref().map(|m| m[index]),
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    pub fn has_z(&self) -> bool {
        self.z.is_some()
    }

    /// Run a transformer over this batch in place.
    pub fn transform<T: CoordTransform>(&mut self, transform: &mut T) -> Result<()> {
        let z = if transform.preserves_z() {
            self.z.as_deref_mut()
        } else {
            None
        };
        transform
            .transform_batch(&mut self.x, &mut self.y, z)
            .map_err(|err| Error::Transform(err.to_string()))
    }
}

/// Extract a geometry's coordinates as a structure-of-arrays batch.
pub fn coord_batch(geometry: &Geometry) -> Result<CoordBatch> {
    Ok(CoordBatch::from_coords(&crate::get_coordinates(geometry)?))
}

/// Reproject a geometry with any [`CoordTransform`].
///
/// The geometry structure is preserved exactly; only coordinates change. Z is
/// carried through untouched when the transformer does not handle it.
pub fn reproject<T: CoordTransform>(geometry: &Geometry, transform: &mut T) -> Result<Geometry> {
    let reprojected = rebuild(geometry, transform)?;
    match transform.target_srid() {
        Some(srid) => {
            let mut output = reprojected;
            output.set_srid(srid);
            Ok(output)
        }
        None => Ok(reprojected),
    }
}

/// Reproject many geometries, batching all coordinates into a single transform
/// call. This amortizes FFI overhead across the whole collection.
pub fn reproject_all<T: CoordTransform>(
    geometries: &[Geometry],
    transform: &mut T,
) -> Result<Vec<Geometry>> {
    geometries
        .iter()
        .map(|geometry| reproject(geometry, transform))
        .collect()
}

/// Reproject a bounding box by densifying its edges, matching the behavior of
/// PROJ's `transform_bounds`. Densification matters because projected edges are
/// generally curved.
pub fn reproject_bounds<T: CoordTransform>(
    bounds: crate::Bounds,
    densify_points: usize,
    transform: &mut T,
) -> Result<crate::Bounds> {
    let steps = densify_points.max(1);
    let mut x = Vec::with_capacity(steps * 4 + 4);
    let mut y = Vec::with_capacity(steps * 4 + 4);

    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let lerp_x = bounds.xmin + (bounds.xmax - bounds.xmin) * t;
        let lerp_y = bounds.ymin + (bounds.ymax - bounds.ymin) * t;

        x.push(lerp_x);
        y.push(bounds.ymin);
        x.push(lerp_x);
        y.push(bounds.ymax);
        x.push(bounds.xmin);
        y.push(lerp_y);
        x.push(bounds.xmax);
        y.push(lerp_y);
    }

    transform
        .transform_batch(&mut x, &mut y, None)
        .map_err(|err| Error::Transform(err.to_string()))?;

    let xmin = x.iter().copied().fold(f64::INFINITY, f64::min);
    let xmax = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ymin = y.iter().copied().fold(f64::INFINITY, f64::min);
    let ymax = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    if !xmin.is_finite() || !xmax.is_finite() || !ymin.is_finite() || !ymax.is_finite() {
        return Err(Error::Transform(
            "transformed bounds are not finite".to_string(),
        ));
    }

    Ok(crate::Bounds {
        xmin,
        ymin,
        xmax,
        ymax,
    })
}

fn rebuild<T: CoordTransform>(geometry: &Geometry, transform: &mut T) -> Result<Geometry> {
    let geometry_type = geometry.geometry_type()?;
    match geometry_type {
        GeometryType::Point | GeometryType::LineString | GeometryType::LinearRing => {
            if geometry.is_empty()? {
                return geometry.try_clone();
            }
            let coords = transform_coords(&geometry.coords::<Coord>()?, transform)?;
            match geometry_type {
                GeometryType::Point => Geometry::point_from(coords[0]),
                GeometryType::LineString => Geometry::line_string(&coords),
                _ => Geometry::linear_ring(&coords),
            }
        }
        GeometryType::Polygon => {
            if geometry.is_empty()? {
                return geometry.try_clone();
            }
            let shell = transform_coords(&geometry.exterior_ring()?.coords::<Coord>()?, transform)?;
            let mut holes = Vec::with_capacity(geometry.num_interior_rings()?);
            for index in 0..geometry.num_interior_rings()? {
                holes.push(transform_coords(
                    &geometry.interior_ring_n(index)?.coords::<Coord>()?,
                    transform,
                )?);
            }
            Geometry::polygon(&shell, &holes)
        }
        GeometryType::MultiPoint
        | GeometryType::MultiLineString
        | GeometryType::MultiPolygon
        | GeometryType::GeometryCollection => {
            let parts = geometry
                .parts()?
                .iter()
                .map(|part| rebuild(part, transform))
                .collect::<Result<Vec<_>>>()?;
            let kind = match geometry_type {
                GeometryType::MultiPoint => CollectionType::MultiPoint,
                GeometryType::MultiLineString => CollectionType::MultiLineString,
                GeometryType::MultiPolygon => CollectionType::MultiPolygon,
                _ => CollectionType::GeometryCollection,
            };
            Geometry::collection(kind, parts)
        }
        _ => Err(Error::InvalidInput(
            "reprojection does not support curved geometry types",
        )),
    }
}

fn transform_coords<T: CoordTransform>(coords: &[Coord], transform: &mut T) -> Result<Vec<Coord>> {
    let mut batch = CoordBatch::from_coords(coords);
    batch.transform(transform)?;
    Ok(batch.to_coords())
}

#[cfg(test)]
mod tests {
    use super::*;

    type BatchFn =
        fn(&mut [f64], &mut [f64], Option<&mut [f64]>) -> std::result::Result<(), TransformError>;

    fn shift() -> BatchTransform<BatchFn> {
        let f: BatchFn = |x, y, z| {
            for value in x.iter_mut() {
                *value += 100.0;
            }
            for value in y.iter_mut() {
                *value += 200.0;
            }
            if let Some(z) = z {
                for value in z.iter_mut() {
                    *value *= 2.0;
                }
            }
            Ok(())
        };
        BatchTransform::new(f).with_z()
    }

    #[test]
    fn reprojects_polygons_and_preserves_structure() {
        let polygon = crate::box_(0.0, 0.0, 1.0, 1.0).unwrap();
        let moved = reproject(&polygon, &mut shift()).unwrap();
        let bounds = moved.bounds().unwrap();
        assert_eq!(bounds.xmin, 100.0);
        assert_eq!(bounds.ymin, 200.0);
        assert_eq!(moved.geometry_type().unwrap(), GeometryType::Polygon);
        assert_eq!(moved.area().unwrap(), 1.0);
    }

    #[test]
    fn reprojects_z_when_the_transform_supports_it() {
        let line =
            Geometry::line_string(&[Coord::xyz(0.0, 0.0, 3.0), Coord::xyz(1.0, 1.0, 4.0)]).unwrap();
        let moved = reproject(&line, &mut shift()).unwrap();
        let coords = moved.coords::<Coord>().unwrap();
        assert_eq!(coords[0].z, Some(6.0));
        assert_eq!(coords[1].z, Some(8.0));
    }

    #[test]
    fn leaves_z_untouched_when_the_transform_is_2d() {
        let line =
            Geometry::line_string(&[Coord::xyz(0.0, 0.0, 3.0), Coord::xyz(1.0, 1.0, 4.0)]).unwrap();
        let mut planar =
            BatchTransform::new(|x: &mut [f64], _: &mut [f64], _: Option<&mut [f64]>| {
                for value in x.iter_mut() {
                    *value += 1.0;
                }
                Ok(())
            });
        let moved = reproject(&line, &mut planar).unwrap();
        let coords = moved.coords::<Coord>().unwrap();
        assert_eq!(coords[0].z, Some(3.0));
        assert_eq!(coords[0].x, 1.0);
    }

    #[test]
    fn propagates_transform_errors() {
        let point = crate::point(0.0, 0.0).unwrap();
        let mut failing =
            BatchTransform::new(|_: &mut [f64], _: &mut [f64], _: Option<&mut [f64]>| {
                Err(TransformError::new("no operation found"))
            });
        let err = reproject(&point, &mut failing).unwrap_err();
        assert!(err.to_string().contains("no operation found"));
    }

    #[test]
    fn densified_bounds_cover_the_transformed_edges() {
        let bounds = crate::Bounds {
            xmin: 0.0,
            ymin: 0.0,
            xmax: 10.0,
            ymax: 10.0,
        };
        let moved = reproject_bounds(bounds, 21, &mut shift()).unwrap();
        assert_eq!(moved.xmin, 100.0);
        assert_eq!(moved.xmax, 110.0);
        assert_eq!(moved.ymin, 200.0);
        assert_eq!(moved.ymax, 210.0);
    }
}
