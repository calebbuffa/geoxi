//! Coordinate access, coordinate rewriting, and GeoArrow-style ragged arrays.
//!
//! Coordinate traversal is depth-first and order-stable, so
//! [`get_coordinates`] and [`set_coordinates`] round-trip exactly.

use crate::{CollectionType, Coord, Error, Geometry, GeometryType, Result};

/// Collect every coordinate of a geometry in depth-first order.
pub fn get_coordinates(geometry: &Geometry) -> Result<Vec<Coord>> {
    let mut out = Vec::with_capacity(geometry.num_coordinates()?);
    collect_coords(geometry, &mut out)?;
    Ok(out)
}

/// Collect the coordinates of many geometries into one flat buffer.
pub fn get_coordinates_all(geometries: &[Geometry]) -> Result<Vec<Coord>> {
    let mut out = Vec::new();
    for geometry in geometries {
        collect_coords(geometry, &mut out)?;
    }
    Ok(out)
}

/// Count a geometry's coordinates, including repeated ring-closing points.
pub fn count_coordinates(geometry: &Geometry) -> Result<usize> {
    geometry.num_coordinates()
}

/// Rebuild a geometry with replacement coordinates, preserving its structure.
///
/// The replacement slice must contain exactly [`count_coordinates`] entries, in
/// the same depth-first order produced by [`get_coordinates`].
pub fn set_coordinates(geometry: &Geometry, coords: &[Coord]) -> Result<Geometry> {
    let expected = geometry.num_coordinates()?;
    if coords.len() != expected {
        return Err(Error::InvalidInput(
            "replacement coordinate count must match the geometry's coordinate count",
        ));
    }
    let mut cursor = 0usize;
    rebuild(geometry, coords, &mut cursor)
}

/// Apply a per-coordinate function, preserving geometry structure.
///
/// Unlike [`Geometry::transform_xy`], the callback here is a plain closure with
/// no `'static`/`Send` requirement, because no pointer crosses the FFI boundary.
pub fn transform_coords<F>(geometry: &Geometry, mut f: F) -> Result<Geometry>
where
    F: FnMut(Coord) -> Result<Coord>,
{
    let coords = get_coordinates(geometry)?
        .into_iter()
        .map(&mut f)
        .collect::<Result<Vec<_>>>()?;
    set_coordinates(geometry, &coords)
}

/// Split every line-like component into its individual two-point segments.
pub fn get_segments(geometry: &Geometry) -> Result<Vec<Geometry>> {
    let mut out = Vec::new();
    collect_segments(geometry, &mut out)?;
    Ok(out)
}

fn collect_segments(geometry: &Geometry, out: &mut Vec<Geometry>) -> Result<()> {
    match geometry.geometry_type()? {
        GeometryType::LineString | GeometryType::LinearRing => {
            let coords = geometry.coords::<Coord>()?;
            for pair in coords.windows(2) {
                out.push(Geometry::line_string(pair)?);
            }
        }
        GeometryType::Polygon if !geometry.is_empty()? => {
            collect_segments(&geometry.exterior_ring()?, out)?;
            for index in 0..geometry.num_interior_rings()? {
                collect_segments(&geometry.interior_ring_n(index)?, out)?;
            }
        }
        GeometryType::MultiPoint
        | GeometryType::MultiLineString
        | GeometryType::MultiPolygon
        | GeometryType::GeometryCollection => {
            for part in geometry.parts()? {
                collect_segments(&part, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_coords(geometry: &Geometry, out: &mut Vec<Coord>) -> Result<()> {
    match geometry.geometry_type()? {
        GeometryType::Point | GeometryType::LineString | GeometryType::LinearRing => {
            if !geometry.is_empty()? {
                out.extend(geometry.coords::<Coord>()?);
            }
        }
        GeometryType::Polygon => {
            if !geometry.is_empty()? {
                out.extend(geometry.exterior_ring()?.coords::<Coord>()?);
                for index in 0..geometry.num_interior_rings()? {
                    out.extend(geometry.interior_ring_n(index)?.coords::<Coord>()?);
                }
            }
        }
        GeometryType::MultiPoint
        | GeometryType::MultiLineString
        | GeometryType::MultiPolygon
        | GeometryType::GeometryCollection => {
            for part in geometry.parts()? {
                collect_coords(&part, out)?;
            }
        }
        _ => {
            return Err(Error::InvalidInput(
                "coordinate access does not support curved geometry types",
            ));
        }
    }
    Ok(())
}

fn take<'a>(coords: &'a [Coord], cursor: &mut usize, count: usize) -> &'a [Coord] {
    let slice = &coords[*cursor..*cursor + count];
    *cursor += count;
    slice
}

fn rebuild(geometry: &Geometry, coords: &[Coord], cursor: &mut usize) -> Result<Geometry> {
    let geometry_type = geometry.geometry_type()?;
    match geometry_type {
        GeometryType::Point | GeometryType::LineString | GeometryType::LinearRing => {
            if geometry.is_empty()? {
                return geometry.try_clone();
            }
            let count = geometry.coords::<Coord>()?.len();
            let slice = take(coords, cursor, count);
            match geometry_type {
                GeometryType::Point => Geometry::point_from(slice[0]),
                GeometryType::LineString => Geometry::line_string(slice),
                _ => Geometry::linear_ring(slice),
            }
        }
        GeometryType::Polygon => {
            if geometry.is_empty()? {
                return geometry.try_clone();
            }
            let shell_len = geometry.exterior_ring()?.coords::<Coord>()?.len();
            let shell = take(coords, cursor, shell_len).to_vec();
            let mut holes = Vec::with_capacity(geometry.num_interior_rings()?);
            for index in 0..geometry.num_interior_rings()? {
                let hole_len = geometry.interior_ring_n(index)?.coords::<Coord>()?.len();
                holes.push(take(coords, cursor, hole_len).to_vec());
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
                .map(|part| rebuild(part, coords, cursor))
                .collect::<Result<Vec<_>>>()?;
            Geometry::collection(collection_type(geometry_type)?, parts)
        }
        _ => Err(Error::InvalidInput(
            "coordinate rewriting does not support curved geometry types",
        )),
    }
}

fn collection_type(geometry_type: GeometryType) -> Result<CollectionType> {
    Ok(match geometry_type {
        GeometryType::MultiPoint => CollectionType::MultiPoint,
        GeometryType::MultiLineString => CollectionType::MultiLineString,
        GeometryType::MultiPolygon => CollectionType::MultiPolygon,
        GeometryType::GeometryCollection => CollectionType::GeometryCollection,
        _ => return Err(Error::InvalidInput("not a collection geometry type")),
    })
}

/// The geometry class encoded by a [`RaggedArray`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaggedType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
}

/// A GeoArrow-style flat coordinate buffer plus nesting offsets.
///
/// This is the cheapest way to hand geometry to columnar systems: coordinates
/// stay in one contiguous allocation.
#[derive(Debug, Clone, PartialEq)]
pub struct RaggedArray {
    /// The geometry class every element shares.
    pub geometry_type: RaggedType,
    /// Flat, depth-first coordinate buffer.
    pub coords: Vec<Coord>,
    /// Nesting offsets, outermost last. `Point` has none; `MultiPolygon` has
    /// three (ring, polygon, geometry).
    pub offsets: Vec<Vec<usize>>,
}

impl RaggedArray {
    pub fn len(&self) -> usize {
        match self.offsets.last() {
            Some(outer) => outer.len().saturating_sub(1),
            None => self.coords.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Encode homogeneous geometries into a [`RaggedArray`].
pub fn to_ragged_array(geometries: &[Geometry]) -> Result<RaggedArray> {
    let Some(first) = geometries.first() else {
        return Err(Error::InvalidInput(
            "ragged array encoding requires at least one geometry",
        ));
    };
    let geometry_type = ragged_type(first.geometry_type()?)?;
    for geometry in geometries {
        if ragged_type(geometry.geometry_type()?)? != geometry_type {
            return Err(Error::InvalidInput(
                "ragged array encoding requires homogeneous geometry types",
            ));
        }
    }

    let mut coords = Vec::new();
    match geometry_type {
        RaggedType::Point => {
            for geometry in geometries {
                coords.extend(geometry.coords::<Coord>()?);
            }
            Ok(RaggedArray {
                geometry_type,
                coords,
                offsets: Vec::new(),
            })
        }
        RaggedType::LineString | RaggedType::MultiPoint => {
            let mut geom_offsets = vec![0usize];
            for geometry in geometries {
                coords.extend(get_coordinates(geometry)?);
                geom_offsets.push(coords.len());
            }
            Ok(RaggedArray {
                geometry_type,
                coords,
                offsets: vec![geom_offsets],
            })
        }
        RaggedType::Polygon | RaggedType::MultiLineString => {
            let mut ring_offsets = vec![0usize];
            let mut geom_offsets = vec![0usize];
            for geometry in geometries {
                for ring in inner_rings(geometry)? {
                    coords.extend(ring);
                    ring_offsets.push(coords.len());
                }
                geom_offsets.push(ring_offsets.len() - 1);
            }
            Ok(RaggedArray {
                geometry_type,
                coords,
                offsets: vec![ring_offsets, geom_offsets],
            })
        }
        RaggedType::MultiPolygon => {
            let mut ring_offsets = vec![0usize];
            let mut polygon_offsets = vec![0usize];
            let mut geom_offsets = vec![0usize];
            for geometry in geometries {
                for polygon in geometry.parts()? {
                    for ring in inner_rings(&polygon)? {
                        coords.extend(ring);
                        ring_offsets.push(coords.len());
                    }
                    polygon_offsets.push(ring_offsets.len() - 1);
                }
                geom_offsets.push(polygon_offsets.len() - 1);
            }
            Ok(RaggedArray {
                geometry_type,
                coords,
                offsets: vec![ring_offsets, polygon_offsets, geom_offsets],
            })
        }
    }
}

/// Decode a [`RaggedArray`] back into geometries.
pub fn from_ragged_array(array: &RaggedArray) -> Result<Vec<Geometry>> {
    match array.geometry_type {
        RaggedType::Point => array
            .coords
            .iter()
            .map(|coord| Geometry::point_from(*coord))
            .collect(),
        RaggedType::LineString | RaggedType::MultiPoint => {
            let offsets = offsets_at(array, 0)?;
            offsets
                .windows(2)
                .map(|window| {
                    let slice = &array.coords[window[0]..window[1]];
                    if array.geometry_type == RaggedType::LineString {
                        Geometry::line_string(slice)
                    } else {
                        let points = slice
                            .iter()
                            .map(|coord| Geometry::point_from(*coord))
                            .collect::<Result<Vec<_>>>()?;
                        Geometry::collection(CollectionType::MultiPoint, points)
                    }
                })
                .collect()
        }
        RaggedType::Polygon | RaggedType::MultiLineString => {
            let ring_offsets = offsets_at(array, 0)?;
            let geom_offsets = offsets_at(array, 1)?;
            geom_offsets
                .windows(2)
                .map(|window| {
                    let rings = rings_between(array, ring_offsets, window[0], window[1]);
                    if array.geometry_type == RaggedType::Polygon {
                        build_polygon(&rings)
                    } else {
                        let lines = rings
                            .iter()
                            .map(|ring| Geometry::line_string(ring))
                            .collect::<Result<Vec<_>>>()?;
                        Geometry::collection(CollectionType::MultiLineString, lines)
                    }
                })
                .collect()
        }
        RaggedType::MultiPolygon => {
            let ring_offsets = offsets_at(array, 0)?;
            let polygon_offsets = offsets_at(array, 1)?;
            let geom_offsets = offsets_at(array, 2)?;
            geom_offsets
                .windows(2)
                .map(|window| {
                    let polygons = (window[0]..window[1])
                        .map(|index| {
                            let rings = rings_between(
                                array,
                                ring_offsets,
                                polygon_offsets[index],
                                polygon_offsets[index + 1],
                            );
                            build_polygon(&rings)
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Geometry::collection(CollectionType::MultiPolygon, polygons)
                })
                .collect()
        }
    }
}

fn offsets_at(array: &RaggedArray, level: usize) -> Result<&[usize]> {
    array
        .offsets
        .get(level)
        .map(Vec::as_slice)
        .ok_or(Error::InvalidInput("ragged array is missing offset levels"))
}

fn rings_between(
    array: &RaggedArray,
    ring_offsets: &[usize],
    start: usize,
    end: usize,
) -> Vec<Vec<Coord>> {
    (start..end)
        .map(|index| array.coords[ring_offsets[index]..ring_offsets[index + 1]].to_vec())
        .collect()
}

fn build_polygon(rings: &[Vec<Coord>]) -> Result<Geometry> {
    let Some((shell, holes)) = rings.split_first() else {
        return Err(Error::InvalidInput("a polygon requires an exterior ring"));
    };
    Geometry::polygon(shell, holes)
}

fn inner_rings(geometry: &Geometry) -> Result<Vec<Vec<Coord>>> {
    match geometry.geometry_type()? {
        GeometryType::Polygon => {
            let mut rings = vec![geometry.exterior_ring()?.coords::<Coord>()?];
            for index in 0..geometry.num_interior_rings()? {
                rings.push(geometry.interior_ring_n(index)?.coords::<Coord>()?);
            }
            Ok(rings)
        }
        GeometryType::MultiLineString => geometry
            .parts()?
            .iter()
            .map(|part| part.coords::<Coord>())
            .collect(),
        _ => Err(Error::InvalidInput(
            "expected a Polygon or MultiLineString while encoding a ragged array",
        )),
    }
}

fn ragged_type(geometry_type: GeometryType) -> Result<RaggedType> {
    Ok(match geometry_type {
        GeometryType::Point => RaggedType::Point,
        GeometryType::LineString | GeometryType::LinearRing => RaggedType::LineString,
        GeometryType::Polygon => RaggedType::Polygon,
        GeometryType::MultiPoint => RaggedType::MultiPoint,
        GeometryType::MultiLineString => RaggedType::MultiLineString,
        GeometryType::MultiPolygon => RaggedType::MultiPolygon,
        _ => {
            return Err(Error::InvalidInput(
                "ragged arrays support only point, line, and polygon geometry types",
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_round_trip_through_set_coordinates() {
        let polygon = crate::box_(0.0, 0.0, 2.0, 2.0).unwrap();
        let coords = get_coordinates(&polygon).unwrap();
        assert_eq!(coords.len(), count_coordinates(&polygon).unwrap());

        let moved = set_coordinates(
            &polygon,
            &coords
                .iter()
                .map(|c| Coord::xy(c.x + 10.0, c.y))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(moved.bounds().unwrap().xmin, 10.0);
        assert_eq!(moved.area().unwrap(), 4.0);
    }

    #[test]
    fn transform_coords_preserves_z() {
        let line =
            Geometry::line_string(&[Coord::xyz(0.0, 0.0, 1.0), Coord::xyz(1.0, 0.0, 2.0)]).unwrap();
        let scaled = transform_coords(&line, |coord| {
            Ok(Coord::xyz(coord.x, coord.y, coord.z.unwrap_or(0.0) * 10.0))
        })
        .unwrap();
        let coords = scaled.coords::<Coord>().unwrap();
        assert_eq!(coords[1].z, Some(20.0));
    }

    #[test]
    fn segments_split_lines_into_pairs() {
        let line = Geometry::from_wkt("LINESTRING (0 0, 1 0, 2 0)").unwrap();
        let segments = get_segments(&line).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].length().unwrap(), 1.0);
    }

    #[test]
    fn ragged_arrays_round_trip_polygons() {
        let geometries = vec![
            Geometry::from_wkt("POLYGON ((0 0, 3 0, 3 3, 0 3, 0 0), (1 1, 2 1, 2 2, 1 2, 1 1))")
                .unwrap(),
            crate::box_(10.0, 10.0, 11.0, 11.0).unwrap(),
        ];
        let array = to_ragged_array(&geometries).unwrap();
        assert_eq!(array.geometry_type, RaggedType::Polygon);
        assert_eq!(array.len(), 2);

        let decoded = from_ragged_array(&array).unwrap();
        assert_eq!(decoded.len(), 2);
        assert!((decoded[0].area().unwrap() - 8.0).abs() < 1e-9);
        assert_eq!(decoded[0].num_interior_rings().unwrap(), 1);
        assert_eq!(decoded[1].area().unwrap(), 1.0);
    }

    #[test]
    fn ragged_arrays_round_trip_multipolygons_and_points() {
        let multi = vec![
            Geometry::from_wkt(
                "MULTIPOLYGON (((0 0, 1 0, 1 1, 0 1, 0 0)), ((5 5, 6 5, 6 6, 5 5)))",
            )
            .unwrap(),
        ];
        let decoded = from_ragged_array(&to_ragged_array(&multi).unwrap()).unwrap();
        assert_eq!(decoded[0].num_geometries().unwrap(), 2);

        let points = vec![
            crate::point(1.0, 2.0).unwrap(),
            crate::point(3.0, 4.0).unwrap(),
        ];
        let array = to_ragged_array(&points).unwrap();
        assert_eq!(array.len(), 2);
        let decoded = from_ragged_array(&array).unwrap();
        assert_eq!(decoded[1].x().unwrap(), 3.0);
    }

    #[test]
    fn mismatched_coordinate_counts_are_rejected() {
        let polygon = crate::box_(0.0, 0.0, 1.0, 1.0).unwrap();
        let err = set_coordinates(&polygon, &[Coord::xy(0.0, 0.0)]).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }
}
