//! Splitting, orientation, validation, and equality helpers.

use crate::internal_prelude::*;

pub fn split(geometry: &Geometry, splitter: &Geometry) -> Result<Geometry> {
    match geometry.geometry_type()? {
        GeometryType::LineString | GeometryType::LinearRing => {
            let pieces = split_line(geometry, splitter)?;
            Geometry::collection(CollectionType::GeometryCollection, pieces)
        }
        GeometryType::MultiLineString => {
            let mut pieces = Vec::new();
            for part in geometry.parts()? {
                pieces.extend(split_line(&part, splitter)?);
            }
            Geometry::collection(CollectionType::GeometryCollection, pieces)
        }
        GeometryType::Polygon | GeometryType::MultiPolygon => {
            let boundary = geometry.boundary()?;
            let noded = boundary.union(splitter)?.node()?;
            let polygons = polygonize(&[noded])?;
            let kept = polygons
                .parts()?
                .into_iter()
                .map(|part| {
                    let inside = part.point_on_surface()?.within(geometry)?;
                    Ok(inside.then_some(part))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect();
            Geometry::collection(CollectionType::GeometryCollection, kept)
        }
        _ => Err(Error::InvalidInput(
            "split supports only line and polygon geometries",
        )),
    }
}

/// Split a single line. Point splitters cut the line at the projected
/// positions; anything else is removed with `difference`.
fn split_line(line: &Geometry, splitter: &Geometry) -> Result<Vec<Geometry>> {
    let point_like = matches!(
        splitter.geometry_type()?,
        GeometryType::Point | GeometryType::MultiPoint
    );
    if !point_like {
        return as_collection(
            &line.difference(splitter)?,
            CollectionType::GeometryCollection,
        )?
        .parts();
    }

    let splitters = match splitter.geometry_type()? {
        GeometryType::MultiPoint => splitter.parts()?,
        _ => vec![splitter.try_clone()?],
    };

    let mut cuts = Vec::with_capacity(splitters.len());
    for point in &splitters {
        if !point.intersects(line)? {
            continue;
        }
        let fraction = line.line_locate_point(point, true)?;
        if fraction > f64::EPSILON && fraction < 1.0 - f64::EPSILON {
            cuts.push(fraction);
        }
    }
    if cuts.is_empty() {
        return Ok(vec![line.try_clone()?]);
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    cuts.dedup_by(|a, b| (*a - *b).abs() <= f64::EPSILON);

    let mut pieces = Vec::with_capacity(cuts.len() + 1);
    let mut start = 0.0;
    for cut in cuts {
        pieces.push(line.line_substring(start, cut)?);
        start = cut;
    }
    pieces.push(line.line_substring(start, 1.0)?);
    Ok(pieces)
}

fn as_collection(geometry: &Geometry, kind: CollectionType) -> Result<Geometry> {
    match geometry.geometry_type()? {
        GeometryType::MultiPoint
        | GeometryType::MultiLineString
        | GeometryType::MultiPolygon
        | GeometryType::GeometryCollection => Geometry::collection(kind, geometry.parts()?),
        _ => Geometry::collection(kind, vec![geometry.try_clone()?]),
    }
}

/// Enforce ring orientation. `exterior_cw = true` gives clockwise exteriors and
/// counter-clockwise interiors; `false` gives the reverse.
pub fn orient_polygons(geometry: &Geometry, exterior_cw: bool) -> Result<Geometry> {
    let mut oriented = geometry.try_clone()?;
    oriented.orient_polygons(exterior_cw)?;
    Ok(oriented)
}

/// Enforce ring orientation by sign: positive keeps exteriors
/// counter-clockwise, negative makes them clockwise.
pub fn orient(geometry: &Geometry, sign: f64) -> Result<Geometry> {
    orient_polygons(geometry, sign < 0.0)
}

/// Whether a linear ring is counter-clockwise.
pub fn is_ccw(geometry: &Geometry) -> Result<bool> {
    let coords = geometry.coords::<Coord>()?;
    if coords.len() < 4 {
        return Err(Error::InvalidInput(
            "is_ccw requires a closed ring of at least four coordinates",
        ));
    }
    let twice_area: f64 = coords
        .windows(2)
        .map(|pair| (pair[1].x - pair[0].x) * (pair[1].y + pair[0].y))
        .sum();
    Ok(twice_area < 0.0)
}

/// Human-readable explanation of why a geometry is invalid, or `"Valid
/// Geometry"` if it is sound.
pub fn explain_validity(geometry: &Geometry) -> Result<String> {
    geometry.is_valid_reason()
}

/// Alias for [`explain_validity`].
pub fn validate(geometry: &Geometry) -> Result<String> {
    geometry.is_valid_reason()
}

/// Whether a geometry can be used as input at all.
pub fn is_valid_input(geometry: &Geometry) -> Result<bool> {
    geometry.geometry_type().map(|_| true)
}

/// Compare two geometries for structural equality within a tolerance.
pub fn geometries_equal(a: &Geometry, b: &Geometry, tolerance: f64) -> Result<bool> {
    if a.geometry_type()? != b.geometry_type()? {
        return Ok(false);
    }
    if tolerance == 0.0 {
        return a.equals_identical(b);
    }
    a.equals_exact(b, tolerance)
}

/// Assert structural equality, returning a descriptive error on mismatch.
pub fn assert_geometries_equal(a: &Geometry, b: &Geometry, tolerance: f64) -> Result<()> {
    if geometries_equal(a, b, tolerance)? {
        Ok(())
    } else {
        Err(Error::Transform(format!(
            "geometries differ: {} != {}",
            a.to_wkt()?,
            b.to_wkt()?
        )))
    }
}

/// Total bounds across many geometries.
pub fn total_bounds(geometries: &[Geometry]) -> Result<Option<Bounds>> {
    GeometryArray::new(geometries.to_vec()).total_bounds()
}
